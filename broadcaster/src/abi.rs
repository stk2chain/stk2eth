// broadcaster/src/abi.rs
use alloy::sol;

sol! {
    #[sol(rpc)]
    contract Burner7702 {
        function execute(address to, uint256 value, bytes calldata data) external payable;
    }
}

/// Encode a call to `execute(to, value, data)`. Returns the calldata bytes
/// that get placed in the outer EIP-7702 tx's `input` field.
pub fn encode_execute(to: alloy::primitives::Address, value: alloy::primitives::U256, data: alloy::primitives::Bytes) -> alloy::primitives::Bytes {
    use alloy::sol_types::SolCall;
    Burner7702::executeCall { to, value, data }.abi_encode().into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::primitives::{address, bytes, keccak256, U256};

    /// Known-good selector for `execute(address,uint256,bytes)`.
    const EXECUTE_SELECTOR: [u8; 4] = [0xb6, 0x1d, 0x27, 0xf6];

    #[test]
    fn execute_selector_matches_keccak() {
        let hash = keccak256(b"execute(address,uint256,bytes)");
        let selector: [u8; 4] = hash[..4].try_into().unwrap();
        assert_eq!(selector, EXECUTE_SELECTOR, "keccak selector mismatch");
    }

    #[test]
    fn sol_macro_produces_same_selector() {
        use alloy::sol_types::SolCall;
        let call = Burner7702::executeCall {
            to: address!("0000000000000000000000000000000000000001"),
            value: U256::from(1u64),
            data: bytes!(""),
        };
        let encoded = call.abi_encode();
        let selector: [u8; 4] = encoded[..4].try_into().unwrap();
        assert_eq!(selector, EXECUTE_SELECTOR);
    }

    #[test]
    fn encode_execute_roundtrip() {
        use alloy::sol_types::SolCall;
        let to = address!("1234567890123456789012345678901234567890");
        let value = U256::from(10_000_000_000_000_000u64); // 0.01 ETH
        let data = bytes!("");
        let encoded = encode_execute(to, value, data.clone());
        assert_eq!(&encoded[..4], &EXECUTE_SELECTOR);
        // Decode roundtrip
        let decoded = Burner7702::executeCall::abi_decode(&encoded, true).unwrap();
        assert_eq!(decoded.to, to);
        assert_eq!(decoded.value, value);
        assert_eq!(decoded.data, data);
    }
}
