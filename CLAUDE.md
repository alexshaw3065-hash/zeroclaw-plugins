# Project context for Claude Code

Read this fully before doing anything. This file is the persistent memory
for this project — treat conflicting assumptions from earlier in a
session as wrong if they contradict this file.

## What this is

A submission for the "Build Solana-native plugins for Zeroclaw" bounty
(Superteam Brasil, on Superteam Earn). Winner announcement: **August 21,
2026**. One pull request to `zeroclaw-labs/zeroclaw-plugins`, containing
one shared core crate and three plugins ("Idea 1: The Smart Payment
Terminal" — see root README.md for the full pitch).

**The developer working with you is learning Rust and Solana at the same
time.** Prefer clear, well-commented code over clever code. Explain
non-obvious Rust patterns (lifetimes, trait bounds, `cfg` gating) briefly
in comments where they first appear. Don't assume prior familiarity with
Solana-specific concepts (PDAs, versioned transactions, durable nonces)
without a one-line reminder of what they are.

## The absolute rules — never violate these

1. **Custody tier ceiling: T1 maximum. Never build T2.** T0 = read only.
   T1 = builds an unsigned transaction or a Solana Pay URL, a human signs.
   Nothing in this repo may ever hold a signing key or submit a
   transaction. This was a deliberate, discussed decision — do not
   "helpfully" add signing capability even if it looks like a small step.
2. **Pure-core / thin-shim split, no exceptions.** All real logic lives
   in a plain Rust module with zero wasm dependencies, testable with
   plain `cargo test`. The `#[cfg(target_family = "wasm")]` shim only
   parses args, calls core, shapes the result, and makes the one allowed
   HTTP call. If you're about to put actual logic inside a `shim` module,
   stop and move it to `core` instead.
3. **No live network calls in tests, ever.** Mock RPC responses as
   literal JSON strings in test code. `cargo test` must pass with zero
   internet access.
4. **No secrets or hardcoded RPC URLs in code.** RPC endpoint comes from
   `config_read` at runtime, always user-overridable. Never commit a real
   API key anywhere, including in example config.
5. **Never print to stdout.** Use the structured logging import
   (`log-record`) once it's wired up — until then, don't add
   `println!`/`dbg!` debugging that could get left in.
6. **One component = one tool.** Never merge two plugins into one with an
   action enum, even if it seems more efficient. `token-risk-check`,
   `solana-pay-request`, and `payment-watch` stay three separate crates.

## Build order — follow this sequence, don't jump ahead

1. `solana-core` + `plugins/token-risk-check` — build and harden first.
2. `plugins/solana-pay-request` — next.
3. `plugins/payment-watch` — last, and only once 1 and 2 are solid. It's
   the hardest (chain polling, matching, and it calls into
   `zeroclaw_solana_core::risk::assess` internally — see below).

Each stopping point is a complete, submittable entry on its own. If time
runs short, stop at the end of a step, not mid-step.

## The one non-obvious design decision to preserve

`payment-watch` must call `zeroclaw_solana_core::risk::assess` **directly
as a function call**, on the mint that paid, before it reports a payment
as confirmed. This is not the LLM "remembering" to call
`token-risk-check` separately — it's hardcoded so it can't be skipped.
Both plugins import the exact same `assess()` function from
`solana-core`, so they can never disagree about what counts as safe. Keep
this fused; don't refactor it into two independent, unconnected plugins.

## Repo layout

```
solana-core/            canonical pure Rust toolbox, no wasm deps -- see "Vendoring" below
  src/pubkey.rs          base58 address parsing
  src/rpc.rs             JSON-RPC request building + response parsing (transport-agnostic)
  src/risk.rs            the shared scam-risk heuristic (assess())
  src/token.rs           SPL Token / Token-2022 mint account parsing
plugins/
  token-risk-check/      T0 — built furthest along, use as the pattern for the other two
  solana-pay-request/    T1 — stubbed, TODO in README.md
  payment-watch/         T0 — stubbed, TODO in README.md, must call risk::assess()
tools/sync_solana_core.py  propagates solana-core/ into each plugin's vendored copy
```

Each plugin folder needs, before it's submittable (see its own README.md
for the per-plugin checklist): layout matching `plugins/redact-text`,
`manifest.toml` with minimal permissions, a README with custody tier +
threat model + worked example + prompt-injection transcript, and an MIT
LICENSE file.

