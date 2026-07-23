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
   a. Guardrails on `solana-pay-request` (config max amount + mint
      allowlist, enforced in core, prompt-injection tested)
   b. LP status check added to `token-risk-check`
   c. Dust-defense hardening on `payment-watch` (if step 2 found it's a
      real gap), including exposing recipient/amount/mint/reference as
      individual verified fields (the "Trust Report" structure)
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

**RULE:** step 1 takes priority over everything else, including items
already "in progress" from before — it's the single highest-leverage
open gap. Do not skip to steps 5+ before 1-4 are confirmed.

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

## Where the fuller story lives

- Root `README.md` — the full pitch, track mapping, and per-plugin status
  table.
- Each `plugins/*/README.md` — that plugin's custody tier writeup,
  threat model, and TODOs. Keep these updated as you build; they get
  submitted as-is.
