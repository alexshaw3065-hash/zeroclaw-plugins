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
| `solana-core` | Shared toolbox (RPC, base58, mint/Token-2022 parsing). Not a plugin — no wasm dependency; vendored into each plugin that needs it (see `tools/sync_solana_core.py`). | n/a | built |
| `plugins/token-risk-check` | Given a mint address, returns a red/amber/green scam risk verdict with reasons. | **T0** — read only | built: real RPC fetch + WIT shim wired, `cargo test` and `cargo build --target wasm32-wasip2 --release` both clean |
| `plugins/solana-pay-request` | Turns "charge 25 USDC" into a Solana Pay QR code. | **T1** — builds a request, never signs | built: core + WIT shim wired, `cargo test` and `cargo build --target wasm32-wasip2 --release` both clean |
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

## Building this repo

There is deliberately no root `Cargo.toml` — every plugin here (including
these three) is a fully standalone crate, matching the rest of this repo.
Run commands per-crate:

```bash
rustup target add wasm32-wasip2   # once

(cd solana-core && cargo test)                                       # canonical core's own tests
(cd plugins/<name> && cargo test --locked)                           # that plugin's host tests, no network
(cd plugins/<name> && cargo build --locked --target wasm32-wasip2 --release)  # the real component

python3 tools/sync_solana_core.py check   # verify no vendored solana-core copy has drifted
```

`solana-core/` at the repo root is the single canonical source; each
plugin that needs it carries its own literal copy under
`plugins/<name>/solana-core/`, because the real CI
(`tools/ci/validate_components.sh`) builds every plugin from an isolated
snapshot of just that plugin's own folder plus `wit/v0` — a path
dependency reaching outside a plugin's directory would not resolve
there. Run `python3 tools/sync_solana_core.py sync` after any edit under
`solana-core/src/` and commit the result.

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
