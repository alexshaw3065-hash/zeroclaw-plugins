//! Parsing for raw SPL Token / Token-2022 mint account bytes.
//!
//! This turns the raw `data` bytes a `getAccountInfo` RPC call returns for a
//! mint address into the plain facts `risk::assess` needs. It never touches
//! the network itself — callers (the wasm shim) fetch the bytes over RPC and
//! hand them here, which is what keeps this host-testable with `cargo test`.
//!
//! Layout reference (SPL Token program, `spl_token::state::Mint`, a fixed
//! 82-byte struct):
//!   offset  0.. 4  mint_authority: COption tag (u32 LE, 0 = None, 1 = Some)
//!   offset  4..36  mint_authority: Pubkey (32 bytes, ignored when tag is 0)
//!   offset 36..44  supply: u64 LE
//!   offset 44      decimals: u8
//!   offset 45      is_initialized: u8 (bool)
//!   offset 46..50  freeze_authority: COption tag (u32 LE)
//!   offset 50..82  freeze_authority: Pubkey (32 bytes)
//!
//! A legacy SPL Token mint account is always exactly 82 bytes. A Token-2022
//! mint that uses any extension is longer: spl-token-2022 zero-pads the base
//! `Mint` struct out to 165 bytes (the size of a token *account*, so a
//! program can distinguish mint vs. account data purely by length), writes a
//! 1-byte `AccountType` discriminator at offset 165 (`1` = Mint), then a TLV
//! (type, length, value) extension stream starting at offset 166:
//!   [type: u16 LE][length: u16 LE][value: `length` bytes], repeated to EOF.
//!
//! Extension type discriminants used here are from `spl_token_2022::
//! extension::ExtensionType` (a stable, published enum):
//!   1  = TransferFeeConfig
//!   12 = PermanentDelegate
//!   14 = TransferHook
//!
//! CAVEAT: the offsets and extension layout above are transcribed from the
//! spl-token-2022 spec, not captured from a live mint (this environment has
//! no network access to verify against a real Token-2022 account). Before
//! trusting this against real funds, confirm it against a
//! `getAccountInfo` response for a known Token-2022 mint with each
//! extension.

use serde::{Deserialize, Serialize};

/// Fixed size of a legacy SPL Token `Mint` account, and the point at which a
/// Token-2022 mint's padded base region ends.
pub const MINT_BASE_LEN: usize = 82;

/// Token-2022 pads the base `Mint` out to this length (== token account
/// size) before the `AccountType` discriminator byte.
const TOKEN2022_ACCOUNT_TYPE_OFFSET: usize = 165;

/// First byte of the TLV extension stream (right after the `AccountType`
/// discriminator byte at `TOKEN2022_ACCOUNT_TYPE_OFFSET`).
const TOKEN2022_TLV_START: usize = TOKEN2022_ACCOUNT_TYPE_OFFSET + 1;

const EXT_TRANSFER_FEE_CONFIG: u16 = 1;
const EXT_PERMANENT_DELEGATE: u16 = 12;
const EXT_TRANSFER_HOOK: u16 = 14;

/// `TransferFeeConfig`'s value is a fixed 108-byte struct: two Pubkeys (64
/// bytes) + withheld_amount (8 bytes) + older_transfer_fee (18 bytes) +
/// newer_transfer_fee (18 bytes). Each `TransferFee` is
/// `{ epoch: u64, maximum_fee: u64, transfer_fee_basis_points: u16 }`, so the
/// presently-active ("newer") fee's basis points sit in the last 2 bytes.
const TRANSFER_FEE_CONFIG_LEN: usize = 108;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MintParseError(pub String);

impl std::fmt::Display for MintParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "could not parse mint account: {}", self.0)
    }
}

/// Facts pulled directly out of a mint account's raw bytes, before
/// holder-concentration (a separate RPC call) is folded in.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ParsedMint {
    pub mint_authority_active: bool,
    pub freeze_authority_active: bool,
    pub supply: u64,
    pub has_permanent_delegate: bool,
    pub has_transfer_hook: bool,
    pub transfer_fee_bps: u16,
}

