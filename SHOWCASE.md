# Fiel — a self-hosted AI payment terminal for Solana

*"Evidence before trust."*

**Video:** _[link — recording pending]_
**Repo:** https://github.com/alexshaw3065-hash/zeroclaw-plugins
**Site:** https://fiel-site.vercel.app · docs at https://fiel-site.vercel.app/docs

---

## What it does

A merchant messages their agent: **"Charge Table 4 for 25 USDC."** It
replies with a real Solana Pay QR code. The customer scans it, pays.
Seconds later — unprompted, no one asking — the agent notices the
payment land, checks the coin it was paid in for scam risk, and only
then says **"PAYMENT VERIFIED — safe to trust."** If it landed in a
dangerous token, it says so instead: **"DO NOT TRUST THIS PAYMENT."**
It also understands voice notes in either direction, and can prepare a
guardrailed swap when a customer only holds SOL, and resolve `.sol`
names, and build unsigned SPL transfers — but the fused
payment-confirm-and-risk-check loop above is the core job.

## Who it's for

A small merchant — table-service restaurant, market stall, any shop
with a Telegram/WhatsApp presence already — who wants to accept Solana
payments without personally watching a block explorer for scam tokens
on every single payment.

## Which ZeroClaw features it uses

The Telegram channel (voice + text), the top-level `zeroclaw cron`
mechanism + the gateway REST API's `delivery.mode="announce"` (the
unprompted "paid ✓" message — no chat interaction needed), `config_read`
for every plugin's encrypted secrets, and `zeroclaw service install` for
persistence (this is genuinely running as an OS-level service right
now, not started fresh for a demo).

## What we had to build

Five ZeroClaw tool plugins on a shared `solana-core` Rust toolbox:
`solana-pay-request` (T1, + a swap-preparation upgrade), `payment-watch`
(T0), `token-risk-check` (T0), `spl-transfer-build` (T1, with fail-closed
action certification), `sns-resolve` (T0). Full detail, including an
honest correct-layering defense (one of the five is closer to something
a Tier-1 skill could've done — said plainly, not glossed over), in the
[repo README](https://github.com/alexshaw3065-hash/zeroclaw-plugins).

## Custody tier + threat model

**T0/T1 only. No plugin here ever holds a signing key, ever.** T0 =
reads and reports. T1 = builds something a human signs themselves —
an unsigned transaction or a Solana Pay URL. T2 was never built and
isn't planned.

Every fund-adjacent input has a real, passing test proving it fails
closed — not asserted, proven. Full transcript, 7 real malicious inputs
quoted verbatim from actual test code (an "ignore previous instructions"
memo trying to redirect a transfer, a crafted amount trying to blow past
a config-enforced spend cap, a swap request trying to smuggle in a
different destination address, and more), each with what actually
happens and why:
[`PROMPT_INJECTION_TRANSCRIPT.md`](https://github.com/alexshaw3065-hash/zeroclaw-plugins/blob/main/PROMPT_INJECTION_TRANSCRIPT.md)

**Third-party trust, declared:** a Helius RPC endpoint, Jupiter's public
Swap API, Groq's Whisper endpoint, and Frankfurter's FX API — none of
them hold a key or sign anything on this repo's behalf; worst case of
any of them failing is a bad read or a missing display figure, never an
unauthorized transfer. No MCP servers, no facilitators.

## Reproduce this yourself

Full step-by-step (build from source, install each plugin, config keys,
persistent service, the two Windows gotchas we hit ourselves) in the
[repo README](https://github.com/alexshaw3065-hash/zeroclaw-plugins#reproducibility--set-this-up-yourself-in-an-evening).
184 tests pass across all six crates, host-only, zero live network
access required.

## Proof this is real, not staged

**The live deployment runs on mainnet now** — real Helius mainnet RPC,
confirmed with real calls (real slot numbers, a real live `sns-resolve`
lookup: `bonfida.sol` → `Fw1ETanDZafof7xEULsnq9UY6o71Tpds89tNwPkWLb1v`,
on request, on the actual running bot).

During development, every plugin was proven against real chain data at
least once: a real scam-shaped Token-2022 mint correctly flagged red on
live devnet. A real devnet payment correctly reported "paid" but
flagged red because it arrived in that same dangerous mint. A real
Solana Pay QR code scanned and recognized by a live wallet (Solflare).
Two real signed-and-confirmed devnet SPL transfers (normal +
durable-nonce), signed entirely outside this repo's code. A real
mainnet `.sol` domain resolution matching the upstream SNS SDK's own
published test vectors. A real, unprompted "payment confirmed" Telegram
message delivered by `zeroclaw cron` seconds after a real devnet
transfer landed, zero chat interaction. Real transaction signatures and
addresses for every one of these are in the individual plugin READMEs,
not just claimed here.

---

*Built on ZeroClaw, MIT/Apache-2.0, community-maintained. This
submission is a Discord showcase post per the bounty's confirmed
format — not a pull request.*
