# Reproducibility — set this up yourself in an evening

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
