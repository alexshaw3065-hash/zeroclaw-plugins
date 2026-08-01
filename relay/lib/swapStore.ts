/**
 * Tiny TTL-backed store for prepared swap transactions.
 *
 * This exists for one reason: a Solana Pay *transaction request* is two
 * separate HTTP calls from the wallet (a GET for the label/icon, then a
 * POST for the transaction itself), and Vercel functions are stateless
 * between them. So the transaction the plugin already built has to live
 * somewhere for the ~60-90 seconds its blockhash stays valid.
 *
 * Deliberately dumb: it stores and returns bytes. No swap logic, no
 * guardrails, no re-derivation. Every safety check that matters
 * (`certify_swap`, the buffer/slippage caps, `max_amount`,
 * `mint_allowlist`) already ran in the tested Rust core before the
 * transaction ever got here -- duplicating any of it in TypeScript would
 * mean a second implementation that can drift from the one the write-up
 * actually claims is enforced.
 *
 * Backed by Upstash Redis over its plain REST API, so there is no SDK
 * dependency to add -- just fetch.
 */

export type PreparedSwap = {
  /** Base64 transaction, exactly as the plugin returned it. */
  transaction: string;
  /** The wallet this swap was built for. A different scanner is rejected. */
  customerWallet: string;
  /** Human-readable line the wallet shows above the approve button. */
  message: string;
  /** Epoch ms when this was stored, for surfacing staleness honestly. */
  storedAt: number;
};

/**
 * Vercel names these `KV_REST_API_*` when Upstash is added through the
 * marketplace integration, but a directly-created Upstash database uses
 * `UPSTASH_REDIS_REST_*`. Accept either rather than making the setup
 * path depend on which route was taken.
 */
function credentials(): { url: string; token: string } {
  const url = process.env.KV_REST_API_URL ?? process.env.UPSTASH_REDIS_REST_URL;
  const token =
    process.env.KV_REST_API_TOKEN ?? process.env.UPSTASH_REDIS_REST_TOKEN;

  if (!url || !token) {
    throw new Error(
      "swap store is not configured: set KV_REST_API_URL and KV_REST_API_TOKEN " +
        "(or the UPSTASH_REDIS_REST_* equivalents) in the Vercel project",
    );
  }
  return { url, token };
}

/** One Upstash REST command, e.g. ["SET", key, value, "EX", "120"]. */
async function command(args: string[]): Promise<unknown> {
  const { url, token } = credentials();
  const res = await fetch(url, {
    method: "POST",
    headers: {
      Authorization: `Bearer ${token}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify(args),
    cache: "no-store",
  });

  if (!res.ok) {
    throw new Error(`swap store request failed: ${res.status} ${res.statusText}`);
  }
  const body = (await res.json()) as { result?: unknown; error?: string };
  if (body.error) {
    throw new Error(`swap store error: ${body.error}`);
  }
  return body.result;
}

function key(id: string): string {
  return `fiel:swap:${id}`;
}

/**
 * Store a prepared swap. `ttlSeconds` defaults to 120 -- deliberately
 * just past the ~60-90s a Jupiter-built blockhash stays valid, so an
 * entry can never outlive the transaction it holds and hand a wallet
 * something guaranteed to fail.
 */
export async function putSwap(
  id: string,
  swap: PreparedSwap,
  ttlSeconds = 120,
): Promise<void> {
  await command(["SET", key(id), JSON.stringify(swap), "EX", String(ttlSeconds)]);
}

/** Fetch a prepared swap, or null if it never existed or has expired. */
export async function getSwap(id: string): Promise<PreparedSwap | null> {
  const result = await command(["GET", key(id)]);
  if (typeof result !== "string") return null;
  try {
    return JSON.parse(result) as PreparedSwap;
  } catch {
    return null;
  }
}
