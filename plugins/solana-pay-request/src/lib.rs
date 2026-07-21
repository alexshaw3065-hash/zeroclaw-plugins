//! solana-pay-request (T1) -- not yet built.
//!
//! Will take {recipient, amount, mint, memo, reference} and return a
//! Solana Pay `solana:` transfer-request URL + QR-ready payload.
//! Builds a request only -- never signs, holds no secrets.
//! See README.md for the TODO list and build-order context.

pub mod core {
    // TODO: pub fn build_solana_pay_url(args: Args) -> Result<PayRequest, CoreError>
}
