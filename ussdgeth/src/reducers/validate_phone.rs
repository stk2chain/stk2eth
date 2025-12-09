// ussdgeth/src/reducers/validate_phone.rs
use crate::{phone_wallet, PhoneWallet};
use spacetimedb::{reducer, ReducerContext, Table};
use std::str;

/// Validate E.164 phone numbers *without* bringing in regex.
/// Rules implemented:
/// - Must start with `+`
/// - Digits only after `+`
/// - Minimum digits after +: 8 (common realistic minimum)
/// - Maximum digits after +: 15 (E.164 max)
/// - First digit after + must be 1-9 (country codes don't start with 0)
///
/// This keeps validation strict but permissive enough for global numbers.
fn is_valid_e164(phone: &str) -> bool {
    // must start with +
    let bytes = phone.as_bytes();
    if bytes.is_empty() || bytes[0] != b'+' {
        return false;
    }

    let digits = &phone[1..];

    // length constraints: 8..=15 digits (after +)
    let len = digits.len();
    if !(8..=15).contains(&len) {
        return false;
    }

    // first digit must be 1-9
    let first = digits.as_bytes()[0];
    if !(b'1'..=b'9').contains(&first) {
        return false;
    }

    // all must be digits
    if !digits.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }

    true
}

/// Simple Ethereum address validator:
/// - must start with "0x"
/// - length 42 (0x + 40 hex chars)
/// - all hex characters after 0x
fn is_valid_eth_address(addr: &str) -> bool {
    let a = addr.as_bytes();
    if a.len() != 42 {
        return false;
    }
    if a[0] != b'0' || a[1] != b'x' {
        return false;
    }
    // check hex digits
    addr[2..].chars().all(|c| c.is_ascii_hexdigit())
}

/// Reducer: map phone -> wallet (upsert)
/// It validates phone (E.164) and wallet (ETH) and then upserts PhoneWallet table.
#[reducer]
pub fn map_phone_to_wallet(ctx: &ReducerContext, phone_number: String, wallet_address: String) {
    // Validate phone
    if !is_valid_e164(&phone_number) {
        log::warn!(
            "map_phone_to_wallet: invalid phone number '{}'",
            phone_number
        );
        return;
    }

    // Validate wallet
    if !is_valid_eth_address(&wallet_address) {
        log::warn!(
            "map_phone_to_wallet: invalid wallet address '{}'",
            wallet_address
        );
        return;
    }

    // Upsert into phone_wallet table
    if let Some(existing) = ctx
        .db
        .phone_wallet()
        .phone_number()
        .find(phone_number.clone())
    {
        // update wallet_address + updated_at
        let updated = PhoneWallet {
            wallet_address: wallet_address.clone(),
            updated_at: ctx.timestamp,
            ..existing
        };
        ctx.db.phone_wallet().phone_number().update(updated);
        log::info!(
            "map_phone_to_wallet: updated mapping {} -> {}",
            phone_number,
            wallet_address
        );
    } else {
        // insert new mapping
        ctx.db.phone_wallet().insert(PhoneWallet {
            phone_number: phone_number.clone(),
            wallet_address: wallet_address.clone(),
            created_at: ctx.timestamp,
            updated_at: ctx.timestamp,
        });
        log::info!(
            "map_phone_to_wallet: inserted mapping {} -> {}",
            phone_number,
            wallet_address
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_e164_good() {
        let good = vec![
            "+254700000000",
            "+14155552671",
            "+447911123456",
            "+33123456789",
            "+918123456789",
        ];
        for p in good {
            assert!(is_valid_e164(p), "expected '{}' to be valid E.164", p);
        }
    }

    #[test]
    fn test_is_valid_e164_bad() {
        let bad = vec![
            "254700000000",      // missing plus
            "+0012345678",       // leading zero in country code
            "+123",              // too short
            "+1234567890123456", // too long (>15 digits)
            "+2547abc0000",      // letters
            "++254700000000",    // double plus
            "",                  // empty
            "+0",                // too short and invalid first digit
        ];
        for p in bad {
            assert!(!is_valid_e164(p), "expected '{}' to be invalid E.164", p);
        }
    }

    #[test]
    fn test_is_valid_eth_address() {
        assert!(is_valid_eth_address(
            "0x0123456789abcdef0123456789abcdef01234567"
        ));
        assert!(!is_valid_eth_address("0xG123")); // invalid hex char
        assert!(!is_valid_eth_address(
            "0123456789abcdef0123456789abcdef01234567"
        )); // missing 0x
        assert!(!is_valid_eth_address("0x0123")); // too short
    }
}
