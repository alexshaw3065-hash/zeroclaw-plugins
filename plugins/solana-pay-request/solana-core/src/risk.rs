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
    pub top_holder_share_pct: f64,
    pub has_permanent_delegate: bool,
    pub has_transfer_hook: bool,
    pub transfer_fee_bps: u16,
}

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
    if facts.freeze_authority_active {
        reasons.push("freeze authority is still active".to_string());
        level = RiskLevel::Red;
    }
    if facts.mint_authority_active && level != RiskLevel::Red {
        reasons.push("mint authority is still active, supply can be inflated".to_string());
        level = RiskLevel::Amber;
    }
    if facts.top_holder_share_pct > 50.0 && level == RiskLevel::Green {
        reasons.push(format!(
            "top holder controls {:.0}% of supply",
            facts.top_holder_share_pct
        ));
        level = RiskLevel::Amber;
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
            top_holder_share_pct: 10.0,
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

    #[test]
    fn active_freeze_authority_is_red() {
        let facts = MintFacts {
            freeze_authority_active: true,
            ..Default::default()
        };
        assert_eq!(assess(&facts).level, RiskLevel::Red);
    }

    #[test]
    fn concentrated_holders_is_amber_not_red() {
        let facts = MintFacts {
            top_holder_share_pct: 80.0,
            ..Default::default()
        };
        assert_eq!(assess(&facts).level, RiskLevel::Amber);
    }

    #[test]
    fn reasons_are_never_empty() {
        let facts = MintFacts::default();
        assert!(!assess(&facts).reasons.is_empty());
    }
}
