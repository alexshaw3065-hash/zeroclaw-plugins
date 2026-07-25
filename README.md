# Fiel — a self-hosted AI payment terminal for Solana

*"Evidence before trust."*

A merchant messages their agent: **"Charge Table 4 for 25 USDC."** It
replies with a real Solana Pay QR code. The customer scans it and pays.
The agent — unprompted, no one asking — notices the payment land, checks
the coin it was paid in for scam risk, and only then tells the owner
*"PAYMENT VERIFIED — safe to trust."* If the payment arrived in a
dangerous token, it says so instead, explicitly: **"DO NOT TRUST THIS
PAYMENT."**

This is that terminal, built as five real ZeroClaw plugins on top of
Solana, submitted to the *"Build Solana-native plugins for ZeroClaw"*
bounty (Superteam Brasil).

The actual showcase write-up lives at [`SHOWCASE.md`](./SHOWCASE.md) —
this README is the deeper technical reference behind it.

**Who this is for:** a small merchant — a table-service restaurant, a
market stall, a shop with a WhatsApp/Telegram presence already — who
wants to accept Solana payments without taking on the job of manually
watching a block explorer for scam tokens on every payment. Point it at
a chat, and it becomes the payment terminal.

**Which ZeroClaw features this uses**, beyond the plugins themselves:
the Telegram channel (voice + text), the top-level `zeroclaw cron`
mechanism paired with the gateway REST API's `delivery.mode="announce"`
(the unprompted "paid ✓" message), `config_read` for every plugin's
encrypted-at-rest secrets (RPC URLs, guardrail thresholds), and OS
service management (`zeroclaw service install`) for persistence.

---

## The one thing to understand before anything else

`payment-watch` will never report a payment as confirmed without first
calling the exact same risk-assessment function `token-risk-check` uses
— **as a direct function call inside tested Rust, not a suggestion the
model is asked to remember.** There is no code path that produces a
clean "paid" result without a risk verdict attached. A payment that
lands in a scam token still comes back flagged, explicitly, even though
the money genuinely arrived.

That fusion — not any single plugin — is the actual differentiator here.
See [`PROMPT_INJECTION_TRANSCRIPT.md`](./PROMPT_INJECTION_TRANSCRIPT.md)
for seven real, passing tests proving none of this can be talked around.

---

## Is this actually running?

Yes. Right now, not as a demo-day exception — configured against
**mainnet** (a real Helius RPC endpoint). Every plugin here was
originally built and verified against devnet (see "Live verification,"
below, for that historical record); the live deployment now runs on
mainnet, so payments through it are real. Switching networks was a
one-line `rpc_url` config change per plugin, not a rebuild.

- A real Telegram bot (`@Veepaymenttweminal_bot`), backed by a real
  ZeroClaw daemon, installed as a **persistent Windows scheduled task**
  (`zeroclaw service install`) — it survives reboots and logons, not
  just this session's terminal.
- An unprompted "payment confirmed" message really does land in
  Telegram on its own, seconds after a real payment, with zero chat
  interaction — verified live via `zeroclaw cron` + the gateway's
  `delivery.mode="announce"` (see "Live verification," below).

