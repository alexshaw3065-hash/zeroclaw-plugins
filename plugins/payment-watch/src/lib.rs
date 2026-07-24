//! payment-watch
//!
//! T0 (read-only). Watches an address for an expected payment (amount +
//! optional SPL mint, correlated by a Solana Pay `reference`) and, the
//! moment a matching transfer has landed, reports it -- but only after
//! screening the token it was paid in for scam risk.
//!
//! ## The fusion (the reason this is one system, not three tools)
//!
//! Before this plugin will ever emit a "paid" result, `core::confirm`
//! calls `zeroclaw_solana_core::risk::assess` on the mint that actually
//! paid -- the exact same function `token-risk-check` runs. It is a plain
//! internal function call inside tested core, not a request to the LLM to
//! "remember" to double-check, so the screening cannot be skipped, talked
//! out of, or prompt-injected away. There is no code path that produces a
//! confirmation without a risk verdict attached.
//!
//! Pure-core / thin-shim split: all matching, parsing, and the fused risk
//! call live in `core` (host-tested with mocked RPC JSON, no network); the
//! `#[cfg(target_family = "wasm")]` shim only makes the RPC calls and
//! shapes the result.

pub mod core {
    use serde::{Deserialize, Serialize};
    use serde_json::Value;
    use zeroclaw_solana_core::risk::{assess, MintFacts, RiskReport};

    #[derive(Debug, Deserialize)]
    pub struct Args {
        /// Wallet address expected to receive the payment.
        pub recipient: String,
        /// Expected amount as a decimal string, e.g. "25" or "25.50".
        pub amount: String,
        /// Expected SPL token mint. Omit for native SOL.
        #[serde(default)]
        pub mint: Option<String>,
        /// Solana Pay reference (base58) to correlate the payment by. Strongly
        /// recommended, and required for reliable SPL detection -- see the
        /// module note and README.
        #[serde(default)]
        pub reference: Option<String>,
    }

    /// One transfer *received by the watched recipient*, distilled from a
    /// transaction's balance deltas. Plain data so core matching is fully
    /// host-testable; the shim builds these from real RPC responses via
    /// [`transfers_from_tx_meta`].
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ObservedTransfer {
        pub signature: String,
        /// The mint received, or `None` for native SOL.
        pub mint: Option<String>,
        /// Amount received, in the mint's raw base units.
        pub amount_raw: u64,
        /// Decimals of the received token (9 for native SOL).
        pub decimals: u8,
        /// Whether the invoice's requested `reference` (if any) was found
        /// as an account key in this same transaction -- `true` when no
        /// reference was requested at all (the check doesn't apply, so it
        /// can't fail it). Defense in depth, independent of whichever
        /// address the shim happened to query
        /// `getSignaturesForAddress` by: see `match_payment`.
        pub has_reference: bool,
    }

    /// Where the requested amount stands relative to whatever's actually
    /// landed. `Match` only ever appears on a "paid" result. `Under` and
    /// `None` are purely diagnostic -- informational detail surfaced on a
    /// "pending" result, never a factor in the paid/pending decision
    /// itself (that stays entirely `match_payment`'s job, unchanged).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "lowercase")]
    pub enum AmountStatus {
        /// A landed transfer meets or exceeds the requested amount.
        Match,
        /// A transfer for the right mint (and reference, if requested) has
        /// landed, but for less than the requested amount.
        Under,
        /// No transfer relevant to this invoice has landed yet.
        None,
    }

    /// Per-field verification detail -- makes the result individually
    /// auditable instead of one opaque status string. On a "paid" result
    /// every field here reflects something `match_payment` already
    /// enforced (not merely echoed input) and is unconditionally
    /// satisfied, by construction. On a "pending" result these are an
    /// honest, best-effort diagnosis of what -- if anything -- has been
    /// observed so far, computed by inspecting the same already-fetched
    /// transfer list `match_payment` looked at; this diagnosis never
    /// feeds back into the paid/pending decision itself, only into what
    /// gets displayed while waiting.
    #[derive(Debug, Serialize, PartialEq)]
    pub struct TrustReport {
        /// Whether anything at all has landed at the requested recipient
        /// (any mint, any amount, any reference). Always `true` on a
        /// "paid" result, by construction (`transfers_from_tx_meta` only
        /// ever produces transfers for the exact recipient it was asked
        /// about, so an unmatched transfer never enters `observed` for a
        /// different recipient in the first place).
        pub recipient_verified: bool,
        /// See `AmountStatus`.
        pub amount_status: AmountStatus,
        /// Whether a transfer in the requested mint (native SOL matches a
        /// mint-less request) has been observed.
        pub mint_verified: bool,
        /// `None` when no `reference` was requested at all (not
        /// applicable), or when no mint-matching transfer has been
        /// observed yet to check it against. `Some(true)`/`Some(false)`
        /// once there's a candidate transfer to actually check -- on a
        /// "paid" result this is always `Some(true)` when a reference was
        /// requested, since `match_payment` filters out anything else.
        pub reference_verified: Option<bool>,
        /// Whether a transaction satisfying the full request (recipient +
        /// mint + amount + reference) has actually been confirmed.
        /// `true` only on "paid".
        pub tx_confirmed: bool,
    }

    #[derive(Debug, Serialize, PartialEq)]
    pub struct Output {
        /// "paid" or "pending".
        pub status: String,
        pub signature: Option<String>,
        /// Echoed expected amount (human decimal).
        pub amount: Option<String>,
        pub mint: Option<String>,
        /// Risk verdict on the paying mint: "green" | "amber" | "red".
        /// Present only on a "paid" result -- a confirmation always carries
        /// one, by construction.
        pub risk_level: Option<String>,
        pub risk_reasons: Vec<String>,
        /// One short human-readable sentence for the chat channel. Kept
        /// deliberately compact (bounty trap #3: never dump raw RPC JSON).
        pub summary: String,
        /// "R$<amount * brl_rate>" for a confirmed payment, present only
        /// when the operator has configured a `brl_rate` -- the root
        /// README's "Brazil touch." Always `None` on a "pending" result;
        /// a not-yet-landed payment has nothing confirmed to convert.
        pub brl_estimate: Option<String>,
        /// Per-field verification detail -- see `TrustReport`. Always
        /// present: on "paid" every field is satisfied; on "pending" it's
        /// an honest diagnosis of what's been observed so far.
        pub trust_report: TrustReport,
        /// The exact, ready-to-send reply text -- see `format_reply`.
        /// Built here, in core, from the fields above, so it's
        /// host-tested like everything else; the shim's job is to hand
        /// this straight to the channel, not compose its own wording.
        pub reply: String,
    }

    #[derive(Debug, PartialEq, Eq)]
    pub enum CoreError {
        BadInput(String),
    }

