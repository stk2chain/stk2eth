use sha3::{Digest, Keccak256};

/// Compute the 4-byte function selector for an ABI signature like
/// `"transfer(address,uint256)"`. Pure, no SpacetimeDB dependency.
pub fn keccak_selector(signature: &str) -> [u8; 4] {
    let hash = Keccak256::digest(signature.as_bytes());
    let mut out = [0u8; 4];
    out.copy_from_slice(&hash[..4]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transfer_selector_matches_known_vector() {
        assert_eq!(keccak_selector("transfer(address,uint256)"),
                   [0xa9, 0x05, 0x9c, 0xbb]);
    }

    #[test]
    fn balance_of_selector_matches_known_vector() {
        assert_eq!(keccak_selector("balanceOf(address)"),
                   [0x70, 0xa0, 0x82, 0x31]);
    }

    #[test]
    fn swap_exact_tokens_selector_matches_known_vector() {
        assert_eq!(
            keccak_selector(
                "swapExactTokensForTokens(uint256,uint256,address[],address,uint256)"
            ),
            [0x38, 0xed, 0x17, 0x39]
        );
    }
}