A companion site — [fiel-site.vercel.app](https://fiel-site.vercel.app)
— and its [`/docs`](https://fiel-site.vercel.app/docs) page explain the
product for a non-technical reader; this README is the technical
reproduction guide.

---

## Voice — hands-free, not a gimmick

The merchant scenario this is built for is a busy counter, hands full —
so the terminal understands voice notes exactly like typed messages, in
either direction:

- **Speech-to-text**: Groq's Whisper endpoint, via the legacy flat
  `[transcription]` config path (see "Reproducibility," step 8, for why
  the newer per-alias registry silently didn't register on this build).
- **Text-to-speech**: Edge TTS (free, no API key) piped through `ffmpeg`
  (with libopus) to produce a real Telegram voice note, not a text
  reply with a voice icon bolted on.
- **It's automatic, not a mode you flip**: send one voice note, and the
  chat switches into voice-reply mode for you — every reply after that
  comes back as a voice note too, until you type again. Real production
  behavior, not a demo toggle.
- **Real bug found and fixed while testing this**: very short voice
  notes (1–2 seconds) gave Whisper too little signal and it occasionally
  transcribed the wrong language entirely — not a ZeroClaw or plugin
  bug, a genuine STT limitation. Documented so a reproducing operator
  doesn't lose time on the same thing: keep voice notes to 3+ seconds.

None of this touches Solana directly — it's the interaction layer the
five plugins sit behind — but it's real, tested, and part of what makes
this a terminal someone would actually use at a counter, not just a
chat command.

---

## The Brazil touch

Superteam Brasil sponsors this bounty, and `solana-pay-request` /
`payment-watch` both carry an opt-in BRL display: set `brl_rate` and
every invoice and confirmation shows a real-time BRL estimate alongside
the crypto amount (Jupiter's price API × Frankfurter's daily USD→BRL
rate, falling back to the operator's static rate on any failure — never
a hard error over a display nicety). No PIX/bank integration — just
making the number a Brazilian merchant actually cares about visible
without asking them to do the math themselves.

---

## The five plugins

| Plugin | Tier | What it actually does |
|---|---|---|
| [`solana-pay-request`](./plugins/solana-pay-request) | **T1** — builds a request, never signs | Turns a charge into a Solana Pay QR code + link. Config-enforced `max_amount`/`mint_allowlist` guardrails, auto-generated correlation reference. Also builds a bounded, guardrailed **swap-preparation** transaction when a customer only holds SOL (still T1 — prepared for the customer's own wallet to approve, never autonomous trading). |
| [`payment-watch`](./plugins/payment-watch) | **T0** — read only | Watches for the expected payment, then fuses in the unconditional risk check above before ever reporting "paid." |
| [`token-risk-check`](./plugins/token-risk-check) | **T0** — read only | Red / amber / green scam verdict: mint/freeze authority, holder concentration, Token-2022 extensions (transfer hooks, transfer fees, permanent delegate), optional liquidity-pool depth check. |
| [`spl-transfer-build`](./plugins/spl-transfer-build) | **T1** — unsigned transaction, human signs | Builds an unsigned SPL transfer (optional ATA creation, memo, durable-nonce support). After building, **re-parses its own serialized output** and independently re-verifies every field before returning it — a second, structurally independent check on every real call, not just in tests. |
| [`sns-resolve`](./plugins/sns-resolve) | **T0** — read only | Resolves a `.sol` domain to its owner's wallet, so a merchant can say "charge lucas.sol" instead of typing a raw address. |

Nothing in this repo ever holds a signing key. **T1 is the ceiling** —
by design, not by omission.

---

## Custody ladder — and an honest defense, not a claim

| Tier | Meaning | Used here? |
|---|---|---|
| T0 | Reads public chain data, returns an observation | `payment-watch`, `token-risk-check`, `sns-resolve` |
| T1 | Builds an unsigned transaction or payment URL; a human signs | `solana-pay-request`, `spl-transfer-build` |
| T2 | Signs and submits | **Never. Not built. Not planned.** |

**Correct layering, argued honestly, not assumed:** `token-risk-check`'s
Token-2022 TLV parsing and `spl-transfer-build`'s hand-built unsigned
transactions are genuinely bounded computation that belongs in compiled
code — hand-rolling either correctly in a prompt would be unreasonable.
`solana-pay-request`'s *URL-building* half is honestly closer to
something a Tier-1 skill could do with zero compiled code — a Solana Pay
`solana:` URI is plain string formatting. Building it as a plugin here
was a consistency choice (one shared `solana-core` toolbox, one
pure-core/thin-shim pattern across every plugin), not a claim that it
was necessary in isolation. What *does* need compiled code, and is the
actual reason `solana-pay-request` and `payment-watch` exist as plugins
rather than skills, is the fused, unconditional risk check described
above — an LLM-driven skill cannot *guarantee* a second check always
runs; a direct Rust function call can, and does.

**One thing we deliberately didn't use, worth naming plainly:** ZeroClaw's
SOP approval-checkpoint feature (a run pausing until a human explicitly
approves a step). Every T1 plugin here routes approval through the
*wallet* instead — a human reviews and signs a real transaction or scans
a real QR code, rather than clicking "approve" inside a ZeroClaw SOP run.
That's a legitimate, equally real gate (nothing moves without the
human's own signature either way), but it means this submission never
exercises that specific ZeroClaw feature, and there's no refund/reversal
flow anywhere in this repo to demonstrate it against. Said outright
rather than implied.

---

## The named traps, and what we actually did about them

- **Blockhash expiry (trap #1).** `spl-transfer-build` supports durable
  nonce accounts — the real fix, not a workaround. All three gotchas
  handled: `AdvanceNonceAccount` is enforced as the first instruction,
  rent was *measured* live rather than estimated (1,447,680 lamports for
  an 80-byte nonce account, ≈0.00145 SOL), and the README documents that
  one nonce account covers exactly one in-flight transaction — parallel
  approvals need one each.
- **The wasm dependency wall (trap #2).** Confirmed by build, not by
  claim: the modular `solana-pubkey`/`solana-instruction`/`solana-message`/
  `solana-transaction`/`solana-hash` crates all compile clean to
  `wasm32-wasip2` and were used directly in `sns-resolve` (real
  `curve25519` PDA math) and `spl-transfer-build` (real transaction
  construction) — not hand-rolled byte encoding. The component-boundary
  risk the brief flags as real did surface: `wasm-tools component wit`
  against the actual compiled components confirmed one addition beyond
  the baseline WASI surface (`wasi:random/insecure-seed`, from the
  `curve25519-dalek` chain), independently confirmed satisfied by
  ZeroClaw's own host wiring by reading the actual host source, not
  assumed.
- **Context flooding (trap #3).** Every plugin returns a short, deterministic,
  pre-formatted reply (`format_reply`/`core::run`'s output) — never a raw
  RPC dump. `payment-watch`'s reply is a fixed checklist + one-line
  verdict, tested with an explicit assertion that no reply anywhere
  contains a made-up confidence score or percentage.
- **Fail-closed action certification** (named directly in the bounty's
  own "custody design space" list): built into `spl-transfer-build`. After
  constructing a transaction, it re-parses the *exact serialized wire
  bytes* it's about to return and independently re-verifies every
  field — fee payer, nonce, ATA creation, transfer amount/decimals/
  destination, memo — against the original request, on every real call,
  not just in tests. Proven by tests that hand-corrupt an
  already-correct transaction and confirm certification rejects it.
- **Squads v4 ("agent proposes, multisig disposes")** was investigated
  before building anything — real program ID, PDA seeds, Anchor
  discriminator scheme, `SmallVec` encoding all confirmed against actual
  upstream source. Verdict: moderate complexity, hand-encoding
  Anchor-style instructions from wasm exactly as the brief warns.
  Deliberately not pursued this bounty — the existing use case's
  approval path (a human reviewing a QR code or an unsigned transaction
  directly) already satisfies the same "agent proposes, human disposes"
  principle with far less risk, and depth on one real use case beats
  breadth across two.

---

## Threat model

Every plugin's own README has its full threat model. The short version,
proven with real passing tests, not just asserted:
[`PROMPT_INJECTION_TRANSCRIPT.md`](./PROMPT_INJECTION_TRANSCRIPT.md) —
seven real malicious inputs (an "ignore previous instructions" memo
trying to redirect funds, a crafted amount trying to blow past a
config-enforced spend cap, a swap request trying to smuggle in a
different destination address, and more), each with the actual test
code and why the attack has no field to travel through.

**Third-party services this depends on, declared plainly:** a Helius
RPC endpoint (reads chain data, never holds a key); Jupiter's public
Swap API (only ever returns an unsigned transaction, never signs or
holds funds — used by `solana-pay-request`'s swap-prepare path);
Groq's Whisper endpoint and Frankfurter's FX API (voice transcription
and a display-only BRL rate, respectively — both cosmetic/interaction
layer, neither touches a transaction). None of these can move funds or
hold a key on this repo's behalf — if any of them is wrong, slow, or
unreachable, the failure mode is a bad read or a missing display
figure, never an unauthorized transfer. No MCP servers or facilitators
are used anywhere in this submission.

---

## Reproducibility — set this up yourself in an evening

**1. Build ZeroClaw from source** (plugins aren't in the stock release
binary):
```bash
git clone <the zeroclaw daemon repo>
cd zeroclaw
cargo build --release --features plugins-wasm,plugins-wasm-cranelift
```

**2. Build each plugin's wasm component:**
```bash
rustup target add wasm32-wasip2   # once
(cd plugins/<name> && cargo build --locked --target wasm32-wasip2 --release)
```

**3. Install each plugin** (copies the built `.wasm` next to a generated
`manifest.toml`):
```bash
zeroclaw plugin install ./plugins/<name>
```

**4. Configure**, per plugin (`zeroclaw config set plugins.entries.<name>.config.<key> <value>`):

| Key | Needed by |
|---|---|
| `rpc_url` | every plugin except `solana-pay-request` (which only needs it for the swap-prepare upgrade) |
| `max_amount`, `mint_allowlist` | `solana-pay-request` (optional guardrails) |
| `brl_rate` | `solana-pay-request`, `payment-watch` (optional BRL display) |
| `max_slippage_bps`, `buffer_bps` | `solana-pay-request` (swap-prepare guardrails, sensible defaults if unset) |

**5. Wire a channel** (Telegram, in this deployment) and a model
provider, then run:
```bash
zeroclaw daemon
```

**6. For a genuinely persistent deployment** (not just a terminal
session):
```bash
zeroclaw service install
zeroclaw service start
```
This registers a real OS-level scheduled task/service that survives
reboots. *(One thing we hit ourselves, worth knowing: on Windows, the
generated task defaults to `DisallowStartIfOnBatteries`/
`StopIfGoingOnBatteries` — fine for a laptop, wrong for something meant
to stay up — and `zeroclaw service status`'s own self-report can give a
false "not running" on this backend even when the daemon is genuinely
healthy; verify via `curl http://127.0.0.1:<port>/health` instead.)*

**7. Auto-notification on payment** (the unprompted "paid ✓" message):
pair with the gateway (`POST /pair`), then `POST /api/cron` with a
`delivery: {mode: "announce", channel: "<your channel>", to: "<id>"}`
block and a prompt that calls `payment-watch` and returns the literal
string `NO_REPLY` while pending.

**8. Voice (optional)** — free, but needs three real pieces installed:
a Groq API key set via `transcription.api_key` (the **legacy flat
config path**, not the newer `providers.transcription.groq.<alias>`
registry — that one silently failed to register on the ZeroClaw build
this was tested against, even though it's the documented form; if voice
transcription silently doesn't work, this is the first thing to check),
the `edge-tts` Python package (`pip install edge-tts`) for replies, and
`ffmpeg` with libopus support to transcode replies into a real Telegram
voice note. All three were genuinely missing on a fresh machine and had
to be installed and diagnosed live — noted here so a reproducing
operator doesn't lose the same hour.

---

## Repo layout

```
solana-core/            canonical pure-Rust toolbox (RPC, base58, Token-2022 parsing, risk::assess) — no wasm deps
plugins/
  token-risk-check/      T0
  solana-pay-request/    T1 (+ swap-prepare upgrade)
  payment-watch/         T0 (fuses risk::assess directly)
  sns-resolve/           T0
  spl-transfer-build/    T1 (+ fail-closed action certification)
tools/sync_solana_core.py   propagates solana-core/ into each plugin's vendored copy
```

Each plugin is a fully standalone crate (no root `Cargo.toml`, matching
the wider ZeroClaw plugin ecosystem's convention) — `solana-core/` is
edited once at the repo root and vendored into each plugin that needs it
via `tools/sync_solana_core.py sync`, since the real CI builds every
plugin from an isolated snapshot of just that plugin's own folder.

## Running the test suite

184 tests pass across all six crates, host-only, zero live network
access required:
```bash
(cd solana-core && cargo test)                    # 45
(cd plugins/token-risk-check && cargo test)       # 4
(cd plugins/solana-pay-request && cargo test)     # 61
(cd plugins/payment-watch && cargo test)          # 39
(cd plugins/sns-resolve && cargo test)            # 8
(cd plugins/spl-transfer-build && cargo test)     # 27
```

## Live verification (not just mocked tests)

Every plugin has been run against real chain data at least once during
development — full detail and real transaction signatures/addresses in
each plugin's own README — including: a real scam-shaped Token-2022
mint correctly flagged red on live devnet; a real devnet payment
correctly reported "paid" but flagged red because it arrived in that
same dangerous mint; a real Solana Pay QR code scanned and recognized
correctly by a live wallet (Solflare); two real signed-and-confirmed
devnet SPL transfers via `spl-transfer-build` (a normal transfer and a
durable-nonce transfer), signed entirely outside this repo's code; a
real mainnet `.sol` domain resolution matching the upstream SNS SDK's
own published test vectors; and a real, unprompted "payment confirmed"
Telegram message delivered by `zeroclaw cron` seconds after a real
devnet transfer landed, with zero chat interaction.

**Since moving the live deployment to mainnet**, re-confirmed against
real mainnet data, not just re-tested against mocks: `sns-resolve` on
the actual running bot resolved `bonfida.sol` →
`Fw1ETanDZafof7xEULsnq9UY6o71Tpds89tNwPkWLb1v`, live, on request.

## License

MIT, per plugin (each plugin folder carries its own `LICENSE`).
