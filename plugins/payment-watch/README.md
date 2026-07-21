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
`getTokenLargestAccounts`. Never builds, signs, or submits a transaction.
Secrets held: an RPC endpoint URL via `config_read`, nothing else.

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
text through an address field, or (c) get a payment in a scam token waved
through as safe.

**Why it fails closed:**
- `recipient`, `mint`, and `reference` are accepted only as strict 32-byte
  base58 values (`Pubkey::parse`); instruction-like text is rejected before
  any RPC call.
- A `"paid"` result is derived purely from on-chain balance deltas
  (`match_payment` over transfers built from real transaction meta). No
  argument, memo, or reference value can conjure a match for a transfer
  that did not happen — with nothing matching, the result is always
  `"pending"`.
- The risk screening is unconditional and lives in core (`confirm` →
  `assess`), so a paid-but-dangerous mint is surfaced as `FLAGGED RED`, not
  quietly confirmed.

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

Response once it lands in a clean token:
```json
{
  "status": "paid",
  "signature": "5xY…9kR",
  "amount": "25",
  "mint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
  "risk_level": "green",
  "risk_reasons": ["no red flags found in mint/freeze authority, holder concentration, or Token-2022 extensions"],
  "summary": "Payment confirmed: 25 of EPjF…Dt1v received (sig 5xY…9kR). Paying token risk: GREEN."
}
```

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
- [ ] Verified against live chain data — this environment has no network
      access (Solana RPC hosts are blocked by egress policy), so the RPC
      response shapes are exercised only against mocked fixtures. Verify
      against a real Solana Pay payment (native SOL and an SPL token, with
      a reference) before production use.
- [ ] Durable-nonce / blockhash-expiry handling is **not applicable here**
      (this plugin builds no transactions — it only observes); it becomes
      relevant only if a future T1 builder is added.

## What we'd build next

Close the loop with `solana-pay-request`: an SOP that takes "charge table 4
for 25 USDC", calls `solana-pay-request` to post a QR, then schedules
`payment-watch` on the returned `reference` until it flips to `paid` —
already risk-screened — and replies in the chat.
