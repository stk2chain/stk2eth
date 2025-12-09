use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose, Engine as _};
use hex::FromHex;
use rand::rngs::OsRng;
use rand::RngCore;
use std::convert::TryFrom;

#[allow(dead_code)]
const NONCE_LEN: usize = 12; // AES-GCM recommended nonce size

#[allow(dead_code)]
fn key_from_hex(hex_str: &str) -> Result<[u8; 32]> {
    let bytes = Vec::from_hex(hex_str).map_err(|e| anyhow!("Invalid hex MASTER_KEY: {:?}", e))?;
    if bytes.len() != 32 {
        return Err(anyhow!(
            "MASTER_KEY must be 32 bytes (64 hex chars). Provided length: {}",
            bytes.len()
        ));
    }
    let mut key = [0u8; 32];
    key.copy_from_slice(&bytes);
    Ok(key)
}

#[allow(dead_code)]
pub fn encrypt_blob(master_key_hex: &str, plaintext: &[u8]) -> Result<String> {
    let key_bytes = key_from_hex(master_key_hex)?;
    let cipher = Aes256Gcm::new_from_slice(&key_bytes)?;

    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::try_from(&nonce_bytes[..])?;

    let ciphertext = cipher.encrypt(&nonce, plaintext)?;

    let mut out = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);

    Ok(general_purpose::STANDARD.encode(&out))
}

#[allow(dead_code)]
pub fn decrypt_blob(master_key_hex: &str, b64_blob: &str) -> Result<Vec<u8>> {
    let key_bytes = key_from_hex(master_key_hex)?;
    let cipher = Aes256Gcm::new_from_slice(&key_bytes)?;

    let raw = general_purpose::STANDARD.decode(b64_blob)?;

    if raw.len() < NONCE_LEN + 16 {
        return Err(anyhow!("ciphertext too short"));
    }

    let (nonce_bytes, ciphertext) = raw.split_at(NONCE_LEN);
    let nonce = Nonce::try_from(nonce_bytes)?;

    let plaintext = cipher.decrypt(&nonce, ciphertext)?;
    Ok(plaintext)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip() {
        let key_hex = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let plaintext = b"hello-secret-key-0x123";

        let enc = encrypt_blob(key_hex, plaintext).expect("encrypt");
        let dec = decrypt_blob(key_hex, &enc).expect("decrypt");
        assert_eq!(dec, plaintext);
    }
}
