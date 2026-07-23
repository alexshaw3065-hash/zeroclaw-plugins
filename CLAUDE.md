# Project context for Claude Code

Read this fully before doing anything. This file is the persistent memory
for this project — treat conflicting assumptions from earlier in a
session as wrong if they contradict this file.

## RULE CHANGE — CONFIRMED

The bounty rules changed. Submission is now a showcase post in
**#solana-bounty on Discord**: a ≤3min video + a written report + a
linked repo. **Not a pull request.** Do not open a registry PR against
`zeroclaw-labs/zeroclaw-plugins` during the bounty — explicit sponsor
instruction, not a preference.

**Strategy, locked in:**
- The existing use case (payment terminal, fused safety check) already
  matches the sponsors' own "winning showcase" example almost exactly.
  No new use case needed.
- The originality/revolutionary angle IS the fused design: `payment-watch`
  refuses to report a payment as trustworthy without an unconditional
  risk check, proven live against real devnet data. Lead the showcase
  write-up and video with this, not with a plugin list.
- Tier defense, stated honestly in the write-up: `token-risk-check` is a
  clear Tier 3 case (Token-2022 TLV parsing, matches sponsors' own
  example). `payment-watch` is defensible via its fused logic being real
  bounded computation, not a thin wrapper. `solana-pay-request` is
  honestly closer to something a Tier 1 Skill could do — say so
  explicitly rather than pretend otherwise; owning this is itself good
  craft. **Important nuance (found 2026-07-23 reading the actual brief,
  see below): the sponsors' own reference "winning showcase" is itself
  built at Tier 1 — stock binary, one payment *skill*, one cron SOP. Our
  entire payment-request/QR piece is heavier (a compiled plugin) than
  their own reference architecture for the identical use case. Own this
  explicitly in the write-up; don't imply our approach was necessary
  when their own example proves it wasn't, for that piece.**
- Frontier ideas from the new doc (agent-hires-agent/escrow, sub-agents,
  x402 marketplaces, on-chain skill distribution, policy wallets,
  transaction firewalls) are noted and deliberately **not pursued** —
  wrong risk/time tradeoff against a proven, nearly-finished submission.
  This is a considered decision, not an oversight — do not revisit
  without a major runway change.
  **x402 feasibility check (2026-07-23), for if that changes:** confirmed
  against the real daemon source (`C:\Users\User\Desktop\plugin\zeroclaw`,
  `wit/v0/tool.wit`, `docs/book/src/plugins/`), not guessed. Tool plugins
  cannot receive inbound requests at all — `world tool-plugin` imports
  only `logging`, no `inbound`, ever; they're invoked exclusively by the
  agent's own LLM. Channel plugins do import `inbound` and are the only
  capability with an inbound design at all, but per
  `docs/book/src/plugins/index.md`'s wiring-status table and
  `writing-a-channel-plugin.md` verbatim: "a channel plugin loads and
  passes its contract tests but is **not yet constructed by a running
  daemon**" — orchestrator registration and the per-vendor host listener
  are the missing seam. So a plugin (tool or channel) cannot serve real
  inbound traffic today, full stop — not a config issue, a genuine
  platform gap, same category as the cron/SOP-autonomy finding already
  logged above. The native `[channels.webhook]` channel (not a plugin)
  *does* run a real embedded HTTP server today, but its shape is wrong
  for x402 regardless: inbound POST must be `{sender, content,
  thread_id}` (a chat message, not an API call), it returns `200 OK` on
  ingestion before the agent does anything, and any reply goes out
  *later* as a separate outbound POST to a configured `send_url` —
  there's no path to return the tool's actual JSON output in the same
  HTTP response. x402 needs a synchronous round trip (`402` + payment
  requirements, then the real content on retry, same request/response).
  **If x402 is ever revisited:** the correct shape is a small standalone
  HTTP server *outside* ZeroClaw's plugin/channel model entirely, calling
  `solana-core::risk::assess` directly (already pure, host-tested, zero
  wasm dependency) — not a fourth plugin, and not the webhook channel.

## The official bounty brief — key facts (read 2026-07-23)

Source: the actual Superteam Earn listing text, pasted in full by the
user. This is the primary source; treat anything else in this file that
conflicts with it as secondary. Full original text is in the
conversation this was captured from, not reproduced verbatim here —
what follows is the load-bearing extract.

**Rewards:** 🥇 1,800 USDG · 🥈 1,200 USDG · 🥉 1,000 USDG · 4×
Honorable mention, 250 USDG each. Total pool 5,000 USDG.

**Judging weights** (know these — they say what to spend polish time
on): Use case 30% · Safety & custody design 25% · Craft 20% ·
Reproducibility 15% · Showcase 10%. Tiebreak: build-in-public logs on X
during the bounty (**not currently being done — consider starting if
there's runway for it**).

**Submission mechanics, precise:**
- A *working* use case: real agent, real channel, real job, actually
  running (not a concept). Link to the GitHub repo.
- A showcase post in `#solana-bounty` on Discord: video (≤3 min, real
  agent + real channel, no slides — "terminal + phone is perfect"),
  write-up (what it does, who it's for, which ZeroClaw features it
  uses, what was built, custody tier + threat model, links to
  config/SOPs/skills/code, secrets redacted), plus any supporting
  material.
- Standalone plugin PRs are explicitly **not** accepted as submissions
  on their own. Registry merges happen *separately*, after judging —
  maintainers invite the strongest implementation per plugin family.
  Confirms (with more precision) the RULE CHANGE above.
- Reproducibility is scored directly: "another operator replicates your
  setup from your write-up in an evening" is the literal bar.

**The sponsors' own reference "winning showcase" (read this closely —
it's the actual bar, and it's Tier 1, not Tier 3):**
> Someone DMs the shop's **WhatsApp** (not Telegram): "charge table 4,
> 25 USDC." Agent replies with a QR. Customer wallet pays it. **Forty
> seconds later** the agent posts "Invoice #412 paid ✓" in the owner's
> channel **on its own**. Write-up: *stock release binary*, webhook +
> WhatsApp channel, **one payment skill** (Solana Pay URL construction +
> response shaping — not a compiled plugin), **one cron SOP** polling
> `getSignaturesForAddress`, one approval checkpoint for refunds. T1, no
> keys held.