/// Parse a mint account's raw bytes (as returned by `getAccountInfo`, after
/// base64-decoding). Fails closed on anything shorter than a legacy mint
/// (82 bytes) rather than guessing.
pub fn parse_mint_account(data: &[u8]) -> Result<ParsedMint, MintParseError> {
    if data.len() < MINT_BASE_LEN {
        return Err(MintParseError(format!(
            "mint account too short: {} bytes, expected at least {MINT_BASE_LEN}",
            data.len()
        )));
    }

    let mint_authority_tag = u32::from_le_bytes(data[0..4].try_into().unwrap());
    let supply = u64::from_le_bytes(data[36..44].try_into().unwrap());
    let freeze_authority_tag = u32::from_le_bytes(data[46..50].try_into().unwrap());

    let mut parsed = ParsedMint {
        mint_authority_active: mint_authority_tag != 0,
        freeze_authority_active: freeze_authority_tag != 0,
        supply,
        ..Default::default()
    };

    if data.len() > TOKEN2022_TLV_START {
        scan_extensions(&data[TOKEN2022_TLV_START..], &mut parsed);
    }

    Ok(parsed)
}

/// Walk a Token-2022 TLV extension stream, updating `parsed` for the
/// extension types this plugin cares about. Unknown types (and trailing
/// zero padding, which reads as type 0) are skipped, not treated as errors —
/// new extension types roll out over time and must not make this fail
/// closed on an otherwise-legitimate mint.
fn scan_extensions(tlv: &[u8], parsed: &mut ParsedMint) {
    let mut offset = 0usize;
    while offset + 4 <= tlv.len() {
        let ext_type = u16::from_le_bytes(tlv[offset..offset + 2].try_into().unwrap());
        let ext_len = u16::from_le_bytes(tlv[offset + 2..offset + 4].try_into().unwrap()) as usize;
        let value_start = offset + 4;
        let value_end = value_start + ext_len;
        if value_end > tlv.len() {
            // Truncated trailer: stop rather than read past the buffer.
            break;
        }
        let value = &tlv[value_start..value_end];
        match ext_type {
            EXT_PERMANENT_DELEGATE => parsed.has_permanent_delegate = true,
            EXT_TRANSFER_HOOK => parsed.has_transfer_hook = true,
            EXT_TRANSFER_FEE_CONFIG if value.len() == TRANSFER_FEE_CONFIG_LEN => {
                let bps_offset = TRANSFER_FEE_CONFIG_LEN - 2;
                parsed.transfer_fee_bps =
                    u16::from_le_bytes(value[bps_offset..].try_into().unwrap());
            }
            _ => {}
        }
        offset = value_end;
    }
}

