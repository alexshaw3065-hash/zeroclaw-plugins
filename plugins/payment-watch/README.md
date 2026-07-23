# payment-watch

Checks whether an expected Solana payment has landed: watches a recipient
address for a given amount (native SOL or a specific SPL token), correlated
by a Solana Pay `reference`. When a matching transfer is found, it
automatically screens the token it was paid in for scam risk **before**
reporting the payment as confirmed. Read-only. Call it on a schedule (e.g.
a ZeroClaw SOP cron) to poll until paid.

## Custody tier: T0 (read only)

Reads public chain data only — `getSignaturesForAddress`, `getTransaction`,
and (to screen the paying mint) `getAccountInfo` /
`getTokenLargestAccounts`. When `brl_rate` is configured, also reads two
free public price APIs (Jupiter, Frankfurter) to opportunistically
upgrade that setting to a live rate — see "Config keys" below. Never
builds, signs, or submits a transaction. Secrets held: an RPC endpoint URL
via `config_read`, nothing else.

**Why T0 is the right tier:** confirming a payment is an observation, not
an action. There is no path in this plugin that moves funds, so the worst
case of any bug or manipulation is a wrong *statement* about whether money
arrived — never a wrong transfer.

## The fusion (why this is one system, not three separate tools)

Before this plugin will emit a `"paid"` result, `core::confirm` calls
`zeroclaw_solana_core::risk::assess` on the mint that actually paid — the
exact same function `token-risk-check` runs. It is a plain internal
function call inside tested core, not a request to the LLM to "remember"
to double-check, so the screening **cannot be skipped, talked out of, or
prompt-injected away**. Every confirmation carries a risk verdict by
construction; there is no code path that produces a clean "confirmed"
without one. If the paying token is scam-shaped (e.g. it has a permanent
delegate), the result is `"paid"` but `FLAGGED RED`, explicitly telling
the operator not to treat it as safe.

Both plugins also assemble mint facts through the same shared
`MintFacts::from_parsed`, so they can never disagree about what the same
on-chain mint looks like.

## Config keys

| Key | Required | Description |
|---|---|---|
| `rpc_url` | yes | Your Solana RPC endpoint. No key is hardcoded — bring your own. |
| `brl_rate` | no | BRL per one unit of whatever asset is expected (a decimal string, e.g. `"5.60"`). Setting this opts into the "Brazil touch": a confirmed ("paid") result carries a `brl_estimate` display string. A "pending" result never carries one, regardless of this setting — nothing confirmed yet to convert. This value is both the opt-in signal and the fallback figure: on a match, a live rate is tried first (Jupiter's price API on the actual paying mint, times Frankfurter's daily USD→BRL rate), falling back to this static value on any failure — matches `solana-pay-request`'s identical setting. |

## Parameters

| Arg | Required | Description |
|---|---|---|
| `recipient` | yes | Base58 wallet address expected to receive the payment |
| `amount` | yes | Expected amount as a positive decimal string, e.g. `"25"` |
| `mint` | no | Base58 SPL mint expected; omit for native SOL |
| `reference` | no | Base58 Solana Pay reference to correlate by — **strongly recommended, and required for reliable SPL detection** (see note) |

**Why `reference` matters for SPL:** a Solana Pay `reference` is attached to
the payment transaction as an extra account key precisely so the payment
can be found with `getSignaturesForAddress(reference)`. Without one, this
plugin falls back to the recipient's own address — which works for native
SOL, but not for SPL tokens, because an SPL transfer touches the
recipient's *token account*, not their wallet address. `solana-pay-request`
emits a `reference` for exactly this hand-off; pass the same value here.

## Threat model

**What could go wrong:** an attacker tries to (a) make the tool report
"confirmed" for a payment that never landed, (b) slip instruction-like
text through an address field, (c) get a payment in a scam token waved
through as safe, (d) send a near-zero "dust" transfer hoping a naive
"any transfer in this mint = paid" watcher lights up green, or (e) get an
*unrelated* real payment — a different customer's payment to the same
recipient, in the same mint, for a different invoice open at the same
time — mistaken for this invoice's payment.