Two things this directly changes about how we should think about our
own submission:
1. Their reference build is entirely **Tier 1** for the payment-terminal
   piece — a skill plus a cron SOP, zero compiled code. Ours does the
   same job with a compiled `solana-pay-request` plugin. Not wrong (the
   brief explicitly scores "correct layering," and over-building isn't
   disqualifying) but it means our tier defense for that specific plugin
   has to be an honest "this could have been a skill" admission, not a
   claim of necessity — already captured above, now with the primary
   source to cite directly in the write-up.
2. **Sharper point:** their own reference architecture is *also* "one
   cron SOP" driving a tool call to post an unprompted result 40 seconds
   later. That is exactly the mechanism we spent real effort verifying
   is currently broken in ZeroClaw (`THE ROADMAP` step 1, re-confirmed
   2026-07-23: a cron-triggered SOP whose step calls a tool cannot
   self-execute without a live agent-loop turn already in progress —
   "no agent loop available to execute," reproduced cleanly across 6
   real cron ticks with zero chat interaction). If that's a genuine
   platform-wide limitation and not something specific to our setup, the
   sponsors' own literal reference example would hit the identical wall
   today. This is worth stating plainly and precisely in the write-up —
   not as an excuse, as a real, evidenced finding about the platform
   itself, which is exactly the kind of thing "Craft" (20%) and honest
   "Safety & custody" framing (25%) are supposed to reward. Don't
   overclaim this (we have not tested their exact skill+cron+webhook
   architecture, only our own plugin+cron architecture) — but the
   underlying SOP-execution mechanism we found broken is shared, not
   plugin-specific, per our own earlier finding
   ("ZeroClaw daemon integration findings" below).

**Tier 3 guidance on wasm32-wasip2 — the exact source text for
`THE ROADMAP` step 4's rewrite (quote this closely, don't paraphrase
from stale memory):**
> The modular `solana-pubkey` / `solana-instruction` / `solana-message` /
> `solana-transaction` / `solana-hash` crates, plus `borsh` and `bs58`,
> all compile clean to `wasm32-wasip2` on the stock toolchain — no
> hand-rolled byte encoding needed to build and serialize transactions.
> Even `solana-sdk` itself compiles for wasip2 now; prefer the modular
> crates for a minimal component. Two caveats: this is compile-verified
> as a library, not yet exercised as an instantiated component inside
> the ZeroClaw host, whose WASI capability grants are narrower — budget
> for surprises at the component boundary, and write down what you hit.
> The browser-targeted crates (`wasm_client_solana`,
> `solana-client-wasm`) still won't work (JavaScript glue). Transport is
> unchanged either way: RPC goes over `waki` (blocking `wasi:http`) +
> `serde_json`, not `solana-client`.

This means the old framing in our own README/CLAUDE.md ("stay on `waki`
+ `serde_json` + `bs58` + hand-rolled encoding, not the official Solana
Rust SDK," in "Traps called out by the bounty sponsors" below) is
**half-stale**: `waki`/`serde_json` for transport still holds, but "no
official SDK at all" is now outdated guidance — the *modular* crates
work fine as libraries. We never actually needed them (none of our
three plugins build a raw transaction), so nothing to change in the
plugins themselves, but the write-up's wasm32-wasip2 paragraph (step 4)
needs to describe this accurately, not repeat the old blanket avoidance
framing as if it still fully holds.

**New/refined traps not already captured below:**
- `wit/v0` is explicitly experimental, no `.frozen` marker — the ABI can
  move; pin assumptions, expect a rebuild. (Already implicitly true for
  us; now explicit.)
- Pyth Core deprecates **2026-07-31** (mid-bounty) — unauthenticated
  Hermes endpoints stop serving. Not currently relevant (we use Jupiter
  + Frankfurter for BRL, not Pyth) — but if price-feed work ever touches
  Pyth, get an API key first or fall back to Switchboard's Crossbar.
- **Design for polling, not webhooks**: a chat-resident cron agent has
  no guaranteed public inbound ingress — validates `payment-watch`'s
  existing poll-based design. Where hand-building a transaction is the
  hard part, the brief suggests routing through a **Blink** instead
  (Actions/Jupiter/Drift Gateway/Kamino hand back a ready-to-sign base64
  transaction over plain REST) — worth considering for `THE ROADMAP`
  step 9's `spl-transfer-build` stretch goal as a lower-effort
  alternative to hand-rolling the transaction ourselves, if a suitable
  Action/Blink provider exists for a plain SPL transfer.

**"We will not accept" — self-check against this list before submitting:**
concepts/mockups (must run — we're clear, everything's live-tested);
"a plugin with no use case around it" (we're clear — one coherent
payment-terminal use case, not three disconnected components); "thin
single-RPC-call wrappers padded into WASM" (worth re-reading with a
critical eye against `solana-pay-request` specifically — see the tier
defense note above); anything holding a raw private key with no
caps/allowlist/approval gate (we're clear — T0/T1 only, no plugin in
this repo ever holds a key); trading/sniper/"buy this token" bots (not
applicable).

**Resource noted, unvetted:** a bounty commenter linked a third-party
crate, `solana-client-wasip2` (crates.io), claiming wasm32-wasip2
Solana-client support. Not evaluated or trusted yet — if `THE ROADMAP`
step 9 (`spl-transfer-build`) is reached, vet this before depending on
it; a random comment-linked crate is not itself a source of truth about
safety or correctness.

## What this is

