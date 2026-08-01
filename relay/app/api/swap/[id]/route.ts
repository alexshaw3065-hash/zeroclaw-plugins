/**
 * Solana Pay **transaction request** endpoint.
 *
 * This is the piece that lets a customer scan one QR code and get a real
 * approve screen in their own wallet, instead of being handed a base64
 * blob with no way to sign it.
 *
 * The flow a wallet performs, per the Solana Pay spec:
 *   1. GET  this URL  -> `{ label, icon }`, shown while it loads.
 *   2. POST `{ account }` -> `{ transaction, message }`, which it then
 *      renders on an approve screen. The human signs, or doesn't.
 *
 * Custody note, since this is the one part of the system that talks to a
 * wallet directly: this endpoint still never signs anything and never
 * holds a key. It returns an unsigned transaction that the plugin's Rust
 * core already built and certified. The wallet does the signing, the
 * human does the approving. That keeps the whole path at T1.
 */

import { NextRequest, NextResponse } from "next/server";
import { getSwap } from "@/lib/swapStore";

/** Wallets fetch this cross-origin, so every response needs CORS. */
const CORS = {
  "Access-Control-Allow-Origin": "*",
  "Access-Control-Allow-Methods": "GET, POST, OPTIONS",
  "Access-Control-Allow-Headers": "Content-Type",
} as const;

function json(body: unknown, status = 200) {
  return NextResponse.json(body, { status, headers: CORS });
}

export async function OPTIONS() {
  return new NextResponse(null, { status: 204, headers: CORS });
}

/**
 * Step 1 of the spec. The wallet shows this while it fetches the real
 * transaction, so it has to answer even before we know who's scanning.
 */
export async function GET(
  request: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  const { id } = await params;
  const swap = await getSwap(id).catch(() => null);

  return json({
    label: "Fiel",
    icon: new URL("/logo.png", request.nextUrl.origin).toString(),
    // Not part of the spec, but harmless and useful when debugging a
    // scan that returns nothing: it distinguishes "wrong id" from
    // "expired" without needing the logs.
    ...(swap ? {} : { note: `no prepared swap for ${id} (expired or unknown)` }),
  });
}

/**
 * Step 2 of the spec. The wallet posts the account that's about to sign,
 * and we hand back the transaction built for exactly that account.
 */
export async function POST(
  request: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  const { id } = await params;

  let account: unknown;
  try {
    ({ account } = (await request.json()) as { account?: unknown });
  } catch {
    return json({ error: "malformed request body" }, 400);
  }

  if (typeof account !== "string" || account.length === 0) {
    return json({ error: "missing `account`" }, 400);
  }

  let swap;
  try {
    swap = await getSwap(id);
  } catch (e) {
    return json({ error: e instanceof Error ? e.message : "store unavailable" }, 500);
  }

  if (!swap) {
    // Almost always the blockhash TTL having elapsed rather than a bad
    // link, so say that plainly -- a wallet surfaces this string to the
    // person holding the phone.
    return json(
      { error: "This swap request has expired. Ask for a fresh one." },
      404,
    );
  }

  // The transaction was built for one specific wallet: that wallet is the
  // fee payer, the swap authority, and the sole destination of the
  // proceeds (which `certify_swap` already verified in Rust). Serving it
  // to a different scanner would only produce a transaction that fails,
  // so reject it here with a clear reason instead.
  if (account !== swap.customerWallet) {
    return json(
      {
        error:
          "This swap was prepared for a different wallet. Ask for one built " +
          "for the wallet you're paying from.",
      },
      403,
    );
  }

  return json({ transaction: swap.transaction, message: swap.message });
}