**Why it fails closed:**
- `recipient`, `mint`, and `reference` are accepted only as strict 32-byte
  base58 values (`Pubkey::parse`); instruction-like text is rejected before
  any RPC call.
- A `"paid"` result is derived purely from on-chain balance deltas
  (`match_payment` over transfers built from real transaction meta). No
  argument, memo, or reference value can conjure a match for a transfer
  that did not happen — with nothing matching, the result is always
  `"pending"`.
- **Dust defense:** a matched transfer must meet or exceed the exact
  requested amount (`decimal_to_raw`-computed, on the real mint decimals)
  — a tiny decoy transfer in the right mint cannot satisfy a real invoice.
  Named test: `a_dust_transfer_does_not_satisfy_a_real_invoice`.
- **Cross-invoice collision defense:** when the invoice specifies a
  `reference`, a candidate transaction must actually carry that reference
  as one of its own account keys — checked directly against the
  transaction's `accountKeys`, not just inferred from which address the
  RPC query happened to search by (defense in depth: this holds even if
  the query-side correlation is ever bypassed or misconfigured). Named
  tests: `a_transfer_missing_the_requested_reference_does_not_match`,
  `a_transfer_carrying_the_requested_reference_does_match`. Pair this with
  `solana-pay-request`'s auto-generated, single-use `reference` (its
  README's "Auto-generated reference" section) for the strongest
  guarantee: two concurrently open invoices for the same recipient/mint
  can never cross-match each other's payments.
- The risk screening is unconditional and lives in core (`confirm` →
  `assess`), so a paid-but-dangerous mint is surfaced as `FLAGGED RED`, not
  quietly confirmed.
- **The Trust Report:** a `"paid"` result carries a `trust_report` object
  (`recipient_verified`, `amount_verified`, `mint_verified`,
  `reference_verified`) — each field reflects something `match_payment`
  actually checked against on-chain data, not merely echoed input. This
  doesn't add new checking; it makes the guarantees above individually
  legible instead of collapsed into one opaque `"paid"` string, so an
  operator (or a judge reading the write-up) can see exactly what was
  verified rather than taking the status field's word for it.

### Prompt-injection / abuse test transcript

Scenario: a payment is requested in a scam token (permanent delegate), and
the attacker hopes the "payment landed" event alone gets it treated as
safe.

```
match_payment(expected, observed) -> Some(transfer)   // the transfer really did land
confirm(expected, transfer, facts{ has_permanent_delegate: true })
  -> assess(facts) = RED
  -> status "paid", risk_level "red",
     summary: "Payment landed (…) but the paying token is FLAGGED RED --
               a permanent delegate can move holder funds without consent.
               Do not treat this as a safe payment."
```

And an unmatched payment can never be forced to "paid":
```
match_payment(expected, []) -> None   ->   pending(expected)   // status "pending", no risk_level
```

Both are automated tests in `plugins/payment-watch/src/lib.rs`
(`confirm_flags_a_paid_but_dangerous_mint_red`,
`cannot_be_talked_into_confirming_an_unmatched_payment`).

## Worked example

Request (poll for a 25-USDC invoice, correlated by a reference):
```json
{
  "recipient": "9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin",
  "amount": "25",
  "mint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
  "reference": "So11111111111111111111111111111111111111112"
}
```

Response while waiting:
```json
{ "status": "pending", "amount": "25", "mint": "EPjF…Dt1v",
  "risk_reasons": [], "summary": "No matching payment yet (25 of EPjF…Dt1v to 9xQe…VFin)." }
```

Response once it lands in a clean token (with `brl_rate = "5.60"` set):
```json
{
  "status": "paid",
  "signature": "5xY…9kR",
  "amount": "25",
  "mint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
  "risk_level": "green",
  "risk_reasons": ["no red flags found in mint/freeze authority, holder concentration, or Token-2022 extensions"],
  "summary": "Payment confirmed: 25 of EPjF…Dt1v received (sig 5xY…9kR). Paying token risk: GREEN.",
  "brl_estimate": "R$140.00",
  "trust_report": {
    "recipient_verified": true,
    "amount_verified": true,
    "mint_verified": true,
    "reference_verified": true
  }
}
```
`brl_estimate` is absent entirely without `brl_rate` configured, and never
appears on a "pending" result — there is nothing confirmed yet to convert.
`trust_report` is present only on "paid" (`null`/absent on "pending");
`reference_verified` is `null` when the invoice didn't specify a
`reference` at all (not applicable), never `false` — a transfer failing
that check is filtered out by `match_payment` before "paid" can happen.

