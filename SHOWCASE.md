# Fiel — a self-hosted AI payment terminal for Solana

*"Evidence before trust."*

**Video:** _[link, recording pending]_
**Repo:** https://github.com/alexshaw3065-hash/zeroclaw-plugins
**Site:** https://fiel-site.vercel.app · docs at https://fiel-site.vercel.app/docs
**X:** https://x.com/0xviktor4

---

## What it does

A merchant messages their agent: **"Charge Table 4 for 25 USDC."** It
replies with a real Solana Pay QR code. The customer scans it, pays.
Seconds later, unprompted, with no one asking, the agent notices the
payment land, checks the coin it was paid in for scam risk, and only
then says **"PAYMENT VERIFIED — safe to trust."** If it landed in a
dangerous token, it says so instead: **"DO NOT TRUST THIS PAYMENT."**
It also understands voice notes in either direction, and can prepare a
guardrailed swap when a customer only holds SOL, and resolve `.sol`
names, and build unsigned SPL transfers, but the fused
payment-confirm-and-risk-check loop above is the core job.

## Who it's for

A small merchant: table-service restaurant, market stall, any shop with
a Telegram/WhatsApp presence already, who wants to accept Solana
payments without personally watching a block explorer for scam tokens
on every single payment.

## Which ZeroClaw features it uses

The Telegram channel (voice + text), the top-level `zeroclaw cron`
mechanism plus the gateway REST API's `delivery.mode="announce"` (the
unprompted "paid ✓" message, no chat interaction needed), `config_read`
for every plugin's encrypted secrets, and `zeroclaw service install` for
persistence (this is genuinely running as an OS-level service right
now, not started fresh for a demo).

## What we had to build