    impl std::fmt::Display for CoreError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                CoreError::BadInput(s) => write!(f, "bad input: {s}"),
            }
        }
    }

    /// Convert a decimal amount string into the mint's raw base units.
    ///
    /// "25" at 6 decimals -> 25_000_000; "25.5" -> 25_500_000. Rejects
    /// anything that isn't a clean non-negative decimal, and rejects more
    /// fractional digits than the mint actually has (that would be an
    /// amount the token cannot represent, not something to silently
    /// truncate). Hand-rolled rather than via `f64`, because float parsing
    /// would accept "inf"/"nan"/"1e6" and lose precision on money.
    pub fn decimal_to_raw(amount: &str, decimals: u8) -> Result<u64, CoreError> {
        if amount.is_empty() {
            return Err(CoreError::BadInput("empty amount".into()));
        }
        let mut parts = amount.split('.');
        let int_part = parts.next().unwrap_or("");
        let frac_part = parts.next().unwrap_or("");
        if parts.next().is_some() {
            return Err(CoreError::BadInput(format!("more than one '.' in {amount:?}")));
        }
        if int_part.is_empty() && frac_part.is_empty() {
            return Err(CoreError::BadInput(format!("no digits in {amount:?}")));
        }
        if !int_part.chars().all(|c| c.is_ascii_digit())
            || !frac_part.chars().all(|c| c.is_ascii_digit())
        {
            return Err(CoreError::BadInput(format!("non-numeric amount {amount:?}")));
        }
        let decimals = decimals as usize;
        if frac_part.len() > decimals {
            return Err(CoreError::BadInput(format!(
                "amount {amount:?} has more fractional digits than the token's {decimals} decimals"
            )));
        }
        // Right-pad the fractional part out to `decimals` digits, then the
        // whole thing is just an integer count of base units.
        let mut digits = String::with_capacity(int_part.len() + decimals);
        digits.push_str(int_part);
        digits.push_str(frac_part);
        for _ in 0..(decimals - frac_part.len()) {
            digits.push('0');
        }
        let trimmed = digits.trim_start_matches('0');
        if trimmed.is_empty() {
            return Err(CoreError::BadInput(format!("amount {amount:?} is zero")));
        }
        trimmed
            .parse::<u64>()
            .map_err(|_| CoreError::BadInput(format!("amount {amount:?} is too large")))
    }

    /// Parse the `result` of a `getTransaction` (jsonParsed) response into
    /// the transfers *received by `recipient`* in that transaction. Reads
    /// balance deltas (the reliable way to see "how much did this address
    /// actually receive"), not instruction decoding. A failed transaction
    /// (`meta.err != null`) transferred nothing and yields no transfers.
    ///
    /// `reference`, when the invoice requested one, is checked here too --
    /// defense in depth. The shim already tries to query
    /// `getSignaturesForAddress` by the reference when one is set, but
    /// this stamps every transfer parsed from this transaction with
    /// whether that reference is *actually* one of its account keys, so
    /// `match_payment` doesn't have to trust the query filter alone (see
    /// CLAUDE.md's "step 2" note on the cross-invoice collision risk this
    /// closes).
    pub fn transfers_from_tx_meta(
        result: &Value,
        recipient: &str,
        reference: Option<&str>,
    ) -> Vec<ObservedTransfer> {
        let mut out = Vec::new();
        let meta = match result.get("meta") {
            Some(m) if !m.is_null() => m,
            _ => return out,
        };
        if !meta.get("err").map(Value::is_null).unwrap_or(true) {
            return out; // transaction failed -- no value moved
        }
        let signature = result
            .get("transaction")
            .and_then(|t| t.get("signatures"))
            .and_then(Value::as_array)
            .and_then(|s| s.first())
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        // No reference requested at all -> the check doesn't apply, so it
        // can't fail it (`true`, vacuously). A reference requested but not
        // found among this transaction's account keys -> `false`.
        let has_reference = reference.is_none_or(|r| account_index_of(result, r).is_some());

        // --- SPL token receipts, from pre/post token balance deltas -------
        let empty = Vec::new();
        let post_tb = meta.get("postTokenBalances").and_then(Value::as_array).unwrap_or(&empty);
        let pre_tb = meta.get("preTokenBalances").and_then(Value::as_array).unwrap_or(&empty);
        for post in post_tb {
            if post.get("owner").and_then(Value::as_str) != Some(recipient) {
                continue;
            }
            let account_index = post.get("accountIndex").and_then(Value::as_u64);
            let mint = post.get("mint").and_then(Value::as_str);
            let (post_amt, decimals) = match token_amount(post) {
                Some(v) => v,
                None => continue,
            };
            // Same token account's balance before this tx (0 if it didn't
            // exist yet -- a freshly created ATA).
            let pre_amt = pre_tb
                .iter()
                .find(|p| p.get("accountIndex").and_then(Value::as_u64) == account_index)
                .and_then(|p| token_amount(p).map(|(a, _)| a))
                .unwrap_or(0);
            if let Some(received) = post_amt.checked_sub(pre_amt).filter(|&d| d > 0) {
                out.push(ObservedTransfer {
                    signature: signature.clone(),
                    mint: mint.map(str::to_string),
                    amount_raw: received,
                    decimals,
                    has_reference,
                });
            }
        }

        // --- native SOL receipt, from pre/post lamport balance delta ------
        if let Some(idx) = account_index_of(result, recipient) {
            let pre = meta
                .get("preBalances")
                .and_then(Value::as_array)
                .and_then(|b| b.get(idx))
                .and_then(Value::as_u64);
            let post = meta
                .get("postBalances")
                .and_then(Value::as_array)
                .and_then(|b| b.get(idx))
                .and_then(Value::as_u64);
            if let (Some(pre), Some(post)) = (pre, post) {
                if let Some(received) = post.checked_sub(pre).filter(|&d| d > 0) {
                    out.push(ObservedTransfer {
                        signature: signature.clone(),
                        mint: None,
                        amount_raw: received,
                        decimals: 9,
                        has_reference,
                    });
                }
            }
        }

        out
    }

    /// `(amount_raw, decimals)` from a token-balance entry's `uiTokenAmount`.
    fn token_amount(entry: &Value) -> Option<(u64, u8)> {
        let ui = entry.get("uiTokenAmount")?;
        let amount = ui.get("amount").and_then(Value::as_str)?.parse::<u64>().ok()?;
        let decimals = ui.get("decimals").and_then(Value::as_u64)? as u8;
        Some((amount, decimals))
    }

    /// Index of `target` in the transaction's account keys. Handles both the
    /// jsonParsed shape (array of `{pubkey, ...}`) and the raw shape (array
    /// of address strings).
    fn account_index_of(result: &Value, target: &str) -> Option<usize> {
        let keys = result
            .get("transaction")?
            .get("message")?
            .get("accountKeys")?
            .as_array()?;
        keys.iter().position(|k| {
            k.as_str() == Some(target)
                || k.get("pubkey").and_then(Value::as_str) == Some(target)
        })
    }

    /// Extract candidate signatures from a `getSignaturesForAddress`
    /// response's `result`, skipping any that errored on chain.
    pub fn signatures_from_response(result: &Value) -> Vec<String> {
        result
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter(|e| e.get("err").map(Value::is_null).unwrap_or(true))
                    .filter_map(|e| e.get("signature").and_then(Value::as_str))
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Find the first observed transfer that satisfies the expected payment:
    /// the right mint (both native, or the same SPL mint), the requested
    /// `reference` actually present in that same transaction when one was
    /// requested (defense in depth -- see `ObservedTransfer::has_reference`),
    /// and at least the expected amount (overpayment counts as paid;
    /// underpayment -- including a "dust" decoy transfer -- does not).
    /// Returns an index into `observed`, or `None` if nothing matches --
    /// there is no argument, memo, or reference value that can make this
    /// return a match for a payment that did not actually land.
    pub fn match_payment(
        args: &Args,
        observed: &[ObservedTransfer],
    ) -> Result<Option<usize>, CoreError> {
        let expected_mint = args.mint.as_deref();
        for (i, t) in observed.iter().enumerate() {
            if t.mint.as_deref() != expected_mint {
                continue;
            }
            if args.reference.is_some() && !t.has_reference {
                continue;
            }
            let expected_raw = decimal_to_raw(&args.amount, t.decimals)?;
            if t.amount_raw >= expected_raw {
                return Ok(Some(i));
            }
        }
        Ok(None)
    }

    /// Build a "paid" result for a matched transfer. **This is the fused
    /// point:** it calls `assess` on the paying mint's facts here,
    /// unconditionally, so no confirmation can exist without a risk verdict.
    /// The shim fetches `MintFacts` for an SPL mint (or passes
    /// `MintFacts::default()` for native SOL, which `assess` scores green --
    /// native SOL has no mint/freeze authority, delegate, hook, or fee).
    ///
    /// `brl_rate` is BRL per one unit of `args.amount`'s asset, read from
    /// config by the shim (`None` when unset) -- see `format_brl` for why
    /// this is parsed as `f64` even though `args.amount` deliberately
    /// never is.
    pub fn confirm(
        args: &Args,
        matched: &ObservedTransfer,
        facts: &MintFacts,
        brl_rate: Option<f64>,
    ) -> Output {
        let report: RiskReport = assess(facts);
        let level = format!("{:?}", report.level).to_lowercase();
        let sig_short = short_sig(&matched.signature);

        let asset = match &matched.mint {
            Some(m) => format!("{} of {}", args.amount, short_addr(m)),
            None => format!("{} SOL", args.amount),
        };
        let summary = if report.level == zeroclaw_solana_core::RiskLevel::Red {
            format!(
                "Payment landed ({asset}, sig {sig_short}) but the paying token is FLAGGED \
                 RED -- {}. Do not treat this as a safe payment.",
                report.reasons.first().cloned().unwrap_or_default()
            )
        } else {
            format!(
                "Payment confirmed: {asset} received (sig {sig_short}). Paying token risk: \
                 {}.",
                level.to_uppercase()
            )
        };

        // Every field here is `true`/`Match` by construction: `matched`
        // only ever reached this function via `match_payment`, which
        // already enforced recipient (transfers_from_tx_meta only parses
        // transfers credited to the requested recipient), mint, amount,
        // and reference. This isn't new checking -- it's making
        // already-enforced guarantees legible per-field instead of
        // collapsing them into one opaque "paid".
        let trust_report = TrustReport {
            recipient_verified: true,
            amount_status: AmountStatus::Match,
            mint_verified: true,
            reference_verified: args.reference.is_some().then_some(true),
            tx_confirmed: true,
        };

        let mut output = Output {
            status: "paid".to_string(),
            signature: Some(matched.signature.clone()),
            amount: Some(args.amount.clone()),
            mint: matched.mint.clone(),
            risk_level: Some(level),
            risk_reasons: report.reasons,
            summary,
            brl_estimate: brl_rate.and_then(|rate| format_brl(&args.amount, rate)),
            trust_report,
            reply: String::new(),
        };
        output.reply = format_reply(&output);
        output
    }

    /// Build a "pending" result -- nothing matching (mint + amount +
    /// reference) has landed yet. `observed` is the same already-fetched
    /// transfer list `match_payment` looked at; inspecting it again here is
    /// purely diagnostic (see `diagnose_pending`) and changes nothing
    /// about the paid/pending decision itself, which already happened
    /// before this function was ever called.
    pub fn pending(args: &Args, observed: &[ObservedTransfer]) -> Output {
        let asset = match &args.mint {
            Some(m) => format!("{} of {}", args.amount, short_addr(m)),
            None => format!("{} SOL", args.amount),
        };
        let trust_report = diagnose_pending(args, observed);
        let mut output = Output {
            status: "pending".to_string(),
            signature: None,
            amount: Some(args.amount.clone()),
            mint: args.mint.clone(),
            risk_level: None,
            risk_reasons: Vec::new(),
            summary: format!("No matching payment yet ({asset} to {}).", short_addr(&args.recipient)),
            brl_estimate: None,
            trust_report,
            reply: String::new(),
        };
        output.reply = format_reply(&output);
        output
    }

    /// Best-effort, display-only diagnosis of what's been observed so far,
    /// for a "pending" result's `TrustReport`. Looks for the closest
    /// candidate transfer to this invoice (right mint, and right reference
    /// when one was requested) among `observed` -- if one exists, it must
    /// be an underpayment (`AmountStatus::Under`): if it had met the
    /// requested amount, `match_payment` would already have matched it and
    /// `confirm` would have run instead of `pending`. This function never
    /// influences that decision, only what gets displayed while waiting.
    fn diagnose_pending(args: &Args, observed: &[ObservedTransfer]) -> TrustReport {
        let expected_mint = args.mint.as_deref();
        let candidate = observed.iter().find(|t| {
            t.mint.as_deref() == expected_mint && (args.reference.is_none() || t.has_reference)
        });

        TrustReport {
            // `observed` only ever contains transfers credited to the
            // requested recipient (see transfers_from_tx_meta) -- "is
            // there anything at all" is the honest question this can
            // answer, not "is the recipient correct" (that's not a
            // meaningful failure mode here).
            recipient_verified: !observed.is_empty(),
            amount_status: if candidate.is_some() { AmountStatus::Under } else { AmountStatus::None },
            mint_verified: candidate.is_some(),
            reference_verified: match (args.reference.is_some(), candidate) {
                (false, _) => None,
                (true, None) => None,
                (true, Some(t)) => Some(t.has_reference),
            },
            tx_confirmed: false,
        }
    }

    fn short_sig(sig: &str) -> String {
        short_addr(sig)
    }

    /// Abbreviate a long base58 string as `head…tail` for compact summaries.
    fn short_addr(s: &str) -> String {
        if s.len() <= 12 {
            return s.to_string();
        }
        format!("{}…{}", &s[..4], &s[s.len() - 4..])
    }

    /// "R$<amount * rate>", formatted to 2 decimal places. `None` on a
    /// non-finite rate/amount rather than a nonsense figure -- display
    /// only, so failing silently by omitting the estimate is the right
    /// default, never something that should block a real confirmation.
    fn format_brl(amount: &str, rate: f64) -> Option<String> {
        let amount: f64 = amount.parse().ok()?;
        let brl = amount * rate;
        if !brl.is_finite() {
            return None;
        }
        Some(format!("R${brl:.2}"))
    }

    /// The exact reply text a channel should send verbatim: a checklist
    /// built from `TrustReport`'s already-computed fields, plus (on
    /// "paid") the real risk verdict and reasons from `assess` -- never an
    /// invented confidence score, only real green/amber/red and the real
    /// booleans already on `output`. Pure formatting over an already-built
    /// `Output`, no new logic: the paid/pending decision and the risk
    /// verdict both happened before this function is ever called.
    fn format_reply(output: &Output) -> String {
        let report = &output.trust_report;
        let mut lines = vec!["Payment Verification".to_string()];

        lines.push(checklist_line(
            report.amount_status == AmountStatus::Match,
            "Amount matches",
            match report.amount_status {
                AmountStatus::Under => " (underpaid)",
                AmountStatus::Match | AmountStatus::None => "",
            },
        ));
        lines.push(checklist_line(report.recipient_verified, "Recipient verified", ""));
        if let Some(reference_ok) = report.reference_verified {
            lines.push(checklist_line(reference_ok, "Reference matches", ""));
        }
        lines.push(checklist_line(report.tx_confirmed, "Transaction confirmed", ""));

        if output.status == "paid" {
            let level = output.risk_level.as_deref().unwrap_or("unknown");
            // A colored circle here, not the ✓/✗ every other line uses --
            // green/amber/red is a genuine three-way severity signal, and
            // collapsing amber into the same ✗ as red (as a checkmark
            // would) loses real information a color doesn't.
            let circle = match level {
                "green" => "\u{1F7E2}", // 🟢
                "amber" => "\u{1F7E0}", // 🟠
                "red" => "\u{1F534}",   // 🔴
                // Defensive only: a "paid" result always carries a real
                // risk_level by construction (see the module doc comment
                // on the fusion) -- this arm should be unreachable.
                _ => "\u{26AA}", // ⚪
            };
            lines.push(format!(
                "{circle} Token risk: {} — {}",
                level.to_uppercase(),
                output.risk_reasons.join("; "),
            ));
            // Only present when `brl_estimate` is `Some` -- an operator
            // who never sets `brl_rate` sees the exact same reply as
            // before this line existed. `brl_estimate` is only ever
            // populated on the "paid" path (see `confirm`), matching
            // where this line is inserted.
            if let Some(estimate) = &output.brl_estimate {
                lines.push(format!("Est.: {estimate}"));
            }
            lines.push(
                match level {
                    "green" => "Verdict: PAYMENT VERIFIED — safe to trust.",
                    "red" => "Verdict: DO NOT TRUST THIS PAYMENT.",
                    _ => "Verdict: PAYMENT LANDED but flagged AMBER — review before trusting.",
                }
                .to_string(),
            );
        } else {
            lines.push("Verdict: NOT CONFIRMED — do not treat this as paid.".to_string());
        }

        lines.join("\n")
    }

    fn checklist_line(ok: bool, label: &str, suffix: &str) -> String {
        let mark = if ok { "\u{2713}" } else { "\u{2717}" };
        format!("{mark} {label}{suffix}")
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use serde_json::json;
        use zeroclaw_solana_core::risk::MintFacts;

        const RECIPIENT: &str = "9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin";
        const USDC: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

        // ---- decimal_to_raw ------------------------------------------------

        #[test]
        fn decimal_to_raw_whole_number() {
            assert_eq!(decimal_to_raw("25", 6).unwrap(), 25_000_000);
        }

        #[test]
        fn decimal_to_raw_with_fraction() {
            assert_eq!(decimal_to_raw("25.5", 6).unwrap(), 25_500_000);
            assert_eq!(decimal_to_raw("0.000001", 6).unwrap(), 1);
        }

        #[test]
        fn decimal_to_raw_rejects_overprecision() {
            // 7 fractional digits against a 6-decimal token.
            assert!(decimal_to_raw("1.0000001", 6).is_err());
        }

        #[test]
        fn decimal_to_raw_rejects_zero_and_junk() {
            assert!(decimal_to_raw("0", 6).is_err());
            assert!(decimal_to_raw("0.00", 6).is_err());
            assert!(decimal_to_raw("", 6).is_err());
            assert!(decimal_to_raw("twenty", 6).is_err());
            assert!(decimal_to_raw("1.2.3", 6).is_err());
            assert!(decimal_to_raw("-5", 6).is_err());
        }

        // ---- transfers_from_tx_meta ---------------------------------------

        fn spl_tx(owner: &str, mint: &str, pre: &str, post: &str, decimals: u64) -> Value {
            json!({
                "meta": {
                    "err": null,
                    "preTokenBalances": [
                        {"accountIndex": 3, "mint": mint, "owner": owner,
                         "uiTokenAmount": {"amount": pre, "decimals": decimals}}
                    ],
                    "postTokenBalances": [
                        {"accountIndex": 3, "mint": mint, "owner": owner,
                         "uiTokenAmount": {"amount": post, "decimals": decimals}}
                    ]
                },
                "transaction": {"signatures": ["5SoLpAyRefSig1111111111111111111111111111111111111111111111111111"]}
            })
        }

        #[test]
        fn parses_an_spl_receipt_from_balance_delta() {
            let tx = spl_tx(RECIPIENT, USDC, "0", "25000000", 6);
            let transfers = transfers_from_tx_meta(&tx, RECIPIENT, None);
            assert_eq!(transfers.len(), 1);
            assert_eq!(transfers[0].mint.as_deref(), Some(USDC));
            assert_eq!(transfers[0].amount_raw, 25_000_000);
            assert_eq!(transfers[0].decimals, 6);
            // No reference was requested -- vacuously true.
            assert!(transfers[0].has_reference);
        }

        #[test]
        fn ignores_token_balances_for_other_owners() {
            let tx = spl_tx("SomeOtherOwner11111111111111111111111111111", USDC, "0", "25000000", 6);
            assert!(transfers_from_tx_meta(&tx, RECIPIENT, None).is_empty());
        }

        #[test]
        fn a_failed_transaction_yields_no_transfers() {
            let mut tx = spl_tx(RECIPIENT, USDC, "0", "25000000", 6);
            tx["meta"]["err"] = json!({"InstructionError": [0, "Custom"]});
            assert!(transfers_from_tx_meta(&tx, RECIPIENT, None).is_empty());
        }

        #[test]
        fn parses_a_native_sol_receipt() {
            let tx = json!({
                "meta": {
                    "err": null,
                    "preBalances": [1000, 500],
                    "postBalances": [1000, 1_000_000_500u64]
                },
                "transaction": {
                    "message": {"accountKeys": [{"pubkey": "Fee1111111111111111111111111111111111111111"}, {"pubkey": RECIPIENT}]},
                    "signatures": ["NativeSig11111111111111111111111111111111111111111111111111111111"]
                }
            });
            let transfers = transfers_from_tx_meta(&tx, RECIPIENT, None);
            assert_eq!(transfers.len(), 1);
            assert!(transfers[0].mint.is_none());
            assert_eq!(transfers[0].amount_raw, 1_000_000_000);
            assert_eq!(transfers[0].decimals, 9);
        }

        // ---- reference defense-in-depth ------------------------------------

        const REFERENCE: &str = "So11111111111111111111111111111111111111112";

        fn spl_tx_with_accounts(
            owner: &str,
            mint: &str,
            pre: &str,
            post: &str,
            decimals: u64,
            account_keys: &[&str],
        ) -> Value {
            let mut tx = spl_tx(owner, mint, pre, post, decimals);
            tx["transaction"]["message"] = json!({
                "accountKeys": account_keys.iter().map(|k| json!({"pubkey": k})).collect::<Vec<_>>()
            });
            tx
        }

        #[test]
        fn has_reference_true_when_the_reference_is_an_account_key() {
            let tx = spl_tx_with_accounts(RECIPIENT, USDC, "0", "25000000", 6, &[RECIPIENT, REFERENCE]);
            let transfers = transfers_from_tx_meta(&tx, RECIPIENT, Some(REFERENCE));
            assert_eq!(transfers.len(), 1);
            assert!(transfers[0].has_reference);
        }

        #[test]
        fn has_reference_false_when_the_reference_is_missing() {
            // A transaction that pays the right recipient/mint/amount but
            // simply doesn't carry the requested reference as an account
            // key -- e.g. an unrelated payment for a different invoice.
            let tx = spl_tx_with_accounts(RECIPIENT, USDC, "0", "25000000", 6, &[RECIPIENT]);
            let transfers = transfers_from_tx_meta(&tx, RECIPIENT, Some(REFERENCE));
            assert_eq!(transfers.len(), 1);
            assert!(!transfers[0].has_reference);
        }

        // ---- signatures_from_response -------------------------------------

        #[test]
        fn extracts_signatures_and_skips_errored_ones() {
            let result = json!([
                {"signature": "good1", "err": null},
                {"signature": "bad1", "err": {"x": 1}},
                {"signature": "good2", "err": null}
            ]);
            assert_eq!(signatures_from_response(&result), vec!["good1", "good2"]);
        }

        // ---- match_payment ------------------------------------------------

        fn spl_transfer(amount_raw: u64) -> ObservedTransfer {
            ObservedTransfer {
                signature: "sig".into(),
                mint: Some(USDC.to_string()),
                amount_raw,
                decimals: 6,
                has_reference: true,
            }
        }

        fn args_for(amount: &str, mint: Option<&str>) -> Args {
            Args {
                recipient: RECIPIENT.to_string(),
                amount: amount.to_string(),
                mint: mint.map(str::to_string),
                reference: None,
            }
        }

        #[test]
        fn matches_exact_payment() {
            let observed = [spl_transfer(25_000_000)];
            assert_eq!(match_payment(&args_for("25", Some(USDC)), &observed).unwrap(), Some(0));
        }

        #[test]
        fn overpayment_counts_as_paid() {
            let observed = [spl_transfer(30_000_000)];
            assert_eq!(match_payment(&args_for("25", Some(USDC)), &observed).unwrap(), Some(0));
        }

        #[test]
        fn underpayment_does_not_match() {
            let observed = [spl_transfer(24_999_999)];
            assert_eq!(match_payment(&args_for("25", Some(USDC)), &observed).unwrap(), None);
        }

        #[test]
        fn wrong_mint_does_not_match() {
            let observed = [spl_transfer(25_000_000)];
            let other = "So11111111111111111111111111111111111111112";
            assert_eq!(match_payment(&args_for("25", Some(other)), &observed).unwrap(), None);
        }

        /// Dust-defense: a near-zero decoy transfer in the right mint (the
        /// classic "dust" pattern -- send a tiny amount so a naive
        /// "any transfer in this mint = paid" watcher lights up green)
        /// must not satisfy a real invoice. Named separately from
        /// `underpayment_does_not_match` to document this as a specific,
        /// deliberately-covered threat, not just an incidental side effect
        /// of the amount check.
        #[test]
        fn a_dust_transfer_does_not_satisfy_a_real_invoice() {
            // 1 raw base unit (0.000001 USDC) against a 25 USDC invoice.
            let observed = [spl_transfer(1)];
            assert_eq!(match_payment(&args_for("25", Some(USDC)), &observed).unwrap(), None);
        }

        #[test]
        fn spl_payment_does_not_satisfy_a_native_sol_request() {
            let observed = [spl_transfer(25_000_000)];
            // Expecting native SOL (mint None) -- an SPL transfer must not match.
            assert_eq!(match_payment(&args_for("25", None), &observed).unwrap(), None);
        }

        /// Cross-invoice collision defense: right mint, right (or more
        /// than) amount, but the transaction doesn't actually carry the
        /// reference this specific invoice requested -- e.g. a second,
        /// unrelated customer's payment to the same recipient in the same
        /// mint, happening to be concurrently open. Must not match, even
        /// though mint+amount alone would satisfy it.
        #[test]
        fn a_transfer_missing_the_requested_reference_does_not_match() {
            let mut args = args_for("25", Some(USDC));
            args.reference = Some(REFERENCE.to_string());
            let mut observed = spl_transfer(25_000_000);
            observed.has_reference = false;
            assert_eq!(match_payment(&args, &[observed]).unwrap(), None);
        }

        #[test]
        fn a_transfer_carrying_the_requested_reference_does_match() {
            let mut args = args_for("25", Some(USDC));
            args.reference = Some(REFERENCE.to_string());
            let observed = spl_transfer(25_000_000); // has_reference: true by default
            assert_eq!(match_payment(&args, &[observed]).unwrap(), Some(0));
        }

        #[test]
        fn has_reference_is_ignored_when_no_reference_was_requested() {
            // args.reference is None -- has_reference on the observed
            // transfer shouldn't matter at all.
            let mut observed = spl_transfer(25_000_000);
            observed.has_reference = false;
            assert_eq!(match_payment(&args_for("25", Some(USDC)), &[observed]).unwrap(), Some(0));
        }

        // ---- the fused risk screening -------------------------------------

        #[test]
        fn confirm_screens_a_clean_mint_green() {
            let observed = spl_transfer(25_000_000);
            let facts = MintFacts { top_holder_share_pct: 5.0, ..Default::default() };
            let out = confirm(&args_for("25", Some(USDC)), &observed, &facts, None);
            assert_eq!(out.status, "paid");
            assert_eq!(out.risk_level.as_deref(), Some("green"));
            assert!(out.summary.contains("confirmed"));
            assert!(out.brl_estimate.is_none());
        }

        // ---- Trust Report ---------------------------------------------------

        #[test]
        fn a_paid_result_always_carries_a_fully_verified_trust_report() {
            let observed = spl_transfer(25_000_000);
            let facts = MintFacts { top_holder_share_pct: 5.0, ..Default::default() };
            let out = confirm(&args_for("25", Some(USDC)), &observed, &facts, None);
            let report = out.trust_report;
            assert!(report.recipient_verified);
            assert_eq!(report.amount_status, AmountStatus::Match);
            assert!(report.mint_verified);
            assert!(report.tx_confirmed);
            // No reference was requested in this invoice -- not applicable.
            assert_eq!(report.reference_verified, None);
        }

        #[test]
        fn trust_report_reference_verified_true_when_a_reference_was_requested() {
            let mut args = args_for("25", Some(USDC));
            args.reference = Some(REFERENCE.to_string());
            let observed = spl_transfer(25_000_000); // has_reference: true
            let facts = MintFacts { top_holder_share_pct: 5.0, ..Default::default() };
            let out = confirm(&args, &observed, &facts, None);
            assert_eq!(out.trust_report.reference_verified, Some(true));
        }

        #[test]
        fn pending_with_nothing_observed_reports_everything_false() {
            let out = pending(&args_for("25", Some(USDC)), &[]);
            let report = out.trust_report;
            assert!(!report.recipient_verified);
            assert_eq!(report.amount_status, AmountStatus::None);
            assert!(!report.mint_verified);
            assert_eq!(report.reference_verified, None);
            assert!(!report.tx_confirmed);
        }

        #[test]
        fn pending_with_an_underpayment_reports_amount_under_but_everything_else_true() {
            let mut args = args_for("25", Some(USDC));
            args.reference = Some(REFERENCE.to_string());
            // A transfer for the right mint+reference landed, but short of
            // the requested 25 -- this is why match_payment didn't match it
            // (and so pending() is what ran at all).
            let observed = [spl_transfer(10_000_000)];
            let out = pending(&args, &observed);
            let report = out.trust_report;
            assert!(report.recipient_verified);
            assert_eq!(report.amount_status, AmountStatus::Under);
            assert!(report.mint_verified);
            assert_eq!(report.reference_verified, Some(true));
            assert!(!report.tx_confirmed);
        }

        #[test]
        fn pending_with_an_unrelated_transfer_reports_recipient_true_but_mint_false() {
            // Something arrived at this recipient, but in a different mint
            // -- e.g. an unrelated payment. Must not be mistaken for
            // progress on this invoice.
            let mut other_mint_transfer = spl_transfer(25_000_000);
            other_mint_transfer.mint = Some("So11111111111111111111111111111111111111112".to_string());
            let out = pending(&args_for("25", Some(USDC)), &[other_mint_transfer]);
            let report = out.trust_report;
            assert!(report.recipient_verified);
            assert_eq!(report.amount_status, AmountStatus::None);
            assert!(!report.mint_verified);
            assert_eq!(report.reference_verified, None);
        }

        /// The safety-critical case: a payment lands, but in a token with a
        /// permanent delegate (a scam-shaped mint). `confirm` must still run
        /// `assess` and surface RED -- it cannot report a clean "confirmed".
        /// This is the fusion doing its job.
        #[test]
        fn confirm_flags_a_paid_but_dangerous_mint_red() {
            let observed = spl_transfer(25_000_000);
            let facts = MintFacts { has_permanent_delegate: true, ..Default::default() };
            let out = confirm(&args_for("25", Some(USDC)), &observed, &facts, None);
            assert_eq!(out.status, "paid");
            assert_eq!(out.risk_level.as_deref(), Some("red"));
            assert!(out.summary.contains("FLAGGED"));
            assert!(!out.summary.to_lowercase().contains("confirmed: "));
        }

        #[test]
        fn confirm_includes_brl_estimate_when_rate_configured() {
            let observed = spl_transfer(25_000_000);
            let facts = MintFacts { top_holder_share_pct: 5.0, ..Default::default() };
            let out = confirm(&args_for("25", Some(USDC)), &observed, &facts, Some(5.60));
            assert_eq!(out.brl_estimate.as_deref(), Some("R$140.00"));
        }

        #[test]
        fn reply_includes_an_est_line_when_brl_estimate_is_present() {
            let observed = spl_transfer(25_000_000);
            let facts = MintFacts { top_holder_share_pct: 5.0, ..Default::default() };
            let out = confirm(&args_for("25", Some(USDC)), &observed, &facts, Some(5.60));
            assert!(out.reply.contains("Est.: R$140.00"));
        }

        #[test]
        fn reply_has_no_est_line_when_brl_estimate_is_absent() {
            let observed = spl_transfer(25_000_000);
            let facts = MintFacts { top_holder_share_pct: 5.0, ..Default::default() };
            let out = confirm(&args_for("25", Some(USDC)), &observed, &facts, None);
            assert!(!out.reply.contains("Est.:"));
        }

        #[test]
        fn pending_never_carries_a_brl_estimate() {
            // Nothing has landed yet -- there is no confirmed amount to
            // convert, regardless of whether a rate is configured.
            let out = pending(&args_for("25", Some(USDC)), &[]);
            assert!(out.brl_estimate.is_none());
        }

        /// Prompt-injection / abuse: nothing in the args (a crafted amount,
        /// or a mint the caller doesn't actually hold) can conjure a match
        /// out of transfers that didn't happen. With no observed transfers,
        /// the result is always "pending", never "paid".
        #[test]
        fn cannot_be_talked_into_confirming_an_unmatched_payment() {
            let observed: [ObservedTransfer; 0] = [];
            let matched = match_payment(&args_for("25", Some(USDC)), &observed).unwrap();
            assert_eq!(matched, None);
            let out = pending(&args_for("25", Some(USDC)), &observed);
            assert_eq!(out.status, "pending");
            assert!(out.risk_level.is_none());
        }

        // ---- format_reply ---------------------------------------------------

        #[test]
        fn reply_green_case_exact_text() {
            let observed = spl_transfer(25_000_000);
            let facts = MintFacts { top_holder_share_pct: 5.0, ..Default::default() };
            let mut args = args_for("25", Some(USDC));
            args.reference = Some(REFERENCE.to_string());
            let out = confirm(&args, &observed, &facts, None);
            let expected = "Payment Verification\n\
                 ✓ Amount matches\n\
                 ✓ Recipient verified\n\
                 ✓ Reference matches\n\
                 ✓ Transaction confirmed\n\
                 🟢 Token risk: GREEN — no red flags found in mint/freeze authority, holder concentration, or Token-2022 extensions\n\
                 Verdict: PAYMENT VERIFIED — safe to trust.";
            assert_eq!(out.reply, expected);
        }

        #[test]
        fn reply_red_case_exact_text() {
            let observed = spl_transfer(25_000_000);
            let facts = MintFacts {
                has_permanent_delegate: true,
                freeze_authority_active: true,
                ..Default::default()
            };
            let mut args = args_for("25", Some(USDC));
            args.reference = Some(REFERENCE.to_string());
            let out = confirm(&args, &observed, &facts, None);
            let expected = "Payment Verification\n\
                 ✓ Amount matches\n\
                 ✓ Recipient verified\n\
                 ✓ Reference matches\n\
                 ✓ Transaction confirmed\n\
                 🔴 Token risk: RED — a permanent delegate can move holder funds without consent; freeze authority is still active\n\
                 Verdict: DO NOT TRUST THIS PAYMENT.";
            assert_eq!(out.reply, expected);
        }

        /// The RED verdict must be just as prominent as GREEN -- same
        /// number of lines, same checklist structure, not a quieter or
        /// truncated version. This asserts that shape directly, on top of
        /// the exact-text assertions above.
        #[test]
        fn red_reply_is_not_a_downgraded_green_reply() {
            let observed = spl_transfer(25_000_000);
            let green_facts = MintFacts { top_holder_share_pct: 5.0, ..Default::default() };
            let red_facts = MintFacts { has_permanent_delegate: true, ..Default::default() };
            let args = args_for("25", Some(USDC));
            let green = confirm(&args, &observed, &green_facts, None);
            let red = confirm(&args, &observed, &red_facts, None);
            assert_eq!(green.reply.lines().count(), red.reply.lines().count());
            assert!(red.reply.starts_with("Payment Verification\n"));
            assert!(red.reply.contains("Verdict: "));
        }

        #[test]
        fn reply_amber_case_exact_text() {
            let observed = spl_transfer(25_000_000);
            let facts = MintFacts { mint_authority_active: true, ..Default::default() };
            let args = args_for("25", Some(USDC));
            let out = confirm(&args, &observed, &facts, None);
            let expected = "Payment Verification\n\
                 ✓ Amount matches\n\
                 ✓ Recipient verified\n\
                 ✓ Transaction confirmed\n\
                 🟠 Token risk: AMBER — mint authority is still active, supply can be inflated\n\
                 Verdict: PAYMENT LANDED but flagged AMBER — review before trusting.";
            assert_eq!(out.reply, expected);
        }

        #[test]
        fn reply_not_paid_case_nothing_observed_exact_text() {
            let out = pending(&args_for("25", Some(USDC)), &[]);
            let expected = "Payment Verification\n\
                 ✗ Amount matches\n\
                 ✗ Recipient verified\n\
                 ✗ Transaction confirmed\n\
                 Verdict: NOT CONFIRMED — do not treat this as paid.";
            assert_eq!(out.reply, expected);
        }

        #[test]
        fn reply_not_paid_case_underpayment_exact_text() {
            let mut args = args_for("25", Some(USDC));
            args.reference = Some(REFERENCE.to_string());
            let observed = [spl_transfer(10_000_000)];
            let out = pending(&args, &observed);
            let expected = "Payment Verification\n\
                 ✗ Amount matches (underpaid)\n\
                 ✓ Recipient verified\n\
                 ✓ Reference matches\n\
                 ✗ Transaction confirmed\n\
                 Verdict: NOT CONFIRMED — do not treat this as paid.";
            assert_eq!(out.reply, expected);
        }

        /// No invented confidence score or numeric anywhere in any reply --
        /// only the real checklist marks and the real green/amber/red word.
        #[test]
        fn reply_never_contains_a_percentage_or_score() {
            let observed = spl_transfer(25_000_000);
            let facts = MintFacts { top_holder_share_pct: 5.0, ..Default::default() };
            let paid = confirm(&args_for("25", Some(USDC)), &observed, &facts, None);
            let not_paid = pending(&args_for("25", Some(USDC)), &[]);
            for reply in [&paid.reply, &not_paid.reply] {
                assert!(!reply.contains('%'));
                assert!(!reply.to_lowercase().contains("confidence"));
                assert!(!reply.to_lowercase().contains("score"));
            }
        }
    }
}

