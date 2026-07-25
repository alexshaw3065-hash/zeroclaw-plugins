use serde_json::Value;

/// Liquidity-pool presence/depth for a mint, as reported by a public DEX
/// aggregator (Dexscreener). Deliberately a thin, honest signal: this
/// confirms a pool *exists* and roughly how deep it is, not whether its
/// LP tokens are locked or burned -- that would need a different,
/// specialized data source this project doesn't integrate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LpStatus {
    /// Whether at least one on-chain pool was found for this mint.
    pub pool_found: bool,
    /// Total liquidity in USD, summed across every pool found. `None`
    /// when `pool_found` is `false` -- nothing to sum.
    pub liquidity_usd: Option<f64>,
}

/// Parse a Dexscreener `/latest/dex/tokens/<mint>` response body into an
/// [`LpStatus`]. Confirmed live (2026-07-23) against the real API: a
/// mint with pools returns `{"pairs": [{"liquidity": {"usd": ...}, ...},
/// ...]}`; a mint Dexscreener has never indexed (including every devnet
/// or freshly-minted address) returns `{"pairs": null}` -- both are
/// handled identically here as "no pool found," never as an error, since
/// this is a real HTTP 200 either way.
pub fn lp_status_from_dexscreener(body: &Value) -> LpStatus {
    let pairs: &[Value] = body
        .get("pairs")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);

    if pairs.is_empty() {
        return LpStatus {
            pool_found: false,
            liquidity_usd: None,
        };
    }

    // Sum every pool's liquidity rather than trusting the first entry --
    // a mint can (and often does) have pools on more than one DEX. A pool
    // entry missing `liquidity.usd` contributes 0, not a parse failure:
    // this is a best-effort enrichment, not a strict on-chain fact.
    let total: f64 = pairs
        .iter()
        .filter_map(|p| p.get("liquidity")?.get("usd")?.as_f64())
        .sum();

    LpStatus {
        pool_found: true,
        liquidity_usd: Some(total),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn null_pairs_means_no_pool_found() {
        let body = json!({"schemaVersion": "1.0.0", "pairs": null});
        let status = lp_status_from_dexscreener(&body);
        assert!(!status.pool_found);
        assert!(status.liquidity_usd.is_none());
    }

    #[test]
    fn empty_pairs_array_means_no_pool_found() {
        let body = json!({"schemaVersion": "1.0.0", "pairs": []});
        let status = lp_status_from_dexscreener(&body);
        assert!(!status.pool_found);
        assert!(status.liquidity_usd.is_none());
    }

    #[test]
    fn missing_pairs_field_means_no_pool_found() {
        let body = json!({"schemaVersion": "1.0.0"});
        let status = lp_status_from_dexscreener(&body);
        assert!(!status.pool_found);
    }

    #[test]
    fn sums_liquidity_across_multiple_pairs() {
        let body = json!({"pairs": [
            {"liquidity": {"usd": 1_117_249.51}},
            {"liquidity": {"usd": 250_000.0}}
        ]});
        let status = lp_status_from_dexscreener(&body);
        assert!(status.pool_found);
        assert_eq!(status.liquidity_usd, Some(1_367_249.51));
    }

    #[test]
    fn a_pair_missing_liquidity_counts_as_zero_not_a_parse_error() {
        let body = json!({"pairs": [
            {"dexId": "orca"},
            {"liquidity": {"usd": 500.0}}
        ]});
        let status = lp_status_from_dexscreener(&body);
        assert!(status.pool_found);
        assert_eq!(status.liquidity_usd, Some(500.0));
    }
}