/// Top holder's share of supply, as a percentage. `supply == 0` (a burned or
/// malformed mint) reports 0.0 rather than dividing by zero.
pub fn holder_share_pct(largest_holder_amount: u64, supply: u64) -> f64 {
    if supply == 0 {
        return 0.0;
    }
    (largest_holder_amount as f64 / supply as f64) * 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn legacy_mint_bytes(mint_authority: bool, freeze_authority: bool, supply: u64) -> Vec<u8> {
        let mut data = vec![0u8; MINT_BASE_LEN];
        data[0..4].copy_from_slice(&(mint_authority as u32).to_le_bytes());
        data[36..44].copy_from_slice(&supply.to_le_bytes());
        data[46..50].copy_from_slice(&(freeze_authority as u32).to_le_bytes());
        data
    }

    /// Padded base + AccountType(1) + a caller-supplied TLV stream.
    fn token2022_mint_bytes(base: Vec<u8>, tlv: &[u8]) -> Vec<u8> {
        let mut data = base;
        data.resize(TOKEN2022_ACCOUNT_TYPE_OFFSET, 0);
        data.push(1); // AccountType::Mint
        data.extend_from_slice(tlv);
        data
    }

    fn tlv_entry(ext_type: u16, value: &[u8]) -> Vec<u8> {
        let mut entry = Vec::new();
        entry.extend_from_slice(&ext_type.to_le_bytes());
        entry.extend_from_slice(&(value.len() as u16).to_le_bytes());
        entry.extend_from_slice(value);
        entry
    }

    #[test]
    fn rejects_input_shorter_than_a_legacy_mint() {
        let data = vec![0u8; 40];
        assert!(parse_mint_account(&data).is_err());
    }

    #[test]
    fn legacy_mint_with_no_authorities() {
        let data = legacy_mint_bytes(false, false, 1_000_000);
        let parsed = parse_mint_account(&data).unwrap();
        assert!(!parsed.mint_authority_active);
        assert!(!parsed.freeze_authority_active);
        assert_eq!(parsed.supply, 1_000_000);
        assert!(!parsed.has_permanent_delegate);
        assert!(!parsed.has_transfer_hook);
        assert_eq!(parsed.transfer_fee_bps, 0);
    }

    #[test]
    fn legacy_mint_with_active_mint_authority() {
        let data = legacy_mint_bytes(true, false, 500);
        let parsed = parse_mint_account(&data).unwrap();
        assert!(parsed.mint_authority_active);
    }

    #[test]
    fn legacy_mint_with_active_freeze_authority() {
        let data = legacy_mint_bytes(false, true, 500);
        let parsed = parse_mint_account(&data).unwrap();
        assert!(parsed.freeze_authority_active);
    }

    #[test]
    fn token2022_mint_with_no_extensions_reads_like_legacy() {
        let base = legacy_mint_bytes(false, false, 42);
        let data = token2022_mint_bytes(base, &[]);
        let parsed = parse_mint_account(&data).unwrap();
        assert_eq!(parsed.supply, 42);
        assert!(!parsed.has_permanent_delegate);
        assert!(!parsed.has_transfer_hook);
    }

    #[test]
    fn detects_permanent_delegate_extension() {
        let base = legacy_mint_bytes(false, false, 1);
        let tlv = tlv_entry(EXT_PERMANENT_DELEGATE, &[0u8; 32]);
        let data = token2022_mint_bytes(base, &tlv);
        let parsed = parse_mint_account(&data).unwrap();
        assert!(parsed.has_permanent_delegate);
    }

    #[test]
    fn detects_transfer_hook_extension() {
        let base = legacy_mint_bytes(false, false, 1);
        let tlv = tlv_entry(EXT_TRANSFER_HOOK, &[0u8; 32]);
        let data = token2022_mint_bytes(base, &tlv);
        let parsed = parse_mint_account(&data).unwrap();
        assert!(parsed.has_transfer_hook);
    }

    #[test]
    fn extracts_transfer_fee_bps_from_the_active_fee() {
        let base = legacy_mint_bytes(false, false, 1);
        let mut fee_value = vec![0u8; TRANSFER_FEE_CONFIG_LEN];
        fee_value[TRANSFER_FEE_CONFIG_LEN - 2..].copy_from_slice(&250u16.to_le_bytes());
        let tlv = tlv_entry(EXT_TRANSFER_FEE_CONFIG, &fee_value);
        let data = token2022_mint_bytes(base, &tlv);
        let parsed = parse_mint_account(&data).unwrap();
        assert_eq!(parsed.transfer_fee_bps, 250);
    }

    #[test]
    fn scans_multiple_extensions_in_one_stream() {
        let base = legacy_mint_bytes(false, false, 1);
        let mut tlv = tlv_entry(EXT_PERMANENT_DELEGATE, &[0u8; 32]);
        tlv.extend(tlv_entry(EXT_TRANSFER_HOOK, &[1u8; 32]));
        let data = token2022_mint_bytes(base, &tlv);
        let parsed = parse_mint_account(&data).unwrap();
        assert!(parsed.has_permanent_delegate);
        assert!(parsed.has_transfer_hook);
    }

    #[test]
    fn truncated_tlv_trailer_does_not_panic() {
        let base = legacy_mint_bytes(false, false, 1);
        // Claims a 32-byte value but only supplies 4 -- must stop cleanly.
        let mut tlv = Vec::new();
        tlv.extend_from_slice(&EXT_PERMANENT_DELEGATE.to_le_bytes());
        tlv.extend_from_slice(&32u16.to_le_bytes());
        tlv.extend_from_slice(&[0u8; 4]);
        let data = token2022_mint_bytes(base, &tlv);
        let parsed = parse_mint_account(&data).unwrap();
        assert!(!parsed.has_permanent_delegate);
    }

    #[test]
    fn unknown_extension_type_is_skipped_not_rejected() {
        let base = legacy_mint_bytes(false, false, 1);
        let tlv = tlv_entry(999, &[0u8; 4]);
        let data = token2022_mint_bytes(base, &tlv);
        assert!(parse_mint_account(&data).is_ok());
    }

    #[test]
    fn holder_share_pct_computes_ratio() {
        assert_eq!(holder_share_pct(50, 100), 50.0);
        assert_eq!(holder_share_pct(0, 100), 0.0);
    }

    #[test]
    fn holder_share_pct_zero_supply_is_zero_not_nan() {
        assert_eq!(holder_share_pct(10, 0), 0.0);
    }
}
