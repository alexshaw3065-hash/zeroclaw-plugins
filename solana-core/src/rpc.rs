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
}
