use serde::{Deserialize, Serialize};

/// A red/amber/green verdict, matching what the bounty doc asks
/// `token-risk-check` to return.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskLevel {
    Green,
    Amber,
    Red,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskReport {
    pub level: RiskLevel,
    pub reasons: Vec<String>,
}

/// Raw, on-chain-shaped facts about a mint. Plain data on purpose, so
/// tests can construct it by hand without touching the network — the
/// wasm shim is responsible for fetching the real values via RPC and
/// building one of these before calling `assess`.
#[derive(Debug, Clone, Default)]
pub struct MintFacts {
    pub mint_authority_active: bool,
    pub freeze_authority_active: bool,
    /// The largest single holder's share of supply, as a percentage.
    /// `None` means this wasn't determined -- `getTokenLargestAccounts`
    /// is rejected outright by some RPC providers for very widely-held
    /// mints (e.g. real mainnet USDC has too many holders for Helius to
    /// serve this call at all), so a failed lookup must degrade to
    /// "unknown", not fail the whole check. Same fail-open contract as
    /// `lp_pool_found` below: `assess` only ever escalates risk on a
    /// definite `Some(pct)` over the threshold, never on `None`.
    pub top_holder_share_pct: Option<f64>,
    pub has_permanent_delegate: bool,
    pub has_transfer_hook: bool,
    pub transfer_fee_bps: u16,
    /// Whether a DEX aggregator (Dexscreener) found at least one on-chain
    /// liquidity pool for this mint. `None` means this enrichment wasn't
    /// attempted or the lookup failed/was unavailable (an unconfigured
    /// operator, a rate-limited API, a devnet-only test mint no aggregator
    /// has ever indexed) -- deliberately distinct from `Some(false)` (a
    /// real "no pool exists" answer). `assess` only ever escalates risk on
    /// `Some(false)`, never on `None`: missing enrichment data must never
    /// be treated as a red flag, or every unconfigured/offline deployment
    /// would wrongly downgrade every mint it checks.
    pub lp_pool_found: Option<bool>,
    /// Total liquidity in USD, summed across every pool found. Only
    /// meaningful when `lp_pool_found` is `Some(true)`.
    pub lp_liquidity_usd: Option<f64>,
}

impl MintFacts {
    /// Assemble `MintFacts` from a parsed mint account plus the separately
    /// fetched top-holder share. This is the single place a `ParsedMint`
    /// (mint layout + Token-2022 extensions) is combined with holder
    /// concentration into the facts `assess` scores — both `token-risk-check`
    /// and `payment-watch` build facts through here, so they can never
    /// disagree about what the same on-chain mint looks like.
    pub fn from_parsed(parsed: &crate::token::ParsedMint, top_holder_share_pct: Option<f64>) -> Self {
        MintFacts {
            mint_authority_active: parsed.mint_authority_active,
            freeze_authority_active: parsed.freeze_authority_active,
            top_holder_share_pct,
            has_permanent_delegate: parsed.has_permanent_delegate,
            has_transfer_hook: parsed.has_transfer_hook,
            transfer_fee_bps: parsed.transfer_fee_bps,
            // LP status comes from a different data source entirely (a
            // DEX aggregator, not the mint account) and is opt-in -- the
            // caller sets these two fields afterward, only when the
            // operator has enabled the LP check. `None` here means
            // exactly what it means everywhere else: not attempted yet.
            lp_pool_found: None,
            lp_liquidity_usd: None,
        }
    }
}

/// Below this much total USD liquidity, a pool is thin enough that a
/// modest trade can move the price a lot or drain it outright -- an
/// approximate, adjustable heuristic, not a precise safety boundary.
const LP_THIN_LIQUIDITY_USD_THRESHOLD: f64 = 1000.0;