// --- wasm component shim -----------------------------------------------
// Thin wrapper only: validate addresses, read rpc_url from the jailed
// __config, make the read-only RPC calls (getSignaturesForAddress ->
// getTransaction, plus getAccountInfo/getTokenLargestAccounts for the
// paying mint), hand everything to `core`, shape the result. The risk
// screening itself is core::confirm's job, not this file's.
#[cfg(target_family = "wasm")]
mod component {
    wit_bindgen::generate!({
        path: "../../wit/v0",
        world: "tool-plugin",
        features: ["plugins-wit-v0"],
    });

    use std::collections::HashMap;
    use std::time::Duration;

    use serde_json::{json, Value};

    use crate::core::{self, Args, ObservedTransfer};
    use exports::zeroclaw::plugin::plugin_info::Guest as PluginInfo;
    use exports::zeroclaw::plugin::tool::{Guest as Tool, ToolResult};
    use zeroclaw::plugin::logging::{
        log_record, LogLevel, PluginAction, PluginEvent, PluginOutcome,
    };
    use zeroclaw_solana_core::rpc::{
        account_data_from_result, max_token_account_amount, parse_response_value, RpcRequest,
    };
    use zeroclaw_solana_core::{holder_share_pct, parse_mint_account, MintFacts, Pubkey};

