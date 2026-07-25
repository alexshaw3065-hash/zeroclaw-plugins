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

**Submission format:** a Discord showcase post (video + write-up + this
repo), per the bounty's confirmed rule change — **not a pull request**.
Registry merges, if any, happen separately after judging.

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

Yes. Right now, not as a demo-day exception — currently configured
against **devnet** (a real Helius RPC endpoint, real on-chain state,
just not real customer money yet), matching how every plugin here was
built and verified. Moving to mainnet is a one-line `rpc_url` config
change per plugin, not a rebuild, whenever real payments are wanted.

- A real Telegram bot (`@Veepaymenttweminal_bot`), backed by a real
  ZeroClaw daemon, installed as a **persistent Windows scheduled task**
  (`zeroclaw service install`) — it survives reboots and logons, not
  just this session's terminal.
- It understands voice notes as well as text (Groq Whisper in, Edge TTS
  out) — send it a voice message and it replies the same way.
- An unprompted "payment confirmed" message really does land in
  Telegram on its own, seconds after a real payment, with zero chat
  interaction — verified live via `zeroclaw cron` + the gateway's
  `delivery.mode="announce"` (see "Live verification," below).

A companion site — [fiel-site](https://fiel.vercel.app) (deploy pending)
— and its `/docs` page explain the product for a non-technical reader;
this README is the technical reproduction guide.

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

Every plugin has been run against real chain data at least once — full
detail and real transaction signatures/addresses in each plugin's own
README — including: a real scam-shaped Token-2022 mint correctly
flagged red on live devnet; a real devnet payment correctly reported
"paid" but flagged red because it arrived in that same dangerous mint;
a real Solana Pay QR code scanned and recognized correctly by a live
wallet (Solflare); two real signed-and-confirmed devnet SPL transfers
via `spl-transfer-build` (a normal transfer and a durable-nonce
transfer), signed entirely outside this repo's code; a real mainnet
`.sol` domain resolution matching the upstream SNS SDK's own published
test vectors; and a real, unprompted "payment confirmed" Telegram
message delivered by `zeroclaw cron` seconds after a real devnet
transfer landed, with zero chat interaction.

## License

MIT, per plugin (each plugin folder carries its own `LICENSE`).