A submission for the "Build Solana-native plugins for Zeroclaw" bounty
(Superteam Brasil, on Superteam Earn). Winner announcement: **August 21,
2026**. One shared core crate and three plugins ("Idea 1: The Smart
Payment Terminal" — see root README.md for the full pitch), submitted as
a Discord showcase post per the RULE CHANGE above, not a pull request.

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
`devnet.helius-rpc.com`) both require an API key. **Correction (found
2026-07-22 via a real manual call through the actual ZeroClaw daemon,
see below): the 0%-holder-share graceful degradation only exists in the
external verification harness, not in the shipped plugin code.**
`fetch_mint_facts` in both plugins' real `#[cfg(target_family = "wasm")]`
shim propagates this RPC error with `?` and fails the whole call closed
— confirmed by `payment-watch` hard-failing with this exact error when
called live through the daemon. Fine from a safety standpoint
(fail-closed is the right default for a risk check), but the plugin
does *not* self-heal against this specific free-endpoint restriction the
way the harness did; only pointing `rpc_url` at a dedicated provider
(Helius/QuickNode/Triton/etc.) actually fixes it — noted in both
plugins' READMEs. Whether the shipped plugin should also degrade
gracefully here (matching the harness) is an open call for later, not
made today (no plugin code changed this session).

## ZeroClaw daemon integration findings (2026-07-22)

Built ZeroClaw from source (`--features plugins-wasm,plugins-wasm-cranelift`
— the prebuilt release binary has no plugin support at all, confirmed via
`zeroclaw plugin --help` being an unrecognized subcommand), installed all
three plugins locally (`zeroclaw plugin install ./plugins/<name>`, after
copying each plugin's `target/wasm32-wasip2/release/*.wasm` next to its
`manifest.toml` — the CLI does not build the component itself), wired a
local Ollama model (no API key needed) and a real Telegram bot, and ran
the actual daemon end to end.

**Manual / on-request calls work correctly.** Asked the agent (via a
one-shot `zeroclaw agent -a assistant -m "..."`) to call `payment-watch`
against the real devnet test payment from the earlier verification run.
Even the tiny local model (`qwen2.5:0.5b`, chosen for a slow/unreliable
sandbox network) parsed the instruction and called the tool with exactly
the right `recipient`/`amount`/`mint` arguments, no approval prompt
(risk-profile `auto_approve` covering the plugin tools worked as
configured), and failed only on the already-known `getTokenLargestAccounts`
429 above — not a new bug. Slow (~55s to first response on this
hardware/model) but functionally correct. This is real evidence
`payment-watch` and `token-risk-check` work as tools inside a live
ZeroClaw agent, not just against the standalone harness.

**Autonomous cron-fired notification does not work, and this traces back
to the bounty brief itself.** Tried to build a cron SOP
(`sops/payment-watch-poll/`) that polls `payment-watch` every 2 minutes
and messages the Telegram owner via `send_via` the moment status flips to
"paid" — the literal implementation of the bounty's own framing of
`payment-watch` as "(T0, SOP-triggered) ... Fires an inbound event when it
lands." Two layers of finding, most specific first:

1. `execution_mode = "supervised"` (the SOP default) gates approval before
   step 1 on *every* run, forever — not just a description of the SOP
   at review time, an approval prompt each cron tick. Setting
   `execution_mode = "auto"` in `SOP.toml` removes that gate.
2. Even with that fixed, the run never executes. The daemon logs, every
   single cron tick: `"ready for step 1 'Check payment' but no agent loop
   available to execute"`. A cron-triggered SOP whose steps call tools
   (anything other than the fixed `deterministic`-mode capability set —
   `shell.exec`, `llm.generate`, `forge.comment`, `notify.channel`) cannot
   self-drive in this ZeroClaw version. It genuinely needs a live
   agent-loop turn — an actual chat message on some channel — to "catch
   up" and advance a pending run. Sending the bot a plain "hi" is what
   made the stuck run move at all; that's opportunistic, not autonomous.
   There is no config fix for this: `deterministic` mode avoids needing
   an agent loop, but its capability set can't call an arbitrary WASM
   tool plugin, so there's no way to get both "no LLM required" and
   "calls our plugin" in one SOP today.

The deeper reason: what the bounty brief describes — something that
watches the chain and pushes a notification into the conversation on its
own, unprompted — is ZeroClaw's `observer` plugin capability. Per
`docs/book/src/plugins/index.md`'s wiring-status table in the daemon repo,
`observer` is **reserved, with no WIT world or adapter implemented yet**
in this version. It isn't buildable by any plugin today, not just ours.
Building `payment-watch` as a `tool` (called on request, or nudged by an
SOP step) isn't a shortfall against the brief — given `observer` doesn't
exist yet, it's the closest correct implementation actually possible
against the real ZeroClaw runtime. State this plainly, with this evidence,
in the final write-up rather than glossing over it or claiming the cron
demo works when it doesn't.

## THE ROADMAP

This section is the single authoritative task list — it replaces the
old "Pre-submission checklist" entirely. If anything elsewhere in this
file (an addendum, a "next thing to tackle" note) conflicts with the
order or content below, this section wins.

**Confirmed context:** bounty rules changed to a showcase-post format
(video + write-up + repo link in `#solana-bounty` Discord). No registry
PR during the bounty. Strategy: lean into the fused, unconditional
safety-check design as the genuine differentiator, since the base use
case closely matches the sponsors' own example already. (Full detail:
"RULE CHANGE — CONFIRMED" at the top of this file.)

**The roadmap, in order:**

1. **HIGHEST PRIORITY:** confirm whether `payment-watch`'s automatic SOP
   cron trigger now actually completes end to end and posts an
   unprompted "Invoice paid" message on its own, with no human asking
   first — this is the single biggest gap against the sponsors' own
   literal winning-showcase example. If it works, that's the headline
   moment for the demo video. If it genuinely can't be made reliable in
   reasonable time, document the honest reason in the write-up rather
   than fake it, and keep the on-demand version as the proven fallback.
   (Prior finding on this exact gap: "ZeroClaw daemon integration
   findings" below — re-verify, don't just trust that log.)
2. Confirm status of two open items from earlier: BRL-equivalent
   display, and whether `payment-watch`'s matching logic already
   rejects a dust/tiny fake payment.
3. Confirm QR-image support for `solana-pay-request` landed correctly.
4. Rewrite the wasm32-wasip2 write-up paragraph honestly against the
   NEW rules' Tier 3 guidance (modular solana crates now compile clean
   to wasm32-wasip2 — don't repeat the old "avoid solana-sdk entirely"
   framing as if it still fully holds; describe what was actually built
   and found, note the newer guidance as real next-step territory).
