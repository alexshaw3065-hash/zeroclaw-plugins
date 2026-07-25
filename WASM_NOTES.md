# The named traps, and what we actually did about them

The bounty brief names several specific, recurring traps. Here's what
this repo actually did about each one — not a restatement of the trap,
proof of the fix.

## Blockhash expiry (trap #1)

`spl-transfer-build` supports durable nonce accounts — the real fix,
not a workaround. All three gotchas the brief calls out are handled:
- `AdvanceNonceAccount` is enforced as the first instruction.
- Rent was *measured* live rather than estimated: 1,447,680 lamports
  for an 80-byte nonce account, ≈0.00145 SOL.
- One nonce account covers exactly one in-flight transaction — the
  README documents that parallel approvals need one nonce account each.

## The wasm dependency wall (trap #2)

Confirmed by build, not by claim: the modular `solana-pubkey` /
`solana-instruction` / `solana-message` / `solana-transaction` /
`solana-hash` crates all compile clean to `wasm32-wasip2`, and were used
directly — `sns-resolve` for real `curve25519` PDA math, and
`spl-transfer-build` for real transaction construction — not
hand-rolled byte encoding.

The component-boundary risk the brief flags as real did actually
surface: `wasm-tools component wit`, run against the actual compiled
components (not assumed from the library-level compile check alone),
confirmed one addition beyond the baseline WASI import surface —
`wasi:random/insecure-seed`, from the `curve25519-dalek` dependency
chain. That addition was independently confirmed satisfied by
ZeroClaw's own host wiring by reading the actual host source
(`wasmtime_wasi::p2::add_to_linker_async` is called unconditionally for
every plugin), not assumed from general WASI knowledge.

## Context flooding (trap #3)

Every plugin returns a short, deterministic, pre-formatted reply — never
a raw RPC dump. `payment-watch`'s reply is a fixed checklist plus a
one-line verdict, and one of its own tests asserts that no reply
anywhere contains a made-up confidence score or percentage — the
temptation an LLM-composed reply would have, structurally
impossible here since the text is built in plain Rust before the model
ever sees it.

## Fail-closed action certification

Named directly in the bounty's own "custody design space" list as one
of the frontier patterns worth building. It's in `spl-transfer-build`:
after constructing a transaction, the plugin re-parses the *exact
serialized wire bytes* it's about to return and independently
re-verifies every field — fee payer, nonce, ATA creation, transfer
amount/decimals/destination, memo — against the original request, on
every real call, not just in tests. Proven, not just asserted: three
tests deliberately hand-corrupt an already-correct transaction (a
mismatched amount, a swapped destination, a dropped memo) and confirm
certification rejects each one.

## Squads v4 ("agent proposes, multisig disposes")

Investigated before building anything, not skipped out of ignorance:
the real program ID, PDA seeds, Anchor discriminator scheme, and
`SmallVec` encoding were all confirmed against actual upstream source.
Verdict: moderate complexity, and the hardest part is exactly what the
brief warns about — hand-encoding Anchor-style instructions from a wasm
component. Deliberately not pursued this bounty: the existing use
case's approval path (a human reviewing a QR code or an unsigned
transaction directly) already satisfies the same "agent proposes, human
disposes" principle with meaningfully less encoding risk, and depth on
one real use case beats breadth across two half-finished ones.
