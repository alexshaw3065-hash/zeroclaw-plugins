# Smart Payment Terminal — ZeroClaw Solana plugins

A self-hosted AI payment terminal for Solana. Charge a customer, get a QR
code in the chat, and the moment payment lands, automatically screen the
coin it was paid in for scam risk before ever saying "confirmed."

**Bounty:** Build Solana-native plugins for ZeroClaw (Superteam Brasil)
**Tracks:** A (Payments) as the primary track, built on a Track E shared
core, with a Track D safety plugin fused directly into the payment flow.

## What's in here

One repo, one pull request, four pieces:

| Piece | What it does | Custody tier | Status |
|---|---|---|---|
| `solana-core` | Shared toolbox (RPC, base58, JSON handling). Not a plugin — no wasm dependency, just a library the plugins import. | n/a | scaffolded |
| `plugins/token-risk-check` | Given a mint address, returns a red/amber/green scam risk verdict with reasons. | **T0** — read only | scaffolded, core logic + tests written |
| `plugins/solana-pay-request` | Turns "charge 25 USDC" into a Solana Pay QR code. | **T1** — builds a request, never signs | not yet built |
| `plugins/payment-watch` | Watches for the payment to land, then calls `token-risk-check`'s logic internally before confirming. | **T0** — read only | not yet built |

Nothing in this repo ever holds a signing key. T0 = only looks. T1 = only
prepares something for a human to sign in their own wallet.

## Build order (do it in this sequence — each stopping point is a
complete, submittable entry on its own)

1. **`solana-core` + `token-risk-check`** — build and harden these first.
   Simplest of the three plugins, and it's the one the bounty doc says
   they want "most of all."
2. **`solana-pay-request`** — mostly formatting a URL correctly, low risk.
3. **`payment-watch`** — the most technically demanding piece (watching
   the chain over time, matching amounts, calling into the risk-check
   logic). Build this last, once the first two are solid.

## Working in Claude Code

This scaffold was written without live access to the ZeroClaw repo (no
internet in the environment that generated it), so a few things need to
happen first when you open this in Claude Code, where you do have real
internet access:

1. `git clone https://github.com/zeroclaw-labs/zeroclaw-plugins` next to
   this folder and diff `plugins/redact-text` against our layout — line up
   folder structure, `Cargo.toml` shape, and logging calls exactly.
2. Read the real `.wit` files under `wit/v0` in the ZeroClaw repo and
   replace the placeholder shim functions in each plugin's `src/lib.rs`
   with the real generated bindings — the comments marked `TODO` show
   where.
3. Install the wasm target: `rustup target add wasm32-wasip2`
4. From the repo root: `cargo test` (this runs the host-side tests against
   `solana-core` and every plugin's `core` module — no wasm toolchain
   needed for this step, by design).
5. Once the shim is wired to real bindings: `cargo build --target
   wasm32-wasip2 --release`

## Required per plugin before submission (from the bounty's hard
requirements — track these per plugin folder)

- [ ] Layout matches `plugins/redact-text`
- [ ] Pure core / thin shim split (already structured this way)
- [ ] Host-run tests, no live network in tests (already structured this way)
- [ ] Builds clean for `wasm32-wasip2`
- [ ] Structured logging via the logging import, never stdout
- [ ] `manifest.toml` with minimal permissions
- [ ] `README.md`: what it does, config keys, custody tier + why, threat
      model, one worked example
- [ ] A prompt-injection test with the transcript in the README
- [ ] MIT License

## The Brazil touch (nice-to-have, add once the core flow works)

`solana-pay-request` and `payment-watch` should show a BRL-equivalent
amount alongside the crypto amount in invoices and confirmations —
e.g. "R$140 (≈25 USDC)" — since this bounty is sponsored by Superteam
Brasil. No real PIX/bank integration needed, just the display detail.
