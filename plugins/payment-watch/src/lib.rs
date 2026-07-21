//! payment-watch (T0) -- not yet built.
//!
//! Watches an address for an expected amount + reference. Before
//! reporting a payment as confirmed, calls
//! `zeroclaw_solana_core::risk::assess` on the paying mint -- the same
//! function `token-risk-check` uses -- so a payment is never confirmed
//! without being screened. See README.md for the TODO list.

pub mod core {
    // TODO: pub fn check_for_payment(args: Args, observed: &[Transfer]) -> Result<Output, CoreError>
    // Output should include the risk::RiskReport for the paying mint.
}