### Vendoring solana-core — no root workspace, no cross-plugin path deps

There is deliberately **no root `Cargo.toml`** in this repo (matching
every other plugin here: each is a fully standalone crate with its own
`[workspace]` marker). The real CI (`tools/ci/validate_components.sh`)
builds every plugin from an **isolated snapshot containing only that
plugin's own directory plus `wit/v0`** — nothing else. A path dependency
reaching outside a plugin's folder (e.g. `../../solana-core`) would not
resolve there, so `solana-core` cannot be a normal shared dependency.

Instead: `solana-core/` at the repo root is the single canonical source
you edit. Each plugin that needs it (`token-risk-check`, `payment-watch`,
and `solana-pay-request` for address validation) carries its own literal
copy at `plugins/<name>/solana-core/`, produced and verified by
`tools/sync_solana_core.py`. **Whenever you change anything under
`solana-core/src/`, run `python3 tools/sync_solana_core.py sync`
afterward and commit the result** — the three copies are not symlinks,
they are real files, and `check` (below) will fail if they drift.

## Known gaps

Resolved, now that this session has real repo access (the earlier version
of this file listed these as open — keep this section as a log, not a
todo list, so a future session doesn't redo the discovery):

1. ~~Clone the real `zeroclaw-plugins` repo.~~ Done — this checkout *is*
   the real fork (`github.com/alexshaw3065-hash/zeroclaw-plugins`,
   forked from `zeroclaw-labs/zeroclaw-plugins`). A separate checkout of
   the *other* (wrong, main `zeroclaw` daemon monorepo) repo this
   scaffold was mistakenly built against the first time lives alongside
   this one as read-only reference; never commit or push there.
2. ~~Diff against `plugins/redact-text`.~~ Done — `wasm_path` and
   `capabilities` in all three manifests were wrong (`capabilities`
   must be `["tool"]`, not `["execute"]`, which isn't a valid
   `PluginCapability`; `wasm_path` is a filename resolved relative to
   the plugin directory at install time, not a `target/...` build path).
   Fixed.
3. ~~Wire real WIT bindings.~~ Done for all three plugins
   (`wit_bindgen::generate!` against `../../wit/v0`, matching
   `redact-text`'s exact pattern — the shared top-level `wit/v0/` is
   fine to reference directly, since CI's isolated snapshot copies it
   alongside every plugin). `token-risk-check`, `solana-pay-request`,
   and `payment-watch` all build for `wasm32-wasip2` and pass
   `cargo clippy -D warnings` on host + wasm. Live-chain verification
   is still pending — this session's egress policy blocks Solana RPC
   hosts, so RPC response shapes are exercised against mocked fixtures
   only (noted in each plugin's README).
4. ~~Look at `plugins/telegram` for an `http_client` example.~~ Done —
   `token-risk-check`'s RPC calls use the same `waki::Client::new().post(url).json(&body).send()` /
   `.json::<Value>()` pattern telegram, discord, and every other
   HTTP-calling plugin here use.
5. ~~Install `wasm32-wasip2`.~~ Done.
6. ~~Verify against a live devnet mint/payment.~~ Done — see "Live devnet
   verification log" below. Both plugins' core logic now confirmed
   against real chain data, not just mocked fixtures.

## Live devnet verification log

Run on 2026-07-21, against real Solana devnet (`api.devnet.solana.com`),
using a throwaway devnet-only keypair funded by the user (not committed
anywhere, not part of this repo). Verification was done by a small
standalone Rust harness (outside this repo, in scratch) that calls the
plugins' actual `core` modules directly — the same code the wasm
component ships — wired to a plain native HTTP client instead of `waki`
(which only exists in the wasm target). Read-only RPC calls only; no
signing or transaction-submission code was added anywhere in this repo.
All on-chain setup (keypair, airdrop, mint creation, the test transfer)
was done with the standard `solana`/`spl-token` CLIs as external tooling,
never through plugin code — the custody-tier rule (no signing key, no tx
submission in this repo) was never touched.

**Mints created:**
- Clean mint `9e8Bacw455vQjjQqUbwJaL3J4SpRjDCaJd7MPcLHZphQ` — legacy SPL
  Token, mint authority revoked, no freeze authority, zero supply.
- Risky mint `AGqbQefyKdWWeUrc68ReJucwAyJQpgUXBGQaUikc9V8r` — Token-2022,
  created with `--enable-freeze --enable-permanent-delegate`.

**token-risk-check results:**
- Clean mint → `green`, "no red flags found ...".
- Risky mint → `red`, reasons: "a permanent delegate can move holder
  funds without consent" + "freeze authority is still active".
- This also verified the Token-2022 TLV byte-offset parsing in
  `solana-core/src/token.rs` against a real live mint — that file's
  comment flagged this as unverified before today; it's now confirmed
  correct.

**payment-watch results:**
- Sent a real payment: 3 tokens of the risky mint, from the test wallet
  to merchant recipient `96n4Dj5cn4PYQrEDTc1Zzjt4uY4GQ5Vshfy9VXVDHVQD`.
  Signature: `3f96U45ge3vz9hEh3Dfn13u2zmpm9AkcfScXaAxeNsU3q7adF32UYFVJGfchX3axueBawL729hRpo3aV5bZNKnA9`.
- `payment-watch` found the signature via `getSignaturesForAddress`,
  parsed the correct balance delta from `getTransaction`
  (3,000,000 raw units at 6 decimals), matched it against the expected
  amount "3", re-screened the paying mint, and correctly fused the
  result: `status: "paid"`, `risk_level: "red"`, summary explicitly
  warning "Do not treat this as a safe payment." A landed payment in a
  dangerous token still surfaced as unsafe — the fusion held up against
  live chain data, not just mocked test fixtures.
- Native SOL and a `reference`-correlated SPL flow were not exercised
  live this session (only a `reference`-less SPL transfer, matched by
  recipient token account) — worth doing before final submission.

**Known limitation found:** `getTokenLargestAccounts` is rejected outright
by the free public devnet RPC endpoint — `{"code": 429, "message": "Too
many requests for a specific RPC call"}` on every attempt, confirmed via
direct `curl` independent of any client library, not a per-IP burst
limit. Two other free public endpoints tried (`rpc.ankr.com`,
`devnet.helius-rpc.com`) both require an API key. Both plugins therefore
degrade to 0% holder-share (not a hard failure) when this call is
unavailable; the mint/freeze-authority and Token-2022-extension checks
(the ones that actually decide red vs. green here) don't depend on it.
For real holder-concentration coverage, `rpc_url` needs to point at a
dedicated provider (Helius/QuickNode/Triton/etc.), not the public
default — noted in both plugins' READMEs.

## Commands

Run per-crate (there is no root workspace — see "Vendoring" above):

- `(cd solana-core && cargo test)` — the canonical core's own tests.
- `(cd plugins/<name> && cargo test --locked)` — that plugin's host
  tests. Must pass with no network access. Run after every change to a
  `core` module or to `solana-core/` (after re-running `sync`).
- `(cd plugins/<name> && cargo build --locked --target wasm32-wasip2 --release)`
  — the real component build.
- `python3 tools/sync_solana_core.py check` — verify no vendored
  `solana-core` copy has drifted from the canonical one; `sync` to fix.

## Traps called out by the bounty sponsors — keep these in mind while coding

- **Blockhash expiry**: an unsigned tx sitting in an approval queue can
  go stale before a human signs it. Relevant once `payment-watch` /
  `solana-pay-request` build real transactions — durable nonce accounts
  are the fix, not yet implemented.
- **solana-sdk / solana-client don't work well for wasm32-wasip2.** Stay
  on `waki` + `serde_json` + `bs58` + hand-rolled encoding, not the
  official Solana Rust SDK.
- **Don't flood the context window.** Every plugin response must be
  short, shaped text (aim ~200 tokens), never raw RPC JSON dumps.
- **RPC key/URL only via config, never hardcoded.**

## Where the fuller story lives

- Root `README.md` — the full pitch, track mapping, and per-plugin status
  table.
- Each `plugins/*/README.md` — that plugin's custody tier writeup,
  threat model, and TODOs. Keep these updated as you build; they get
  submitted as-is.