    struct PaymentWatch;

    const PLUGIN_NAME: &str = "payment-watch";
    const PLUGIN_VERSION: &str = env!("CARGO_PKG_VERSION");
    const TOOL_NAME: &str = "payment-watch";
    /// Cap on candidate transactions inspected per call -- bounds both RPC
    /// work and context size.
    const MAX_SIGNATURES: usize = 10;
    /// The wrapped-SOL mint address, used as the price-lookup id for a
    /// native-SOL payment (Jupiter's price API is SPL-mint-keyed; this is
    /// the standard convention Solana tooling uses to price native SOL
    /// through an SPL-token-shaped price API).
    const WSOL_MINT: &str = "So11111111111111111111111111111111111111112";

    #[derive(serde::Deserialize)]
    struct ExecuteArgs {
        recipient: String,
        amount: String,
        #[serde(default)]
        mint: Option<String>,
        #[serde(default)]
        reference: Option<String>,
        #[serde(rename = "__config", default)]
        config: HashMap<String, String>,
    }

    impl PluginInfo for PaymentWatch {
        fn plugin_name() -> String {
            PLUGIN_NAME.to_string()
        }

        fn plugin_version() -> String {
            PLUGIN_VERSION.to_string()
        }
    }

    impl Tool for PaymentWatch {
        fn name() -> String {
            TOOL_NAME.to_string()
        }

