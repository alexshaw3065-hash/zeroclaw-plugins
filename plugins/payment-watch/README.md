# payment-watch (build order: step 3 -- build this last)

STATUS: not yet built. This is the most technically demanding of the
three plugins -- build it only once token-risk-check and
solana-pay-request are solid.

## Custody tier: T0 (read only)

Watches an address for an expected amount + reference, fires an event
when it lands. Never signs, never moves funds.

## The fusion (this is the piece that makes Idea 1 more than three
separate tools)

Before reporting a payment as confirmed, this plugin calls
`zeroclaw_solana_core::risk::assess` directly -- the exact same function
`token-risk-check` uses -- on the mint that was actually paid. This is a
plain internal function call, not a request to the LLM to "remember" to
double check, so the screening cannot be skipped or talked out of.

## TODO

- [ ] Pure core: match an observed transfer against {address, expected
      amount, reference}
- [ ] Pure core: call `risk::assess` on the paying mint before returning
      "confirmed"
- [ ] Host tests with a mocked `getSignaturesForAddress` /
      `getTransaction` response -- no live network
- [ ] Shape output to a short sentence, not raw JSON (see bounty trap #3)
- [ ] Prompt-injection test: a malicious memo/reference cannot force a
      "confirmed" result for an unmatched or unsafe payment
- [ ] Wire the wasm shim + SOP trigger once real WIT bindings are available