Five ZeroClaw tool plugins on a shared `solana-core` Rust toolbox:
`solana-pay-request` (T1, plus a swap-preparation upgrade), `payment-watch`
(T0), `token-risk-check` (T0), `spl-transfer-build` (T1, with fail-closed
action certification), `sns-resolve` (T0). Full detail, including an
honest correct-layering defense (one of the five is closer to something
a Tier-1 skill could've done, said plainly, not glossed over), in the
[repo README](https://github.com/alexshaw3065-hash/zeroclaw-plugins).

`solana-core` itself is a standalone, wasm32-wasip2 Solana toolbox
(base58 parsing, a thin JSON-RPC client, Token-2022 TLV parsing, durable
nonce helpers, the shared risk heuristic), vendored into all five
plugins rather than reimplemented per plugin. Full spotlight in the
[repo README](https://github.com/alexshaw3065-hash/zeroclaw-plugins#solana-core-shared-infrastructure-worth-its-own-look).

## Custody tier + threat model

**T0/T1 only. No plugin here ever holds a signing key, ever.** T0 =
reads and reports. T1 = builds something a human signs themselves: an
unsigned transaction or a Solana Pay URL. T2 was never built and
isn't planned.

Every fund-adjacent input has a real, passing test proving it fails
closed, not asserted, proven. Full transcript, 7 real malicious inputs
quoted verbatim from actual test code (an "ignore previous instructions"
memo trying to redirect a transfer, a crafted amount trying to blow past
a config-enforced spend cap, a swap request trying to smuggle in a
different destination address, and more), each with what actually
happens and why:
[`PROMPT_INJECTION_TRANSCRIPT.md`](https://github.com/alexshaw3065-hash/zeroclaw-plugins/blob/main/PROMPT_INJECTION_TRANSCRIPT.md)

**Third-party trust, declared:** a Helius RPC endpoint, Jupiter's public
Swap API, Groq's Whisper endpoint, and Frankfurter's FX API. None of
them hold a key or sign anything on this repo's behalf; worst case of
any of them failing is a bad read or a missing display figure, never an
unauthorized transfer. No MCP servers, no facilitators.

## Reproduce this yourself

Full step-by-step (build from source, install each plugin, config keys,
persistent service, the relay, and the gotchas we hit ourselves) in
[`REPRODUCIBILITY.md`](https://github.com/alexshaw3065-hash/zeroclaw-plugins/blob/main/REPRODUCIBILITY.md).
188 tests pass across all six crates, host-only, zero live network
access required.

Everything an operator needs is in the repo, nothing withheld:

- **Config** — [`config.sample.toml`](https://github.com/alexshaw3065-hash/zeroclaw-plugins/blob/main/config.sample.toml),
  this deployment's *actual* `~/.zeroclaw/config.toml` with secrets
  stripped. Not an idealised example: every plugin entry, guardrail,
  risk profile and channel setting exactly as it really runs.
- **Relay source** — [`relay/`](https://github.com/alexshaw3065-hash/zeroclaw-plugins/tree/main/relay),
  ~320 lines of TypeScript, with a README on why it cannot live inside a
  plugin and why it cannot weaken custody.
- **Plugin source** — [`plugins/`](https://github.com/alexshaw3065-hash/zeroclaw-plugins/tree/main/plugins),
  five crates, each with its own README, threat model, and tests.
- **Prompt-injection transcript** — [`PROMPT_INJECTION_TRANSCRIPT.md`](https://github.com/alexshaw3065-hash/zeroclaw-plugins/blob/main/PROMPT_INJECTION_TRANSCRIPT.md).

## Two things real money taught us that tests never would

Both found by running actual mainnet payments through the live daemon,
not by reading code.

**A risk check that could never pass on the most legitimate token.**
`getTokenLargestAccounts` — used to score holder concentration — is
rejected outright by Helius for real USDC: *"Too many accounts requested
(10000000 pubkeys)"*. The mint is too widely held to enumerate, so the
failure lands hardest on the most trustworthy tokens and never on the
thin ones the check exists to catch. Because the error propagated, the
whole assessment failed, and `payment-watch` — correctly failing closed
— refused to confirm the payment. The symptom was silence: a real,
finalised, correctly-matched payment that would never be confirmed, and
could never self-resolve. The fail-closed instinct was right; treating
an optional enrichment signal as mandatory was not. Holder share is now
`Option`, degrading to "unknown", with tests proving absent data can
neither invent risk nor launder a dangerous mint.

**Real USDC was being called dangerous.** Circle holds an active freeze
authority on USDC as a standard compliance control, and the heuristic
treated any freeze authority as Red — so every real USDC payment
returned "DO NOT TRUST THIS PAYMENT". Freeze authority is now Amber: it
can block movement, but unlike a permanent delegate it cannot silently
drain funds. Deliberately a **uniform severity rule, not a known-issuer
allowlist** — an allowlist means maintaining a curated trust list
forever, and contradicts judging a mint on its own observable
properties. A permanent delegate still forces Red, and the freeze reason
is still reported on a Red mint even though it no longer drives the
verdict there.

## The platform gap we hit, and had to build around

The swap feature could be demonstrated but not *completed*, for a reason
that has nothing to do with Solana: **a base64 transaction in a chat
message is unsignable.** No phone wallet has a paste-and-sign screen.

Solana Pay's *transaction request* flow is the supported fix, and it
needs an HTTPS endpoint the wallet can call. A ZeroClaw tool plugin
cannot serve one — `wit/v0/tool.wit`'s `tool` world imports no `inbound`
interface at all, so a plugin can never receive a request. Same gap we
documented for x402 earlier in this project, reached from a completely
different direction.

So the operator runs a small relay and the plugin posts its finished
transaction there, handing back a scannable code. Deliberately outside
the trust boundary: every guardrail runs in the Rust core first, the
transaction is byte-identical whether the relay works or not, publishing
is best-effort, and the relay serves a transaction only to the wallet it
was prepared for. A relay that is down costs convenience, never safety.

## Proof this is real, not staged

**The full loop, completed on mainnet with real money (2026-07-31).** A
customer wallet holding 0.109383 USDC was asked to pay a 0.2 USDC
invoice — a genuine shortfall. The agent detected it, quoted Jupiter,
built and certified a swap, and handed over a scannable request; the
customer approved it in their own wallet. Swap
[`4LxG8eTr…ogMg7`](https://solscan.io/tx/4LxG8eTrmjXS1mTZaU8LSnnCHwCvh8imw24QUKKXDXeX425duYmMeFQm562htc2V4eaqXgaAJfJpBd5z7neogMg7)
moved their balance 0.109383 → 0.310383: exactly **+0.201**, the 0.2
requested plus the 0.5% buffer the guardrails compute, confirmed on
chain rather than asserted. Twenty-two seconds later the payment
[`3RskmPaZ…piWkY`](https://solscan.io/tx/3RskmPaZYfvWU1kXP7LVK1gBKsEnww9ZrrL99vmVrnxzCHhQf1EM2agZtXGaENSK3h3tWxYS7JNosr1Kg69piWkY)
landed, and `payment-watch` confirmed it unprompted with the correct
AMBER verdict. Nothing in this repo signed either transaction.

**The live deployment runs on mainnet now**: real Helius mainnet RPC,
confirmed with real calls (real slot numbers, a real live `sns-resolve`
lookup: `bonfida.sol` → `Fw1ETanDZafof7xEULsnq9UY6o71Tpds89tNwPkWLb1v`,
on request, on the actual running bot).

During development, every plugin was proven against real chain data at
least once: a real scam-shaped Token-2022 mint correctly flagged red on
live devnet. A real devnet payment correctly reported "paid" but
flagged red because it arrived in that same dangerous mint. A real
Solana Pay QR code scanned and recognized by a live wallet (Solflare).
Two real signed-and-confirmed devnet SPL transfers (normal and
durable-nonce), signed entirely outside this repo's code. A real
mainnet `.sol` domain resolution matching the upstream SNS SDK's own
published test vectors. A real, unprompted "payment confirmed" Telegram
message delivered by `zeroclaw cron` seconds after a real devnet
transfer landed, zero chat interaction. Real transaction signatures and
addresses for every one of these are in the individual plugin READMEs,
not just claimed here.

---
