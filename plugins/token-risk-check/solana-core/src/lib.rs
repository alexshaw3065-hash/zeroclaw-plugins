//! zeroclaw-solana-core
//!
//! Pure Rust toolbox for talking to Solana from a wasm32-wasip2 tool
//! plugin. Nothing in this crate depends on wasm — everything here must
//! compile and test on a normal host toolchain (`cargo test`, no wasm
//! target required). Plugins import this crate and call into it from
//! their thin wasm shim.
//!
//! MIT licensed. See ../LICENSE.

pub mod pubkey;
pub mod risk;
pub mod rpc;
pub mod token;

pub use pubkey::{Pubkey, PubkeyParseError};
pub use risk::{assess, MintFacts, RiskLevel, RiskReport};
pub use rpc::{parse_response, parse_response_value, RpcClient, RpcError, RpcRequest};
pub use token::{holder_share_pct, parse_mint_account, MintParseError, ParsedMint};
