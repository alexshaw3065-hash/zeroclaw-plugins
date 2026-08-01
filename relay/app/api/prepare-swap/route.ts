/**
 * Stores a swap transaction that the `solana-pay-request` plugin has
 * already built and certified, and hands back a scannable Solana Pay
 * transaction-request link for it.
 *
 * This endpoint is deliberately a courier, not a builder. It does not
 * call Jupiter, does not compute an amount, does not derive an address,
 * and does not decide whether a swap is safe. All of that happened in
 * the plugin's tested Rust core -- including `certify_swap`, which
 * re-reads the finished transaction and proves the proceeds can only
 * land in the customer's own token account. Re-implementing any of that
 * here would create a second version of the exact logic the submission
 * claims is enforced in one place.
 *
 * Write access is gated by a shared secret so a stranger can't park
 * arbitrary transactions behind a fiel-site.vercel.app link. A scanner
 * would still see exactly what they're signing, but a link on this
 * domain shouldn't be able to imply we prepared something we didn't.
 */

import { NextRequest, NextResponse } from "next/server";
import { putSwap } from "@/lib/swapStore";

/** Base58 has no 0/O/I/l; Solana addresses land in this length range. */
const BASE58_ADDRESS = /^[1-9A-HJ-NP-Za-km-z]{32,44}$/;

export async function POST(request: NextRequest) {
  const secret = process.env.SWAP_PREPARE_SECRET;
  if (!secret) {
    return NextResponse.json(
      { error: "SWAP_PREPARE_SECRET is not configured on this deployment" },
      { status: 500 },
    );
  }
  if (request.headers.get("authorization") !== `Bearer ${secret}`) {
    return NextResponse.json({ error: "unauthorized" }, { status: 401 });
  }

  let body: {
    transaction?: unknown;
    customerWallet?: unknown;
    message?: unknown;
    ttlSeconds?: unknown;
  };
  try {
    body = await request.json();
  } catch {
    return NextResponse.json({ error: "malformed request body" }, { status: 400 });
  }

  const { transaction, customerWallet, message, ttlSeconds } = body;

  if (typeof transaction !== "string" || transaction.length === 0) {
    return NextResponse.json(
      { error: "`transaction` must be the base64 string the plugin returned" },
      { status: 400 },
    );
  }
  if (typeof customerWallet !== "string" || !BASE58_ADDRESS.test(customerWallet)) {
    return NextResponse.json(
      { error: "`customerWallet` must be a base58 Solana address" },
      { status: 400 },
    );
  }

  const id = crypto.randomUUID().replace(/-/g, "");
  const ttl =
    typeof ttlSeconds === "number" && ttlSeconds > 0 && ttlSeconds <= 300
      ? Math.floor(ttlSeconds)
      : 120;

  try {
    await putSwap(
      id,
      {
        transaction,
        customerWallet,
        message:
          typeof message === "string" && message.length > 0
            ? message
            : "Approve this swap in your own wallet. Nothing has been signed for you.",
        storedAt: Date.now(),
      },
      ttl,
    );
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "store unavailable" },
      { status: 500 },
    );
  }

  // No query parameters in the endpoint URL, so the Solana Pay link needs
  // no percent-encoding -- the spec only requires that when the target
  // URL carries its own query string.
  const endpoint = new URL(`/api/swap/${id}`, request.nextUrl.origin).toString();
  const solanaPayUrl = `solana:${endpoint}`;

  return NextResponse.json({
    id,
    endpoint,
    url: solanaPayUrl,
    qr_url: `https://api.qrserver.com/v1/create-qr-code/?size=300x300&data=${encodeURIComponent(solanaPayUrl)}`,
    expires_in_seconds: ttl,
  });
}
