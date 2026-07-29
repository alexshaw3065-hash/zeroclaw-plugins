# Custody & threat model

## The ladder

| Tier | Meaning | Used here? |
|---|---|---|
| T0 | Reads public chain data, returns an observation | `payment-watch`, `token-risk-check`, `sns-resolve` |
| T1 | Builds an unsigned transaction or payment URL; a human signs | `solana-pay-request`, `spl-transfer-build` |
| T2 | Signs and submits | **Never. Not built. Not planned.** |

Nothing in this repo ever holds a signing key. **T1 is the ceiling** —
by design, not by omission.

## Correct layering, argued honestly, not assumed

`token-risk-check`'s Token-2022 TLV parsing and `spl-transfer-build`'s
hand-built unsigned transactions are genuinely bounded computation that
belongs in compiled code — hand-rolling either correctly in a prompt
would be unreasonable. `solana-pay-request`'s *URL-building* half is
honestly closer to something a Tier-1 skill could do with zero compiled
code — a Solana Pay `solana:` URI is plain string formatting. Building
it as a plugin here was a consistency choice (one shared `solana-core`
toolbox, one pure-core/thin-shim pattern across every plugin), not a
claim that it was necessary in isolation. What *does* need compiled
code, and is the actual reason `solana-pay-request` and `payment-watch`
exist as plugins rather than skills, is the fused, unconditional risk
check: `payment-watch` calls `token-risk-check`'s exact risk-assessment
function directly, inside tested Rust, before it will ever report a
payment as confirmed — an LLM-driven skill cannot *guarantee* a second
check always runs; a direct Rust function call can, and does.

## Threat model — proven, not asserted

Every plugin's own README has its full threat model in detail. The
short version:
[`PROMPT_INJECTION_TRANSCRIPT.md`](./PROMPT_INJECTION_TRANSCRIPT.md) —
seven real malicious inputs (an "ignore previous instructions" memo
trying to redirect funds, a crafted amount trying to blow past a
config-enforced spend cap, a swap request trying to smuggle in a
different destination address, and more), each with the actual test
code and why the attack has no field to travel through. All seven were
re-run live to confirm they still pass before this document was
written, not just remembered as passing once.

## Third-party services this depends on, declared plainly

- **Helius RPC endpoint** — reads chain data. Never holds a key.
- **Jupiter's public Swap API** — only ever returns an unsigned
  transaction, never signs or holds funds. Used by
  `solana-pay-request`'s swap-prepare path only.
- **Groq's Whisper endpoint** — voice transcription. Interaction layer
  only, never touches a transaction.
- **Frankfurter's FX API** — a display-only BRL rate. Cosmetic; falls
  back to a static operator-set value on any failure.

None of these can move funds or hold a key on this repo's behalf — if
any of them is wrong, slow, or unreachable, the failure mode is a bad
read or a missing display figure, never an unauthorized transfer. No
MCP servers or facilitators are used anywhere in this submission.
