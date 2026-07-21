# solana-pay-request (build order: step 2)

STATUS: not yet built. See the root README for the full plan and why
this comes after `token-risk-check`.

## Custody tier: T1 (build only)

Takes {recipient, amount, mint, memo, reference}, returns a Solana Pay
`solana:` transfer-request URL and a QR-ready payload. Holds no secrets,
never signs anything -- a human pays it from their own wallet.

## TODO

- [ ] Pure core: `build_solana_pay_url(args) -> Result<PayRequest, CoreError>`
      following the Solana Pay transfer-request spec
- [ ] Host tests: valid recipient, missing amount, native SOL vs SPL mint,
      reference generation
- [ ] Prompt-injection test: a malicious `memo`/`reference` string cannot
      change the recipient or amount encoded in the URL
- [ ] BRL-equivalent display alongside the crypto amount (see root README)
- [ ] Wire the wasm shim once real WIT bindings are available
