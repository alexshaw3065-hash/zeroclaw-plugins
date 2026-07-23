use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug)]
pub enum RpcError {
    Http(String),
    Rpc { code: i64, message: String },
    Parse(String),
}

impl std::fmt::Display for RpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RpcError::Http(s) => write!(f, "http error: {s}"),
            RpcError::Rpc { code, message } => write!(f, "rpc error {code}: {message}"),
            RpcError::Parse(s) => write!(f, "failed to parse rpc response: {s}"),
        }
    }
}

#[derive(Serialize)]
pub struct RpcRequest<'a> {
    pub jsonrpc: &'a str,
    pub id: u64,
    pub method: &'a str,
    pub params: Value,
}

impl<'a> RpcRequest<'a> {
    pub fn new(method: &'a str, params: Value) -> Self {
        RpcRequest {
            jsonrpc: "2.0",
            id: 1,
            method,
            params,
        }
    }

    /// Turn this request into the JSON body a transport layer (waki, in
    /// the wasm shim) can send as-is. Kept here so request-building is
    /// host-testable without a live network call.
    pub fn to_body(&self) -> Result<String, RpcError> {
        serde_json::to_string(self).map_err(|e| RpcError::Parse(e.to_string()))
    }
}

#[derive(Deserialize)]
struct RpcEnvelope {
    result: Option<Value>,
    error: Option<RpcErrorBody>,
}

#[derive(Deserialize)]
struct RpcErrorBody {
    code: i64,
    message: String,
}

/// Parse a raw JSON-RPC response body into either the `result` value or a
/// structured error. This is the piece exercised by host tests with a
/// mocked response string — no live network involved, per the bounty's
/// "mock the RPC, no live network in tests" requirement.
pub fn parse_response(body: &str) -> Result<Value, RpcError> {
    let envelope: RpcEnvelope =
        serde_json::from_str(body).map_err(|e| RpcError::Parse(e.to_string()))?;
    parse_envelope(envelope)
}

/// Same as [`parse_response`], but for a transport that already handed back
/// a parsed `Value` (e.g. `waki`'s `.json::<Value>()`) instead of raw text —
/// avoids a pointless serialize/deserialize round trip in the wasm shim.
pub fn parse_response_value(body: Value) -> Result<Value, RpcError> {
    let envelope: RpcEnvelope =
        serde_json::from_value(body).map_err(|e| RpcError::Parse(e.to_string()))?;
    parse_envelope(envelope)
}

fn parse_envelope(envelope: RpcEnvelope) -> Result<Value, RpcError> {
    if let Some(err) = envelope.error {
        return Err(RpcError::Rpc {
            code: err.code,
            message: err.message,
        });
    }
    envelope
        .result
        .ok_or_else(|| RpcError::Parse("missing result".into()))
}

/// Pull the base64-encoded account bytes out of a decoded `getAccountInfo`
/// `result` value: `{"value": null | {"data": [base64, "base64"], ...}}`. A
/// `null` value means the account does not exist on chain.
pub fn account_data_from_result(result: &Value) -> Result<Vec<u8>, RpcError> {
    let account = result
        .get("value")
        .ok_or_else(|| RpcError::Parse("missing 'value' field".into()))?;
    if account.is_null() {
        return Err(RpcError::Parse("account not found".into()));
    }
    let encoded = account
        .get("data")
        .and_then(|d| d.as_array())
        .and_then(|arr| arr.first())
        .and_then(Value::as_str)
        .ok_or_else(|| RpcError::Parse("missing base64 account data".into()))?;
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|e| RpcError::Parse(format!("invalid base64 account data: {e}")))
}

/// The largest single balance among a decoded `getTokenLargestAccounts`
/// `result` value's accounts, in raw (undecimalled) units. Deliberately
/// takes the max rather than trusting response order, since the RPC spec
/// does not document one.
pub fn max_token_account_amount(result: &Value) -> Result<u64, RpcError> {
    let accounts = result
        .get("value")
        .and_then(Value::as_array)
        .ok_or_else(|| RpcError::Parse("missing 'value' array".into()))?;
    let max = accounts
        .iter()
        .filter_map(|a| a.get("amount").and_then(Value::as_str))
        .filter_map(|s| s.parse::<u64>().ok())
        .max()
        .unwrap_or(0);
    Ok(max)
}

/// A minimal transport-agnostic client. The actual HTTP call is made by
/// the wasm shim (via waki, gated behind the `http_client` permission);
/// this struct only builds the request and parses the response, so it
/// stays fully host-testable.
pub struct RpcClient {
    pub endpoint: String,
}

impl RpcClient {
    pub fn new(endpoint: impl Into<String>) -> Self {
        RpcClient {
            endpoint: endpoint.into(),
        }
    }

    pub fn build_request<'a>(&self, method: &'a str, params: Value) -> RpcRequest<'a> {
        RpcRequest::new(method, params)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn builds_a_valid_request_body() {
        let req = RpcRequest::new("getBalance", json!(["someaddress"]));
        let body = req.to_body().unwrap();
        assert!(body.contains("getBalance"));
        assert!(body.contains("jsonrpc"));
    }

    #[test]
    fn parses_a_successful_response() {
        let body = r#"{"jsonrpc":"2.0","id":1,"result":{"value":42}}"#;
        let result = parse_response(body).unwrap();
        assert_eq!(result["value"], 42);
    }

    #[test]
    fn parses_an_rpc_error_response() {
        let body = r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32602,"message":"invalid params"}}"#;
        let err = parse_response(body).unwrap_err();
        match err {
            RpcError::Rpc { code, message } => {
                assert_eq!(code, -32602);
                assert_eq!(message, "invalid params");
            }
            _ => panic!("expected RpcError::Rpc"),
        }
    }

    #[test]
    fn fails_closed_on_malformed_json() {
        let result = parse_response("not json at all");
        assert!(result.is_err());
    }

    #[test]
    fn parse_response_value_matches_parse_response() {
        let body = r#"{"jsonrpc":"2.0","id":1,"result":{"value":42}}"#;
        let from_str = parse_response(body).unwrap();
        let from_value = parse_response_value(serde_json::from_str(body).unwrap()).unwrap();
        assert_eq!(from_str, from_value);
    }

    #[test]
    fn account_data_from_result_decodes_base64() {
        // "hi" base64-encoded, in the shape getAccountInfo actually returns.
        let result = json!({"value": {"data": ["aGk=", "base64"], "owner": "x"}});
        let data = account_data_from_result(&result).unwrap();
        assert_eq!(data, b"hi");
    }

    #[test]
    fn account_data_from_result_fails_closed_on_missing_account() {
        let result = json!({"value": null});
        assert!(account_data_from_result(&result).is_err());
    }

    #[test]
    fn account_data_from_result_fails_closed_on_malformed_shape() {
        let result = json!({"value": {"nope": true}});
        assert!(account_data_from_result(&result).is_err());
    }

    #[test]
    fn max_token_account_amount_picks_the_largest_regardless_of_order() {
        let result = json!({"value": [
            {"amount": "10"},
            {"amount": "9000"},
            {"amount": "500"}
        ]});
        assert_eq!(max_token_account_amount(&result).unwrap(), 9000);
    }

    #[test]
    fn max_token_account_amount_empty_list_is_zero() {
        let result = json!({"value": []});
        assert_eq!(max_token_account_amount(&result).unwrap(), 0);
    }
}
