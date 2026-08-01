# The swap/transfer relay

A ~320-line Next.js service that turns a prepared transaction into
something a phone wallet can actually scan. It is the smallest piece of
this submission and the one that took the longest to justify, so it is
worth explaining why it exists at all.

## Why this isn't a plugin

`solana-pay-request` and `spl-transfer-build` both return a base64
transaction. That is correct, and it is also **unusable on a phone**:
no wallet has a paste-and-sign screen. The swap feature could be
demonstrated but never *completed*.

Solana Pay's **transaction request** flow is the supported fix. Instead
of a `solana:<address>?amount=...` transfer request (which a wallet
builds itself, and which therefore can only move tokens the customer
already holds), the QR encodes `solana:https://<your-host>/api/swap/<id>`.
The wallet then:

1. `GET`s that URL for a label and icon to show the user,
2. `POST`s `{"account": "<the scanning wallet>"}`,
3. receives a base64 transaction, and
4. renders **its own** approve screen for the human.

That requires an HTTPS endpoint the wallet can call — and **a ZeroClaw
tool plugin can never serve one.** `wit/v0/tool.wit`'s `tool` world
imports no `inbound` interface at all; a tool plugin is only ever
*invoked* by the agent's own LLM, never *called* from outside. This is
the same platform gap this project documented earlier when evaluating
x402, reached from a completely different direction. See
[`CLAUDE.md`](../CLAUDE.md) for both investigations.

So the operator runs this, and the plugins point at it.

## Why it cannot weaken custody

The relay is deliberately outside the trust boundary, and the design
makes that structural rather than promised:

- **Every guardrail has already run before the relay is contacted.**
  Spend ceiling, mint allowlist, buffer cap, quote validation, and the
  fail-closed certification that re-parses the finished wire bytes all
  execute in the pure Rust core. The relay receives a finished artifact,
  not a request.
- **The transaction is byte-identical whether the relay works or not.**
  Publishing is best-effort; a relay that is down costs the one-scan
  convenience and nothing else. Omit the config keys entirely and the
  plugins behave exactly as they did before it existed.
- **It cannot make an unsafe transaction look safe**, because it never
  sees the request that produced one. It stores bytes and hands them
  back.
- **It serves a transaction only to the wallet it was prepared for.** A
  `POST` from any other `account` is refused.
- **It holds no key and signs nothing.** The human approves in their own
  wallet, as everywhere else in this repo. Custody tier is unchanged.

A compromised relay could serve a *different* transaction — but so could
a compromised phone. The human still reads what their wallet shows
before signing, which is the same trust boundary every T1 flow in this
submission relies on.

## What's here

| File | Purpose |
|---|---|
| `app/api/prepare-swap/route.ts` | `POST`, bearer-authenticated. Stores a transaction, returns an id, a `solana:` URL, and a QR image URL. Called by the plugins. |
| `app/api/swap/[id]/route.ts` | The Solana Pay handshake. `GET` returns label/icon; `POST` returns the transaction, but only to the wallet it was prepared for. Called by the customer's wallet. |
| `lib/swapStore.ts` | Short-TTL storage. |

## Deploying it

Any host that runs Next.js works. What we ran, entirely on free tiers:

1. Deploy to Vercel.
2. Add a Redis store (Vercel's Storage tab → Upstash). It injects
   `KV_REST_API_URL` and `KV_REST_API_TOKEN` automatically.
3. Generate a secret and set it as `SWAP_PREPARE_SECRET`.
4. Point the plugins at it — exact commands in
   [`REPRODUCIBILITY.md`](../REPRODUCIBILITY.md), step 9.

**Entries expire after ~2 minutes, deliberately.** A Jupiter swap
carries an ordinary blockhash and dies in roughly 60-90 seconds anyway,
so the store should not outlive the thing it holds. This is the bounty
brief's trap #1 appearing somewhere durable nonces cannot help: the
transaction is Jupiter's to construct, not this repo's.

Treat `SWAP_PREPARE_SECRET` as a real secret — anyone holding it can
park a transaction for someone to scan.

## Status

The `solana-pay-request` path is **proven live end to end on mainnet**
with real funds (signature and balance deltas in that plugin's README).
The `spl-transfer-build` path uses the identical mechanism but has
**not** been confirmed against a real scanned transfer — stated plainly
rather than implied by association.