/// The core scam/risk heuristic. Pure function, no I/O.
///
/// This is deliberately the ONE place this logic lives. Both
/// `token-risk-check` (called directly by the LLM) and `payment-watch`
/// (calling this internally before confirming an inbound payment) use
/// this exact function, so the two plugins can never disagree about what
/// counts as "safe" — and there is nothing here that reads free text, so
/// there is nothing for a prompt-injection attempt to influence.
pub fn assess(facts: &MintFacts) -> RiskReport {
    let mut reasons = Vec::new();
    let mut level = RiskLevel::Green;

    if facts.has_permanent_delegate {
        reasons.push("a permanent delegate can move holder funds without consent".to_string());
        level = RiskLevel::Red;
    }
    // Freeze authority alone is a different, lesser risk category than a
    // permanent delegate: it can only *block* movement (typically behind
    // some accountability -- a regulated issuer's compliance policy, a
    // court order), never silently drain funds the way a permanent
    // delegate can. Real, widely-used regulated stablecoins (e.g. Circle's
    // USDC) genuinely have active freeze authority as a standard control,
    // not a scam signal -- Amber ("review before trusting") reflects that
    // honestly, without hardcoding a specific-issuer allowlist. A mint
    // that's already Red for a real theft primitive (permanent delegate)
    // isn't downgraded by this.
    if facts.freeze_authority_active {
        reasons.push("freeze authority is still active".to_string());
        if level != RiskLevel::Red {
            level = RiskLevel::Amber;
        }
    }
    if facts.mint_authority_active && level != RiskLevel::Red {
        reasons.push("mint authority is still active, supply can be inflated".to_string());
        level = RiskLevel::Amber;
    }
    // Holder concentration: only ever escalates on a definite answer
    // (`Some(pct)`), never on `None` -- see MintFacts::top_holder_share_pct
    // for why a failed lookup must not be conflated with "no concentration
    // risk found". A lookup that never completed can't launder a mint that
    // triggered Red for another reason (checked above), nor invent risk on
    // its own for a mint with no other flags.
    if let Some(pct) = facts.top_holder_share_pct {
        if pct > 50.0 && level == RiskLevel::Green {
            reasons.push(format!("top holder controls {pct:.0}% of supply"));
            level = RiskLevel::Amber;
        }
    }
    if facts.has_transfer_hook && level == RiskLevel::Green {
        reasons.push("token has a transfer hook, review what it does".to_string());
        level = RiskLevel::Amber;
    }
    if facts.transfer_fee_bps > 0 && level == RiskLevel::Green {
        reasons.push(format!(
            "token charges a {}bps transfer fee",
            facts.transfer_fee_bps
        ));
        level = RiskLevel::Amber;
    }
    // LP status: only ever escalates on a definite answer (`Some(...)`),
    // never on `None` -- see MintFacts::lp_pool_found for why absent data
    // must not be conflated with absent liquidity. This can add signal
    // to an otherwise-green mint; it can never launder a mint that's
    // already Red back down.
    if facts.lp_pool_found == Some(false) && level == RiskLevel::Green {
        reasons.push(
            "no on-chain liquidity pool found for this mint -- can't verify it's tradeable"
                .to_string(),
        );
        level = RiskLevel::Amber;
    }
    if level == RiskLevel::Green {
        if let (Some(true), Some(usd)) = (facts.lp_pool_found, facts.lp_liquidity_usd) {
            if usd < LP_THIN_LIQUIDITY_USD_THRESHOLD {
                reasons.push(format!(
                    "liquidity is thin (~${usd:.0} across known pools) -- vulnerable to a \
                     rug or heavy price impact"
                ));
                level = RiskLevel::Amber;
            }
        }
    }
    if reasons.is_empty() {
        reasons.push(
            "no red flags found in mint/freeze authority, holder concentration, \
             or Token-2022 extensions"
                .to_string(),
        );
    }

    RiskReport { level, reasons }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_token_is_green() {
        let facts = MintFacts {
            top_holder_share_pct: Some(10.0),
            ..Default::default()
        };
        assert_eq!(assess(&facts).level, RiskLevel::Green);
    }

    #[test]
    fn permanent_delegate_is_always_red() {
        let facts = MintFacts {
            has_permanent_delegate: true,
            ..Default::default()
        };
        assert_eq!(assess(&facts).level, RiskLevel::Red);
    }

    /// Freeze authority alone (no permanent delegate) is a lesser risk
    /// category -- it can block movement, never silently drain funds --
    /// and real regulated stablecoins (USDC) carry it legitimately. Amber
    /// reflects that honestly; this is a uniform severity rule, not a
    /// specific-issuer carve-out.
    #[test]
    fn active_freeze_authority_alone_is_amber_not_red() {
        let facts = MintFacts {
            freeze_authority_active: true,
            ..Default::default()
        };
        let report = assess(&facts);
        assert_eq!(report.level, RiskLevel::Amber);
        assert!(report.reasons.iter().any(|r| r.contains("freeze authority")));
    }

    /// The lesser severity of freeze authority must never mask a real
    /// theft primitive -- a permanent delegate stays Red regardless of
    /// what else is true about the mint.
    #[test]
    fn freeze_authority_cannot_soften_a_permanent_delegate_verdict() {
        let facts = MintFacts {
            has_permanent_delegate: true,
            freeze_authority_active: true,
            ..Default::default()
        };
        assert_eq!(assess(&facts).level, RiskLevel::Red);
    }

    /// Capping freeze authority's *severity* must not cost the reader the
    /// *information*: a mint that is Red for a permanent delegate and also
    /// carries freeze authority has to report both, or a Red verdict would
    /// silently understate what was actually found. `payment-watch`'s
    /// README documents exactly this two-reason output, so this test is
    /// what keeps that example honest.
    #[test]
    fn a_red_mint_still_reports_its_freeze_authority_reason() {
        let facts = MintFacts {
            has_permanent_delegate: true,
            freeze_authority_active: true,
            ..Default::default()
        };
        let report = assess(&facts);
        assert_eq!(report.level, RiskLevel::Red);
        assert!(report.reasons.iter().any(|r| r.contains("permanent delegate")));
        assert!(report.reasons.iter().any(|r| r.contains("freeze authority")));
    }

    #[test]
    fn concentrated_holders_is_amber_not_red() {
        let facts = MintFacts {
            top_holder_share_pct: Some(80.0),
            ..Default::default()
        };
        assert_eq!(assess(&facts).level, RiskLevel::Amber);
    }

    #[test]
    fn reasons_are_never_empty() {
        let facts = MintFacts::default();
        assert!(!assess(&facts).reasons.is_empty());
    }

    // ---- LP status --------------------------------------------------------

    #[test]
    fn no_lp_pool_found_is_amber_not_green() {
        let facts = MintFacts {
            top_holder_share_pct: Some(5.0),
            lp_pool_found: Some(false),
            ..Default::default()
        };
        let report = assess(&facts);
        assert_eq!(report.level, RiskLevel::Amber);
        assert!(report.reasons.iter().any(|r| r.contains("no on-chain liquidity pool")));
    }

    #[test]
    fn thin_liquidity_is_amber() {
        let facts = MintFacts {
            top_holder_share_pct: Some(5.0),
            lp_pool_found: Some(true),
            lp_liquidity_usd: Some(42.0),
            ..Default::default()
        };
        let report = assess(&facts);
        assert_eq!(report.level, RiskLevel::Amber);
        assert!(report.reasons.iter().any(|r| r.contains("thin")));
    }

    #[test]
    fn healthy_liquidity_stays_green() {
        let facts = MintFacts {
            top_holder_share_pct: Some(5.0),
            lp_pool_found: Some(true),
            lp_liquidity_usd: Some(1_117_249.51),
            ..Default::default()
        };
        assert_eq!(assess(&facts).level, RiskLevel::Green);
    }

    /// The whole point of `Option`: a lookup that was never attempted (no
    /// operator opt-in, an unreachable API, an unconfigured deployment)
    /// must never be treated as a red flag by itself. Otherwise the LP
    /// check would silently downgrade every mint in every deployment that
    /// hasn't enabled it -- including, previously, this repo's own live
    /// devnet demo mints.
    #[test]
    fn missing_lp_data_does_not_change_a_clean_verdict() {
        let facts = MintFacts {
            top_holder_share_pct: Some(5.0),
            lp_pool_found: None,
            lp_liquidity_usd: None,
            ..Default::default()
        };
        assert_eq!(assess(&facts).level, RiskLevel::Green);
    }

    /// The other half of the fail-open guarantee: missing LP data can
    /// never be exploited to launder an already-dangerous mint. A
    /// permanent delegate is Red regardless of what the LP fields say.
    #[test]
    fn missing_lp_data_cannot_mask_a_red_verdict() {
        let facts = MintFacts {
            has_permanent_delegate: true,
            lp_pool_found: None,
            ..Default::default()
        };
        assert_eq!(assess(&facts).level, RiskLevel::Red);
    }

    #[test]
    fn from_parsed_carries_every_field_through() {
        use crate::token::ParsedMint;
        let parsed = ParsedMint {
            mint_authority_active: true,
            freeze_authority_active: true,
            supply: 1000,
            has_permanent_delegate: true,
            has_transfer_hook: true,
            transfer_fee_bps: 300,
        };
        let facts = MintFacts::from_parsed(&parsed, Some(62.5));
        assert!(facts.mint_authority_active);
        assert!(facts.freeze_authority_active);
        assert!(facts.has_permanent_delegate);
        assert!(facts.has_transfer_hook);
        assert_eq!(facts.transfer_fee_bps, 300);
        assert_eq!(facts.top_holder_share_pct, Some(62.5));
        // LP status isn't part of the mint account -- from_parsed leaves
        // it unattempted, for the shim to enrich separately.
        assert_eq!(facts.lp_pool_found, None);
        assert_eq!(facts.lp_liquidity_usd, None);
        // A mint with a permanent delegate is unambiguously red.
        assert_eq!(assess(&facts).level, RiskLevel::Red);
    }

    // ---- holder-concentration fail-open ------------------------------------
    //
    // `getTokenLargestAccounts` is rejected outright by some RPC providers
    // for very widely-held mints -- found live against real mainnet USDC on
    // Helius ("Too many accounts requested"). A failed/skipped lookup must
    // behave exactly like the LP fail-open contract above: never invents
    // risk on a clean mint, never launders a mint that's already Red.

    #[test]
    fn missing_holder_data_does_not_change_a_clean_verdict() {
        let facts = MintFacts { top_holder_share_pct: None, ..Default::default() };
        assert_eq!(assess(&facts).level, RiskLevel::Green);
    }

    #[test]
    fn missing_holder_data_cannot_mask_a_red_verdict() {
        let facts = MintFacts {
            has_permanent_delegate: true,
            top_holder_share_pct: None,
            ..Default::default()
        };
        assert_eq!(assess(&facts).level, RiskLevel::Red);
    }
}
