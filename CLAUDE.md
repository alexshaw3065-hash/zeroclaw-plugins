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
3. ~~Wire real WIT bindings.~~ Done for `token-risk-check`
   (`wit_bindgen::generate!` against `../../wit/v0`, matching
   `redact-text`'s exact pattern — the shared top-level `wit/v0/` is
   fine to reference directly, since CI's isolated snapshot copies it
   alongside every plugin). `solana-pay-request` and `payment-watch`
   still need their shims written when their turn in the build order
   comes.
4. ~~Look at `plugins/telegram` for an `http_client` example.~~ Done —
   `token-risk-check`'s RPC calls use the same `waki::Client::new().post(url).json(&body).send()` /
   `.json::<Value>()` pattern telegram, discord, and every other
   HTTP-calling plugin here use.
5. ~~Install `wasm32-wasip2`.~~ Done.

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
