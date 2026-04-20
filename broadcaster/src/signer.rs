// broadcaster/src/signer.rs
use crate::error::BroadcasterError;
use alloy::signers::local::{LocalSigner, PrivateKeySigner};
use secrecy::{ExposeSecret, SecretString};
use std::path::Path;

pub fn load_operator_signer(
    keystore_path: &Path,
    passphrase: &SecretString,
) -> Result<PrivateKeySigner, BroadcasterError> {
    let signer = LocalSigner::decrypt_keystore(keystore_path, passphrase.expose_secret())
        .map_err(|e| BroadcasterError::Config(format!("keystore decrypt: {e}")))?;
    Ok(signer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/test-keystore.json")
    }

    #[test]
    fn decrypts_with_correct_passphrase() {
        let signer = load_operator_signer(
            &fixture_path(),
            &SecretString::new("test-pw".into()),
        ).expect("should decrypt with correct passphrase");
        let addr = format!("{:#x}", signer.address());
        assert_eq!(addr.len(), 42, "expected 0x-prefixed address");
    }

    #[test]
    fn wrong_passphrase_errors() {
        let err = load_operator_signer(
            &fixture_path(),
            &SecretString::new("wrong-pw".into()),
        ).unwrap_err();
        assert!(matches!(err, BroadcasterError::Config(ref m) if m.contains("keystore")),
            "expected keystore decrypt error, got {err:?}");
    }
}