        fn description() -> String {
            "Checks whether an expected Solana payment has landed: watches a \
             recipient address for a given amount (native SOL or an SPL token \
             mint), correlated by a Solana Pay reference. When a matching \
             transfer is found, it automatically screens the paying token for \
             scam risk before confirming. Read-only, never moves funds. Call \
             it on a schedule (e.g. an SOP cron) to poll until paid. Send the \
             result's `reply` field to the channel VERBATIM as your entire \
             response -- it is already an exact checklist (amount/recipient/ \
             reference/transaction, then the real risk verdict on \"paid\") \
             built from the same fields `trust_report` exposes \
             (`recipient_verified`, `amount_status`, `mint_verified`, \
             `reference_verified`, `tx_confirmed`) plus `risk_level`. Do not \
             paraphrase, reformat, summarize `reply`, or add your own \
             confidence score or numeric estimate -- there isn't one, only \
             the real green/amber/red verdict."
                .to_string()
        }

        fn parameters_schema() -> String {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "recipient": {"type": "string", "description": "Base58 wallet address expected to receive the payment"},
                    "amount": {"type": "string", "description": "Expected amount as a positive decimal string, e.g. \"25\" or \"25.50\""},
                    "mint": {"type": "string", "description": "Base58 SPL token mint expected. Omit for native SOL."},
                    "reference": {"type": "string", "description": "Base58 Solana Pay reference to correlate the payment by. Strongly recommended; required for reliable SPL detection."}
                },
                "required": ["recipient", "amount"]
            })
            .to_string()
        }

        fn execute(args: String) -> Result<ToolResult, String> {
            let parsed: ExecuteArgs = match serde_json::from_str(&args) {
                Ok(a) => a,
                Err(e) => {
                    emit(PluginAction::Fail, PluginOutcome::Failure, "invalid arguments");
                    return Ok(fail(format!("invalid arguments: {e}")));
                }
            };

            // Validate every address-shaped field before spending an RPC
            // call. Same fail-closed property as the other two plugins:
            // instruction-like text in any of these is rejected here, as it
            // can't survive strict base58/length parsing.
            for (label, value) in [
                ("recipient", Some(&parsed.recipient)),
                ("mint", parsed.mint.as_ref()),
                ("reference", parsed.reference.as_ref()),
            ] {
                if let Some(v) = value {
                    if let Err(e) = Pubkey::parse(v) {
                        emit(PluginAction::Fail, PluginOutcome::Failure, "invalid address arg");
                        return Ok(fail(format!("invalid {label}: {e}")));
                    }
                }
            }

            let rpc_url = match parsed.config.get("rpc_url").filter(|v| !v.is_empty()) {
                Some(u) => u.clone(),
                None => {
                    emit(PluginAction::Fail, PluginOutcome::Failure, "no rpc_url configured");
                    return Ok(fail(
                        "payment-watch requires `rpc_url` in this plugin's config section \
                         (see README) -- no RPC endpoint is hardcoded."
                            .to_string(),
                    ));
                }
            };

            let core_args = Args {
                recipient: parsed.recipient.clone(),
                amount: parsed.amount.clone(),
                mint: parsed.mint.clone(),
                reference: parsed.reference.clone(),
            };

            // A configured `brl_rate` is both the opt-in signal for this
            // whole feature and the fallback figure -- absent/empty/
            // unparseable all mean "no BRL estimate," ever a hard
            // failure. `run_check` resolves live-vs-static once it knows
            // which mint actually paid (a "pending" result never reaches
            // that point, so never attempts a live fetch for nothing).
            let static_rate = parsed
                .config
                .get("brl_rate")
                .filter(|v| !v.is_empty())
                .and_then(|v| v.parse::<f64>().ok());

            let output = match run_check(&rpc_url, &core_args, static_rate) {
                Ok(o) => o,
                Err(e) => {
                    emit(PluginAction::Fail, PluginOutcome::Failure, "rpc/check failed");
                    return Ok(fail(e));
                }
            };

            let action = if output.status == "paid" {
                PluginAction::Complete
            } else {
                PluginAction::Note
            };
            emit(action, PluginOutcome::Success, &output.status);

            match serde_json::to_string(&output) {
                Ok(j) => Ok(ToolResult { success: true, output: j, error: None }),
                Err(e) => Err(format!("failed to encode result: {e}")),
            }
        }
    }

    /// The orchestration the shim owns: fetch candidate signatures, pull each
    /// transaction, build the observed-transfer list via core, match, and --
    /// on a match -- fetch the paying mint's facts and hand off to
    /// `core::confirm` (which runs the risk screening). No matching or risk
    /// logic lives here.
    fn run_check(rpc_url: &str, args: &Args, static_rate: Option<f64>) -> Result<core::Output, String> {
        // Correlate by the reference when given (Solana Pay attaches it as a
        // key precisely so the payment is findable this way); otherwise fall
        // back to the recipient's own address (works for native SOL).
        let query_addr = args.reference.as_deref().unwrap_or(&args.recipient);
        let sigs_result = rpc_call(
            rpc_url,
            "getSignaturesForAddress",
            json!([query_addr, {"limit": MAX_SIGNATURES}]),
        )?;
        let signatures = core::signatures_from_response(&sigs_result);

        let mut observed: Vec<ObservedTransfer> = Vec::new();
        for sig in signatures.iter().take(MAX_SIGNATURES) {
            let tx = rpc_call(
                rpc_url,
                "getTransaction",
                json!([sig, {"encoding": "jsonParsed", "maxSupportedTransactionVersion": 0}]),
            )?;
            observed.extend(core::transfers_from_tx_meta(&tx, &args.recipient, args.reference.as_deref()));
        }

        let matched = core::match_payment(args, &observed).map_err(|e| e.to_string())?;
        match matched {
            Some(idx) => {
                let transfer = &observed[idx];
                let facts = match &transfer.mint {
                    // SPL: screen the actual paying mint.
                    Some(mint) => fetch_mint_facts(rpc_url, mint)?,
                    // Native SOL has no mint -- default facts score green.
                    None => MintFacts::default(),
                };
                // Live pricing needs the actual paying mint, which is only
                // known now, at a match -- a "pending" result below never
                // reaches here, so never attempts a live fetch for nothing.
                let brl_rate = static_rate.map(|fallback| {
                    let asset_mint = transfer.mint.as_deref().unwrap_or(WSOL_MINT);
                    resolve_brl_rate(asset_mint, fallback)
                });
                Ok(core::confirm(args, transfer, &facts, brl_rate))
            }
            None => Ok(core::pending(args, &observed)),
        }
    }

    /// Fetch a mint's on-chain facts (same two calls token-risk-check makes)
    /// and assemble them through the shared `MintFacts::from_parsed`, so both
    /// plugins screen an identical mint identically.
    fn fetch_mint_facts(rpc_url: &str, mint: &str) -> Result<MintFacts, String> {
        let account_result =
            rpc_call(rpc_url, "getAccountInfo", json!([mint, {"encoding": "base64"}]))?;
        let data = account_data_from_result(&account_result).map_err(|e| e.to_string())?;
        let parsed_mint = parse_mint_account(&data).map_err(|e| e.to_string())?;

        let largest_result = rpc_call(rpc_url, "getTokenLargestAccounts", json!([mint]))?;
        let largest_amount = max_token_account_amount(&largest_result).map_err(|e| e.to_string())?;

        Ok(MintFacts::from_parsed(
            &parsed_mint,
            holder_share_pct(largest_amount, parsed_mint.supply),
        ))
    }

    /// One JSON-RPC round trip over the host's `wasi:http` (via blocking
    /// `waki`). Request building and response parsing go through
    /// `zeroclaw_solana_core::rpc`, so only the network call itself is here.
    fn rpc_call(rpc_url: &str, method: &str, params: Value) -> Result<Value, String> {
        let req = RpcRequest::new(method, params);
        let body =
            serde_json::to_value(&req).map_err(|e| format!("failed to encode rpc request: {e}"))?;
        let resp = waki::Client::new()
            .post(rpc_url)
            .json(&body)
            .connect_timeout(Duration::from_secs(10))
            .send()
            .map_err(|e| format!("rpc request failed: {e}"))?;
        let resp_body: Value = resp.json().map_err(|e| format!("invalid rpc response: {e}"))?;
        parse_response_value(resp_body).map_err(|e| e.to_string())
    }

    fn fail(message: String) -> ToolResult {
        ToolResult { success: false, output: String::new(), error: Some(message) }
    }

    fn emit(action: PluginAction, outcome: PluginOutcome, message: &str) {
        log_record(
            LogLevel::Info,
            &PluginEvent {
                function_name: "payment_watch::tool::execute".to_string(),
                action,
                outcome: Some(outcome),
                duration_ms: None,
                attrs: None,
                message: message.to_string(),
            },
        );
    }

    /// BRL-per-unit for `asset_mint`: live (Jupiter USD price x Frankfurter
    /// USD->BRL rate) when both succeed, `fallback` (the operator's
    /// configured static `brl_rate`) on any failure -- a rate-limited or
    /// unreachable price API degrades this display-only figure to the
    /// operator's own number, never to an error that blocks the actual
    /// payment confirmation.
    fn resolve_brl_rate(asset_mint: &str, fallback: f64) -> f64 {
        match fetch_live_brl_rate(asset_mint) {
            Some(rate) => {
                emit(PluginAction::Note, PluginOutcome::Success, &format!("live brl rate used: {rate}"));
                rate
            }
            None => {
                emit(PluginAction::Note, PluginOutcome::Failure, "live brl rate unavailable, using static fallback");
                fallback
            }
        }
    }

    fn fetch_live_brl_rate(asset_mint: &str) -> Option<f64> {
        let usd_price = match fetch_jupiter_usd_price(asset_mint) {
            Ok(p) => p,
            Err(e) => {
                emit(PluginAction::Note, PluginOutcome::Failure, &format!("jupiter price fetch failed: {e}"));
                return None;
            }
        };
        let usd_to_brl = match fetch_usd_to_brl_rate() {
            Ok(r) => r,
            Err(e) => {
                emit(PluginAction::Note, PluginOutcome::Failure, &format!("frankfurter fx fetch failed: {e}"));
                return None;
            }
        };
        let rate = usd_price * usd_to_brl;
        rate.is_finite().then_some(rate)
    }

    /// Free, no-API-key endpoint (confirmed live: returns a real USD price
    /// for a known mint, and an empty object -- not an error -- for a mint
    /// with no market). Rate-limited fairly tight without a registered
    /// key, which is exactly why this is a best-effort upgrade, not a
    /// requirement.
    fn fetch_jupiter_usd_price(mint: &str) -> Result<f64, String> {
        let url = format!("https://api.jup.ag/price/v3?ids={mint}");
        let resp = waki::Client::new()
            .get(&url)
            .connect_timeout(Duration::from_secs(5))
            .send()
            .map_err(|e| format!("request failed: {e}"))?;
        let body: Value = resp.json().map_err(|e| format!("invalid json: {e}"))?;
        body.get(mint)
            .and_then(|v| v.get("usdPrice"))
            .and_then(|v| v.as_f64())
            .ok_or_else(|| format!("no usdPrice for {mint} in response: {body}"))
    }

    /// Free, no-API-key, ECB-sourced daily rate, confirmed live. Note the
    /// `.dev` domain: `frankfurter.app` 301-redirects here permanently,
    /// and `waki` does not follow redirects, so the old `.app` URL parsed
    /// as invalid JSON (the redirect body, not real data) -- this is the
    /// one that actually works.
    fn fetch_usd_to_brl_rate() -> Result<f64, String> {
        let resp = waki::Client::new()
            .get("https://api.frankfurter.dev/v1/latest?from=USD&to=BRL")
            .connect_timeout(Duration::from_secs(5))
            .send()
            .map_err(|e| format!("request failed: {e}"))?;
        let body: Value = resp.json().map_err(|e| format!("invalid json: {e}"))?;
        body.get("rates")
            .and_then(|v| v.get("BRL"))
            .and_then(|v| v.as_f64())
            .ok_or_else(|| format!("no BRL rate in response: {body}"))
    }

    export!(PaymentWatch);
}
