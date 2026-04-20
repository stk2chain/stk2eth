// broadcaster/src/error.rs
use alloy::primitives::U256;

#[derive(Debug, thiserror::Error)]
pub enum BroadcasterError {
    #[error("insufficient burner balance: have {have} need {need}")]
    InsufficientBalance { have: U256, need: U256 },
    #[error("invalid recipient address: {0}")]
    InvalidRecipient(String),
    #[error("tx reverted on chain: {0}")]
    Reverted(String),
    #[error("burner derivation failed: {0}")]
    DerivationFailed(String),
    #[error("RPC unavailable: {0}")]
    RpcTransient(String),
    #[error("rate limited by provider")]
    RateLimited,
    #[error("nonce already used")]
    NonceAlreadyUsed,
    #[error("SpacetimeDB reducer rejected: {0}")]
    ReducerRejected(String),
    #[error("config error: {0}")]
    Config(String),
}

impl BroadcasterError {
    pub fn is_terminal(&self) -> bool {
        use BroadcasterError::*;
        matches!(self,
            InsufficientBalance { .. } | InvalidRecipient(_) | Reverted(_) | DerivationFailed(_))
    }

    pub fn is_retryable(&self) -> bool {
        use BroadcasterError::*;
        matches!(self, RpcTransient(_) | RateLimited | NonceAlreadyUsed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classification_matrix() {
        let terminal = [
            BroadcasterError::InsufficientBalance { have: U256::ZERO, need: U256::from(1) },
            BroadcasterError::InvalidRecipient("x".into()),
            BroadcasterError::Reverted("x".into()),
            BroadcasterError::DerivationFailed("x".into()),
        ];
        for e in &terminal {
            assert!(e.is_terminal(), "{e:?} should be terminal");
            assert!(!e.is_retryable(), "{e:?} should not be retryable");
        }

        let retryable = [
            BroadcasterError::RpcTransient("x".into()),
            BroadcasterError::RateLimited,
            BroadcasterError::NonceAlreadyUsed,
        ];
        for e in &retryable {
            assert!(e.is_retryable(), "{e:?} should be retryable");
            assert!(!e.is_terminal(), "{e:?} should not be terminal");
        }

        let neither = [
            BroadcasterError::ReducerRejected("x".into()),
            BroadcasterError::Config("x".into()),
        ];
        for e in &neither {
            assert!(!e.is_terminal(), "{e:?} should not be terminal");
            assert!(!e.is_retryable(), "{e:?} should not be retryable");
        }
    }
}