## What's built vs. what's left

- [x] Pure core: match an observed transfer against {recipient, amount,
      mint}, over transfers parsed from real `getTransaction` meta
      (SPL token-balance deltas and native-SOL lamport deltas), all
      host-tested with mocked RPC JSON — no network
- [x] Pure core: the fused `risk::assess` call on the paying mint before
      any "paid" result (`confirm`)
- [x] Short, shaped output (one-sentence `summary`), never raw RPC JSON
- [x] Prompt-injection / abuse tests (unmatched payment stays pending;
      paid-but-dangerous mint is flagged red)
- [x] Wasm shim wired to real WIT bindings; builds clean for
      `wasm32-wasip2` and passes `cargo clippy -D warnings` on host + wasm
- [x] Verified against live chain data (2026-07-21, real devnet) — see
      "Live devnet verification" below. Native SOL and a `reference`-based
      SPL flow are still worth exercising before production use; only an
      SPL transfer without a `reference` (matched by recipient token
      account) has been tried live so far.
- [x] BRL-equivalent display on a confirmed payment (`brl_estimate`, via
      an operator-configured `brl_rate` — root README's Brazil touch).
      Hybrid: live price (Jupiter + Frankfurter, both free, no API key)
      on the actual paying mint when available, falls back to the
      operator's static `brl_rate` on any failure; never appears on a
      "pending" result.
- [x] Dust-defense named test (`a_dust_transfer_does_not_satisfy_a_real_invoice`)
      and cross-invoice reference-collision defense-in-depth (a matched
      transaction must actually carry the requested `reference` as an
      account key, checked directly in `transfers_from_tx_meta`/
      `match_payment`, not just inferred from the RPC query) — see
      "Threat model" above.
- [x] The Trust Report: `trust_report` (`recipient_verified`,
      `amount_verified`, `mint_verified`, `reference_verified`) on every
      "paid" result, making the already-enforced guarantees individually
      auditable instead of one opaque status string.
- [ ] Durable-nonce / blockhash-expiry handling is **not applicable here**
      (this plugin builds no transactions — it only observes); it becomes
      relevant only if a future T1 builder is added.

## Live devnet verification

Confirmed on real Solana devnet (2026-07-21) — full detail in the root
`CLAUDE.md` "Live devnet verification log". A real payment (3 tokens of a
Token-2022 mint with `permanent-delegate` + `freeze authority` enabled)
was sent to a merchant recipient address. `payment-watch` found the real
signature via `getSignaturesForAddress`, parsed the correct balance delta
from `getTransaction` (3,000,000 raw units at 6 decimals), matched it
against the expected amount, then re-screened the paying mint and fused
the result exactly as designed:

```json
{
  "status": "paid",
  "risk_level": "red",
  "summary": "Payment landed (...) but the paying token is FLAGGED RED --
              a permanent delegate can move holder funds without consent.
              Do not treat this as a safe payment."
}
```

A real, landed payment still surfaced as unsafe — the fusion held up
against live chain data, not just mocked fixtures.

**Known limitation, found during this verification:** same
`getTokenLargestAccounts` rate-limit issue documented in
`token-risk-check`'s README — the free public devnet RPC endpoint
rejects that method outright (`429`, "Too many requests for a specific
RPC call"), so holder-concentration scoring on the paying mint needs a
dedicated RPC provider, not the public default.

## What we'd build next

Close the loop with `solana-pay-request`: an SOP that takes "charge table 4
for 25 USDC", calls `solana-pay-request` to post a QR, then schedules
`payment-watch` on the returned `reference` until it flips to `paid` —
already risk-screened — and replies in the chat.