5. Deepen the three existing plugins, one at a time, each fully tested
   before the next:
   a. **DONE 2026-07-23.** Guardrails on `solana-pay-request` (config
      `max_amount` + `mint_allowlist`, enforced in core, prompt-injection
      tested) plus auto-generated `reference` via `getrandom`. 24/24
      tests pass; wasm32-wasip2 release build + `clippy -D warnings`
      clean on host and wasm.
   b. **DONE 2026-07-23.** LP status check added to `token-risk-check`,
      opt-in via `lp_check = "true"` config (off by default -- zero
      behavior change for anyone who doesn't set it, including this
      repo's own devnet demo mints, which Dexscreener has never indexed
      and would otherwise get flagged amber). New shared `dex` module in
      `solana-core` parses Dexscreener's real response shape (confirmed
      live: `{"pairs": null}` for an unindexed mint, real pair/liquidity
      data for a known one -- e.g. USDC's pools total ~$2.4M live on
      2026-07-23). `MintFacts` gains `lp_pool_found`/`lp_liquidity_usd`
      (both `Option`, both flow through `assess()` in the shared core, so
      `payment-watch` gets this signal too if ever enabled there); the
      fail-open contract is load-bearing and tested both directions
      (`missing_lp_data_does_not_change_a_clean_verdict`,
      `missing_lp_data_cannot_mask_a_red_verdict`) -- a lookup that never
      ran can't downgrade a deployment that hasn't opted in, and can't
      launder a mint that's already Red. Deliberately scoped honestly:
      this confirms pool *existence and depth*, not locked/burned LP
      status, which would need a different data source -- said plainly in
      the README rather than overclaimed. 42/42 solana-core tests (8 new)
      + 4/4 token-risk-check tests pass; wasm32-wasip2 release build +
      `clippy -D warnings` clean on host and wasm. Vendored into all three
      plugins' `solana-core/` copies by hand (no `python3` available in
      this environment to run `tools/sync_solana_core.py`) and verified
      byte-identical to the canonical copy via direct `diff`.
   c. **DONE 2026-07-23, updated same day.** Dust-defense was already a
      non-issue (see step 2 above) but got an explicit named test anyway;
      the real find was the cross-invoice reference-collision gap, now
      closed on both sides (`solana-pay-request` generates the reference,
      `payment-watch` verifies it's actually present in the matched
      transaction). The Trust Report (`recipient_verified`,
      `amount_status`, `mint_verified`, `reference_verified`,
      `tx_confirmed`) is now on **every** result, "paid" or "pending" --
      not just "paid" as first shipped. `amount_verified: bool` was later
      restructured into the tri-state `amount_status` (`Match`/`Under`/
      `None`) and `tx_confirmed` was added, as part of the deterministic
      reply-formatting work ("2026-07-23 addendum: deterministic reply
      formatting" below) -- `output.reply` is now a real, host-tested
      checklist + verdict built from these exact fields, sent verbatim by
      the agent instead of composed by the LLM. 37/37 `payment-watch`
      tests pass (up from 28: 9 new, covering the pending-path diagnosis
      and all four reply cases -- green/red/amber/not-confirmed);
      wasm32-wasip2 release build + `clippy -D warnings` clean on host
      and wasm.
6. Record the demo video (≤3 min): lead with the human story (DM,
   charge, QR, customer pays), peak on the fused safety moment (a
   payment that's "paid" but flagged red), and if step 1 succeeded,
   close on the unprompted automatic notification — matching the
   sponsors' own example beat for beat, proven, not staged.
7. Write the showcase report: use-case framing first, not a plugin
   list. Include honest tier defense for all three plugins — explicitly
   acknowledge `solana-pay-request` could be a Tier 1 Skill per current
   guidance, and that building it as a plugin was a consistency choice,
   not a necessity. Include the prompt-injection transcript, custody
   tier, config/SOP links, and the BRL-specific detail as a called-out
   originality point.
8. Post the video + write-up + repo link in `#solana-bounty` on the
   ZeroClaw Discord. This IS the submission — no PR.
9. **THEN, only if runway remains**, in order: `sns-resolve` →
   `spl-transfer-build` with real durable-nonce support (rent ~0.0015
   SOL, `AdvanceNonceAccount` must be first instruction, one nonce
   account per in-flight transaction — per the new doc's specifics) →
   `depin-attest` (software-only sensor input, no hardware required) →
   "AI negotiates a swap" idea, sequenced after `spl-transfer-build`,
   T1-only with guardrails from line one.
   **`sns-resolve` — DONE, 2026-07-23** (core + shim both, built in that
   order per instruction). Resolves a `.sol` domain (e.g. `lucas.sol`)
   to its owner address so a merchant can say "charge lucas.sol 15
   USDC" instead of a raw address, feeding straight into
   `solana-pay-request`'s `recipient`. T0, read-only, same
   pure-core/thin-shim split as the other three, fourth plugin in this
   repo. The hashing/PDA-derivation algorithm, the
   root domain authority, and the Name Service program ID were pulled
   directly from the real upstream source this session (not recalled
   from memory) — `solana-labs/solana-program-library`'s
   `name-service/program/src/state.rs` (`HASH_PREFIX`, the 3x32-byte
   seed layout, program ID `namesLPneVptA9Z5rqUDD9tMTWEJwofgaYwp8cawRkX`)
   and `SolanaNameService/sns-sdk`'s
   `rust-crates/sns-sdk/src/derivation.rs` (the root-vs-subdomain
   splitting rules, root authority `58PwtjSDuFHuUkYjH9BYnnQKHfwo9reZhC2zMJv9JPkx`).
   Two of the 8 host tests check the derivation against that same file's
   own real, published test vectors (`bonfida` →
   `Crf8hzfthWGbGbLTVCiqRqV5MVnbpHB1L9KQMd6gsinb`, `dex.bonfida` →
   `HoFfFXqFHAC8RP3duuQNzag1ieUwJRBv1HtRNiWFq4Qu`) — both pass exactly.
   **A deliberate, verified break from this repo's usual "hand-roll
   everything" rule:** correct PDA derivation needs real curve25519
   point-validity math (to find the highest bump seed that's off the
   ed25519 curve); hand-rolling that is a real correctness risk for a
   plugin whose entire job is resolving to the right address, so this
   plugin depends on the official `solana-pubkey` crate (`curve25519`
   feature) for exactly that one function
   (`core::find_program_address`) — nowhere else. This is the bounty's
   own verified Tier 3 guidance (modular Solana crates compile clean to
   `wasm32-wasip2`) getting its first real use in this repo rather than
   just being cited. Confirmed by an actual build this session, not
   assumed: `solana-pubkey` with `curve25519` compiles to a real
   `wasm32-wasip2` `cdylib` component (checked in a scratch crate first,
   then in `sns-resolve` itself). 8/8 host tests pass; `cargo clippy -D
   warnings` clean on host. **Also confirmed with `wasm-tools component
   wit` (authoritative, not raw-string grep) against the real
   `sns-resolve` wasm32-wasip2 build:** the compiled component's entire
   import surface is standard `wasi:cli`/`wasi:io`/`wasi:clocks` (stdio,
   environment, exit, clocks, polling) -- nothing from
   `curve25519-dalek`'s dependency chain introduces a JS-bindgen import
   or anything else that wouldn't instantiate inside ZeroClaw's
   constrained WASI host. An earlier raw `strings` grep on the binary
   found `wasm_bindgen`-looking symbol names and wasn't trusted as
   conclusive on its own -- this `wasm-tools` check settled it: those
   were dead names in a debug/name custom section, never actual
   required imports.
   **The wasm shim, built 2026-07-23 (same day):** one `getAccountInfo`
   call on the derived address (`fetch_account_data`), feeding
   `core::run`. New `zeroclaw_solana_core::rpc::account_data_from_result_optional`
   added to the shared core (and vendored into all four plugins' copies)
   for this -- unlike `token-risk-check`'s mint lookup, a `null` account
   here is a normal, expected "unregistered domain" outcome, not an
   error, so the existing `account_data_from_result` (which errors on
   null) was the wrong fit; the new function returns `Ok(None)` instead.
   Re-verified against the *real* `tool-plugin`-world component (not the
   generic placeholder build used for the core-only check) with
   `wasm-tools component wit`: imports are `zeroclaw:plugin/logging`,
   `wasi:http/*` (the `http_client` grant), the standard WASI p2
   baseline, and one addition beyond what the other three plugins need --
   `wasi:random/insecure-seed`, from the `curve25519-dalek` chain.
   Confirmed that import is satisfied unconditionally by ZeroClaw's own
   host wiring by reading the actual source
   (`wasmtime_wasi::p2::add_to_linker_async` in
   `crates/zeroclaw-plugins/src/component.rs:229`, called for every
   plugin regardless of permissions), not assumed from general WASI
   knowledge. 8/8 host tests still pass; `cargo clippy -D warnings`
   clean on host and wasm; wasm32-wasip2 release build succeeds.
   **Live-verified against real mainnet, 2026-07-23** (full detail: the
   plugin's own README, "Live mainnet verification"). Devnet has no
   meaningful public SNS registry to test against and this plugin
   doesn't carry SNS's separate devnet root-domain constant yet
   (`5eoDkP6vCQBXqDV9YN2NdUs3nmML3dMRNmEYpiyVNBm2`, found in the real
   source but not wired in) -- a read-only mainnet check was judged the
   more faithful "live" proof, since that's the network SNS domains
   actually, meaningfully live on. `bonfida.sol` resolved correctly
   (address bit-identical to the golden-vector test, independently
   reconfirmed live: the real account there is owned by our hardcoded
   Name Service program ID); `dex.bonfida.sol` derived to the same
   address as the golden vector but correctly reported "unregistered"
   (that specific domain isn't currently registered -- not a derivation
   bug, the address matches the reference exactly); a
   definitely-never-registered domain also correctly resolved to
   "unregistered." Harness: standalone Rust binary in scratch calling
   `core::domain_key`/`core::run` directly (same code the wasm ships),
   native HTTP client instead of `waki`, same pattern as the original
   three-plugin devnet verification. Also refreshed the
   `payment-watch-poll` SOP (`~/.zeroclaw/data/sops/payment-watch-poll/`,
   outside this repo) to send the new `reply` field instead of the old
   free-text `summary` -- it predated the reply-formatting work.

**RULE:** step 1 takes priority over everything else, including items
already "in progress" from before — it's the single highest-leverage
open gap. Do not skip to steps 5+ before 1-4 are confirmed.

**Step 2 — CONFIRMED, 2026-07-23:**
- **BRL-equivalent display:** done, both plugins, live-tested (full
  detail in "2026-07-22 addendum: the BRL touch" below). Nothing left
  here.
- **Dust/tiny-fake-payment rejection:** already correctly enforced,
  not a gap. `match_payment` in `payment-watch/src/lib.rs` requires
  *both* an exact mint match and `amount_raw >= expected_raw` (computed
  via `decimal_to_raw` on the real mint decimals) — a dust transfer
  (any amount below what was requested) cannot satisfy a real invoice.
  This was previously only proven incidentally by generic tests
  (`underpayment_does_not_match`, `wrong_mint_does_not_match`); added a
  new test named for this specific threat framing,
  `a_dust_transfer_does_not_satisfy_a_real_invoice` (1 raw unit against
  a 25 USDC invoice), so the coverage is explicit, not just inferred.
  20/20 `payment-watch` tests pass.
- **A more important related gap found while checking this (not what
  was asked, but the real risk hiding next to it):** `match_payment`
  has no way to verify a matched transfer against the specific
  `reference` the invoice actually requested — it only uses `reference`
  (when present) to pick which address `getSignaturesForAddress`
  queries against; once a candidate transaction is fetched, matching is
  purely `mint` + `amount >= expected`, checked against *any* transfer
  the recipient received in that transaction. Two consequences: (1) if
  the calling agent omits `reference` (nothing currently forces it not
  to — `solana-pay-request` treats it as fully optional and never
  generates one itself), `payment-watch` falls back to querying by
  `recipient` directly, so two concurrently open invoices for the same
  recipient/mint could cross-match — a second customer's unrelated
  overpayment could satisfy a different invoice's "paid" check; (2)
  even when a `reference` *is* supplied, nothing stops the LLM from
  reusing a well-known address (this happened in the original live
  verification run, which used the WSOL mint address as a stand-in
  reference) instead of a fresh, single-use random value, defeating the
  correlation's purpose. **Fixed 2026-07-23, in step 5:** 5a —
  `solana-pay-request` now auto-generates a cryptographically random
  reference via `getrandom` whenever the caller omits one (same pattern
  `plugins/wecom-ws` already uses on `wasm32-wasip2`, so this wasn't a
  new/unproven capability), and fails the request rather than degrading
  to a guessable value if entropy is unavailable. 5c — `payment-watch`
  now verifies the matched transaction's account keys actually contain
  the requested `reference` directly (`ObservedTransfer::has_reference`,
  enforced in `match_payment`), independent of whichever address the RPC
  query happened to search by — real defense in depth, not just trust in
  the query filter. Tested on both sides: `solana-pay-request`'s
  reference-generation path, and `payment-watch`'s
  `a_transfer_missing_the_requested_reference_does_not_match` /
  `a_transfer_carrying_the_requested_reference_does_match`. 28/28
  `payment-watch` tests pass.

**Step 3 — CONFIRMED, 2026-07-23:** QR-image support for
`solana-pay-request` landed correctly and is solid: `qr_url` field
built via goQR.me, covered by
`core::tests::qr_url_embeds_the_percent_encoded_pay_url`, and
independently confirmed live on Telegram (a real scannable QR image
rendered via the `[IMAGE:<qr_url>]` marker) — full detail in "2026-07-22
addendum: QR code output" below. 17/17 `solana-pay-request` tests pass.
Nothing left here.

## 2026-07-22 addendum: QR code output + a real redaction bug fix

- `solana-pay-request` now also returns `qr_url`: a QR-code image of the
  same pay URL, built via the free, no-auth goQR.me API
  (api.qrserver.com; confirmed via their own docs -- no request limit,
  no attribution required, they state they don't log QR contents).
  Still pure string formatting, no new permission, no plugin-side
  network call -- the plugin only builds the URL string. The wasm shim's
  tool description now tells the agent to include
  `[IMAGE:<qr_url>]` in its reply, which ZeroClaw's Telegram channel
  recognizes as a real outbound photo-attachment marker (confirmed
  working live: an actual scannable QR image arrived in Telegram, not
  just a link). Tests updated (`qr_url_embeds_the_percent_encoded_pay_url`).
- **Real bugs found and fixed (ZeroClaw daemon config, not our plugin) --
  two separate, independent checks, both false-positiving on legitimate
  Solana data:**
  1. `security.leak_detection`'s high-entropy-token heuristic was
     redacting every Solana address as `[REDACTED_HIGH_ENTROPY_TOKEN]`
     -- base58 addresses and API keys look identical to a pure
     Shannon-entropy check. Fixed with
     `security.leak_detection.high_entropy_tokens = false`.
  2. Even after that fix, `&spl-token=<mint>` still came back as
     `&spl-[REDACTED_SECRET]`. Separate cause: `check_generic_secrets`'s
     regex `(?i)token[=:]\s*['"]*[a-zA-Z0-9_.-]{20,}` has no word-boundary
     anchor, so it matches the literal substring "token=" inside
     "spl-**token**=", indistinguishable from a real `access_token=...`
     leak. This check is the only one in the whole leak-detector gated by
     `sensitivity > 0.5` (default 0.7); every other real check --
     API keys, AWS credentials, JWTs, private keys -- is unconditional.
     Fixed with `security.leak_detection.sensitivity = 0.5`, which
     disables just this one overly-broad check and leaves the rest
     active.
  Both confirmed fixed via a live Telegram reply showing the complete,
  unredacted pay URL. Worth a line in the write-up either way: anyone
  deploying this against a real payment terminal needs both of these
  same two config changes, or addresses and mint parameters come back
  redacted and the pay URL is unusable.
- **Tried and reverted:** a markdown hyperlink (`[Tap to pay](solana:...)`)
  for the pay URL, hoping Telegram would render it as clickable text.
  Confirmed via live Telegram test that it does not -- Telegram parsed
  the surrounding code-fence backticks fine but rendered the link syntax
  as literal unparsed text instead of a clickable link. `solana:` isn't a
  scheme Telegram treats as linkable, even via explicit markdown link
  syntax, not just plain-text auto-linking. Reverted the tool's
  description to stop instructing this and to show `url` only once (a
  single code block for copy/paste) -- the QR image (`[IMAGE:<qr_url>]`)
  is the one reliable one-tap path, since a wallet's camera scan bypasses
  the link-scheme problem entirely.

## 2026-07-22 addendum: the BRL touch, hybrid live/static

Both `solana-pay-request` and `payment-watch` now implement the root
README's "Brazil touch": a `brl_estimate` display field alongside the
crypto amount. Design, chosen deliberately over either extreme:

- A configured `brl_rate` (decimal string, BRL per unit of the asset) is
  both the operator's opt-in signal for this whole feature and the
  fallback figure. Omit it entirely and there are zero extra network
  calls and no `brl_estimate` field at all -- unchanged from the
  static-only version.
- When it's set, a live rate is tried first: [Jupiter's price
  API](https://api.jup.ag/price/v3?ids=<mint>) (free, no API key,
  confirmed live -- real USD price for a known mint, empty `{}` --not an
  error-- for a mint with no market, e.g. our own test mints) times
  [Frankfurter's](https://api.frankfurter.dev/v1/latest?from=USD&to=BRL)
  free daily USD->BRL rate. Any failure (unreachable, rate-limited, no
  price data) falls back to the operator's static `brl_rate` silently --
  never a hard error, since this is a display-only nicety that must never
  block the actual payment request/confirmation.
- `core::run`/`core::confirm` are completely unchanged by this -- both
  already just took a single resolved `Option<f64>` rate, so the
  live-vs-static decision lives entirely in each wasm shim, mirroring
  where `token-risk-check` fetches `MintFacts` before calling its own
  pure core.
- New permission: `solana-pay-request` now has `http_client`, used
  *only* for this opportunistic price upgrade -- building the Solana Pay
  URL itself is still pure string formatting and touches no network.
  `payment-watch` already had `http_client` for its RPC calls.

**Real bug found and fixed:** `frankfurter.app` (the URL used in the
first working version) 301-redirects permanently to
`frankfurter.dev/v1/latest` -- confirmed via `curl -I`. `waki` does not
follow redirects, so every live-rate attempt silently failed with
"invalid json: expected value at line 1 column 1" (the redirect's HTML
body, not real data), falling back to the static rate every time without
ever surfacing an error. Only caught because the resulting figure
(R$140.00) exactly matched the static-rate math and didn't match a
manually computed live estimate (~R$127) -- a coincidentally "working
right" static fallback almost hid a completely broken live path. Fixed
by pointing directly at the post-redirect `.dev` URL. Confirmed
genuinely live afterward via an explicit diagnostic log line
(`live brl rate used: 5.0783...`, matching Jupiter's real-time USDC
price times Frankfurter's real daily rate) -- not just a plausible-looking
number.

## 2026-07-23 addendum: a real Solana Pay spec-compliance bug, found while re-checking the QR-scan report

Earlier in this project, a live Telegram test found that a wallet scanned
`solana-pay-request`'s QR code but didn't recognize/display the amount.
At the time this was diagnosed as most likely a mainnet-only USDC mint
address used in what should have been a devnet-context test (a testing-
methodology issue, not a plugin bug) — a native-SOL isolation test was
proposed to narrow it down but never confirmed completed.

Revisited 2026-07-23 by reading the actual Solana Pay spec
(`solana-labs/solana-pay`'s `spec/SPEC.md`, "Amount" section) rather than
guessing further. Found a real, independent bug while there: the spec
states plainly, "If the value is a decimal number less than 1, it must
have a leading `0` before the `.`" — `is_valid_amount` in
`plugins/solana-pay-request/src/lib.rs` didn't enforce this; it accepted
`.5` as valid (missing the leading `0`), which is spec-invalid input. A
wallet that validates strictly is entitled to reject a URL like that as
malformed, which would present to a user exactly as "scanned fine, but
the amount never showed up" — a second, previously-undiscovered
candidate explanation for the original report, found by reading the
actual spec instead of re-guessing. Fixed: `is_valid_amount` now tracks
digits seen before the `.` and rejects a bare leading dot. Two new tests
(`rejects_amount_missing_a_leading_zero_before_the_dot`,
`accepts_amount_with_the_required_leading_zero`); 31/31
`solana-pay-request` tests pass, `cargo clippy -D warnings` clean.

**Not independently re-confirmed live** (would need a physical wallet
app scanning a real QR code, which isn't something this session can do
directly) — and the earlier mainnet/devnet mint-mismatch diagnosis is
still a live, plausible, separate explanation, not ruled out by this
fix. The existing `mint_allowlist` guardrail (5a, above) is the right
operational mitigation for that specific failure mode: an operator
running against devnet should set `mint_allowlist` to their real devnet
test mints, which structurally prevents an agent from ever building a
request against a mainnet-only mint by accident. **Whoever tests this
next needs to rebuild and reinstall the wasm component first** — this
fix is in source only; nothing currently installed in a running ZeroClaw
instance has it yet.

## 2026-07-23 addendum: deterministic reply formatting, not LLM-composed

Both `solana-pay-request` and `payment-watch` now build the exact
channel-ready reply text themselves, in `core::format_reply`, and return
it as `output.reply`. The tool description tells the agent to send it
**verbatim** -- no paraphrasing, no reformatting, no LLM-invented
confidence score. This directly targets a real, repeated failure class
seen earlier in this project: a weaker model mangling structured
arguments (the Groq argument-shape bug), and markdown-hyperlink attempts
that rendered as literal text instead of a link. Taking final-text
composition out of the model's hands removes that whole class of bug --
pure formatting over already-computed fields, no new decision logic.

**`solana-pay-request`:** `reply` is
`"Invoice Created\nInvoice: <reference>\nAmount: <amount> <symbol-or-mint>\nRecipient: <shortened>\n[IMAGE:<qr_url>]\nWaiting for payment..."`.
A small, explicit `KNOWN_MINTS` table (USDC, USDT, SOL/WSOL) maps a mint
to a symbol; anything else shows the raw address, never a guessed
symbol. `reference` is shown in full (functional, gets pasted into
`payment-watch`); `recipient` is shortened (display only, the QR/URL is
the actual payment path). 5 exact-text tests.

**`payment-watch`:** `TrustReport` was restructured to support this --
`amount_verified: bool` became `amount_status: AmountStatus` (`Match` /
`Under` / `None`), and a new `tx_confirmed: bool` was added. **Bigger
change: `trust_report` is no longer `Option` -- it's now present on
every result, "paid" or "pending"**, not just "paid". On "pending", a
new pure function `diagnose_pending` re-inspects the same already-fetched
`observed` transfer list `match_payment` looked at (no new RPC calls, no
new safety logic) to report an honest best-effort diagnosis -- e.g. a
transfer landed in the right mint+reference but under the requested
amount shows `amount_status: "under"` instead of a bare "nothing yet".
This diagnosis never feeds back into the paid/pending decision itself,
only into what's displayed. `format_reply` builds the checklist:
```
Payment Verification
✓/✗ Amount matches [(underpaid) if Under]
✓/✗ Recipient verified
✓/✗ Reference matches          <- only shown when reference_verified is Some(_)
✓/✗ Transaction confirmed
✓/✗ Token risk: GREEN/AMBER/RED — <real reasons from assess>   <- only on "paid"
Verdict: <one of four fixed strings>
```
Verdicts: GREEN -> "PAYMENT VERIFIED — safe to trust."; RED -> "DO NOT
TRUST THIS PAYMENT."; AMBER -> "PAYMENT LANDED but flagged AMBER —
review before trusting." (AMBER wasn't in the user's original spec but
follows the same shape -- `assess()` can definitely produce it, more so
now that 5b's LP check adds another amber path); anything not "paid" ->
"NOT CONFIRMED — do not treat this as paid." **Explicitly verified that
RED is not a downgraded GREEN**: same line count, same checklist
structure, only the verdict/reasons differ
(`red_reply_is_not_a_downgraded_green_reply`). 7 exact-text tests total,
plus one asserting no reply anywhere contains a %, "confidence", or
"score" -- nothing invented, only real computed fields.

Both READMEs updated with real formatted output in their worked
examples; `payment-watch`'s README has the full GREEN/RED/AMBER/NOT
CONFIRMED contrast side by side, since that contrast is this project's
core safety story. 112 tests passing across all four crates
(42 solana-core, 4 token-risk-check, 29 solana-pay-request,
37 payment-watch); `wasm32-wasip2` release builds and
`cargo clippy -D warnings` clean on host and wasm for both plugins
touched.

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
- **Transport stays on `waki` + `serde_json`, never `solana-client`** (its
  transport layer needs real sockets, which plugins don't have). This
  still fully holds. **Corrected 2026-07-23** (was: "solana-sdk /
  solana-client don't work well for wasm32-wasip2... not the official
  Solana Rust SDK" — outdated per the actual bounty brief's verified-by-
  build Tier 3 guidance): the *modular* solana crates (`solana-pubkey`,
  `solana-instruction`, `solana-message`, `solana-transaction`,
  `solana-hash`) plus `borsh`/`bs58` compile clean to `wasm32-wasip2` as
  libraries — even `solana-sdk` itself compiles for wasip2 now. Prefer
  the modular crates over hand-rolled byte encoding *if* a future plugin
  needs to build a real transaction (durable-nonce work, stretch goal 9).
  Not yet exercised as an instantiated component inside the ZeroClaw
  host specifically — budget for surprises at that boundary. Full
  verbatim guidance: "The official bounty brief — key facts" above.
- **Don't flood the context window.** Every plugin response must be
  short, shaped text (aim ~200 tokens), never raw RPC JSON dumps.
- **RPC key/URL only via config, never hardcoded.**
- **`wit/v0` is experimental, no `.frozen` marker** — the ABI can move;
  pin assumptions, expect a rebuild.

## Draft write-up paragraphs (for THE ROADMAP step 7 to paste in as-is)

### wasm32-wasip2 and the Solana Rust SDK

Rewritten 2026-07-23 against the bounty brief's own verified Tier 3
guidance — replaces any earlier "avoid the official SDK entirely"
framing, which was broader than the facts support. Ready to paste into
the showcase write-up's build-details section:

> None of the three plugins in this submission construct or serialize a
> raw Solana transaction — `token-risk-check` and `payment-watch` are
> both read-only (RPC calls only), and `solana-pay-request` builds a
> Solana Pay `solana:...` URL, which is plain string formatting, not a
> transaction. So the actual wasm32-wasip2 surface we needed was
> narrower than "the Solana SDK": base58 address parsing, JSON-RPC
> request/response handling, and Token / Token-2022 mint account TLV
> parsing — all hand-rolled in `solana-core`, verified against real
> devnet data, and requiring no Solana crate dependency at all. Network
> transport goes over `waki` (blocking `wasi:http`) + `serde_json`, not
> `solana-client`, whose transport layer assumes real sockets a wasm
> component doesn't have — this part isn't a stopgap, it's the correct
> shape for this host regardless of SDK choice.
>
> We initially assumed the official Solana Rust SDK was unusable for
> wasm32-wasip2 across the board and planned entirely around hand-rolled
> encoding as a result. That framing turned out to be broader than
> reality: the bounty's own updated guidance (verified by the sponsors
> via an actual build, not just a claim) is that the *modular* crates —
> `solana-pubkey`, `solana-instruction`, `solana-message`,
> `solana-transaction`, `solana-hash`, plus `borsh` and `bs58` — compile
> clean to wasm32-wasip2 on the stock toolchain, and even `solana-sdk`
> itself compiles for wasip2 now. The crates that still don't work are
> the browser-targeted ones (`wasm_client_solana`, `solana-client-wasm`),
> which depend on JS glue that has no wasip2 equivalent. We didn't end
> up needing any of this for the three plugins actually shipped here —
> nothing in this submission builds a transaction — but it's real,
> useful next-step territory: if we build `spl-transfer-build` (a T1
> unsigned-transaction builder with durable-nonce support, our stretch
> goal after this submission), the modular crates are the right starting
> point over further hand-rolled byte encoding. One honest caveat we're
> flagging rather than glossing over: this guidance is verified as a
> *library* compile target, not yet exercised as an instantiated
> component inside the ZeroClaw host specifically, whose WASI capability
> grants are narrower than a generic wasm32-wasip2 target — we'd budget
> time for surprises at that specific boundary before assuming it just
> works end to end.

## Where the fuller story lives

- Root `README.md` — the full pitch, track mapping, and per-plugin status
  table.
- Each `plugins/*/README.md` — that plugin's custody tier writeup,
  threat model, and TODOs. Keep these updated as you build; they get
  submitted as-is.
