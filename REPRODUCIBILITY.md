# Reproducibility — set this up yourself in an evening

**The real config, with secrets stripped:**
[`config.sample.toml`](./config.sample.toml). That is this deployment's
actual `~/.zeroclaw/config.toml`, not an idealised example — every
plugin entry, guardrail, risk profile and channel setting as it really
runs, with encrypted values replaced by placeholders. The relay's own
source is at [`relay/`](./relay).

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

**9. The swap/transfer relay (optional, but required for one-scan
signing).** Skip this and everything still works — the plugins just
return base64 transactions, which is fine if you sign with a CLI or your
own tooling. You need it only if you want a customer to *scan* a
prepared swap or transfer, because no phone wallet can sign a base64
blob pasted into a chat.

Why it has to live outside ZeroClaw: Solana Pay's *transaction request*
flow needs an HTTPS endpoint the wallet calls, and a tool plugin cannot
serve one — `wit/v0/tool.wit`'s `tool` world imports no `inbound`
interface at all. This is the same platform gap documented for x402 in
`CLAUDE.md`, not a shortcut.

**The source is in this repo, at [`relay/`](./relay)** — ~320 lines of
TypeScript, plus a README explaining why it can't live inside a plugin
and why it can't weaken custody. Copy it into any Next.js app.

What we ran, and what it costs (nothing — free tiers):

- Two API routes on a small Next.js app, deployed to Vercel:
  - `POST /api/prepare-swap` — bearer-authenticated; stores a
    transaction, returns an id, a `solana:` URL and a QR image URL.
  - `GET|POST /api/swap/<id>` — the Solana Pay handshake. `GET` returns
    a label and icon; `POST` receives `{account}` and returns the
    transaction, **but only if `account` matches the wallet it was
    prepared for**.
- Any short-TTL key/value store behind it. We used Upstash Redis via
  Vercel's Storage tab (free tier); it injects `KV_REST_API_URL` and
  `KV_REST_API_TOKEN` automatically. Entries expire after ~2 minutes,
  which is deliberate — a Jupiter swap's blockhash dies in about 60-90
  seconds anyway, so the store should not outlive the thing it holds.
- One secret you generate yourself, set as `SWAP_PREPARE_SECRET` on the
  deployment and given to the plugins.

Then point the plugins at it:

```bash
zeroclaw config set --no-interactive plugins.entries.solana-pay-request.config.swap_relay_url     https://<your-app>/api/prepare-swap
zeroclaw config set --no-interactive plugins.entries.solana-pay-request.config.swap_relay_secret  <your-secret>
zeroclaw config set --no-interactive plugins.entries.spl-transfer-build.config.transfer_relay_url    https://<your-app>/api/prepare-swap
zeroclaw config set --no-interactive plugins.entries.spl-transfer-build.config.transfer_relay_secret <your-secret>
```

Restart the daemon afterward — plugin config is read once at startup and
never hot-reloads (see the caveat above).

**Two things that will bite you.** The `config set` syntax takes the key
and value as *separate arguments*; `key=value` is parsed as one long key
and fails with a confusing "Value required" error. And the relay is
outside the trust boundary by design — every guardrail runs in the Rust
core before it ever sees a transaction, the publish is best-effort, and
a relay that is down costs convenience rather than safety. Treat the
secret as a real secret anyway: anyone holding it can park a transaction
for someone to scan.

## Running the test suite

188 tests pass across all six crates, host-only, zero live network
access required:
```bash
(cd solana-core && cargo test)                    # 49
(cd plugins/token-risk-check && cargo test)       # 4
(cd plugins/solana-pay-request && cargo test)     # 61
(cd plugins/payment-watch && cargo test)          # 39
(cd plugins/sns-resolve && cargo test)            # 8
(cd plugins/spl-transfer-build && cargo test)     # 27
```
