use sha3::{Digest, Keccak256};
use sha2::Sha256;
use rlp::{RlpStream, Encodable};
use k256::ecdsa::{RecoveryId, Signature, VerifyingKey};
use std::env;

const NICK_MAX_ATTEMPTS: u64 = 1_000_000;

#[derive(Debug, Clone, PartialEq)]
pub enum AuthGenError {
    DerivationExhausted { attempts: u64 },
    InvalidDelegateAddress,
}

impl std::fmt::Display for AuthGenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthGenError::DerivationExhausted { attempts } =>
                write!(f, "Nick's Method exhausted after {} attempts", attempts),
            AuthGenError::InvalidDelegateAddress =>
                write!(f, "delegate address must be 0x-prefixed 20-byte hex"),
        }
    }
}

fn get_permit2_address() -> String {
    env::var("PERMIT2_7702_ADDRESS")
        .unwrap_or_else(|_| "0x2fDdd08Fb3e796bc68B1a26f3D1a61b073860fEf".to_string())
}

#[derive(Clone)]
struct Authorization {
    chain_id: u64,
    address: [u8; 20],
    nonce: u64,
}

impl Encodable for Authorization {
    fn rlp_append(&self, s: &mut RlpStream) {
        s.begin_list(3);
        s.append(&self.chain_id);
        s.append(&self.address.as_ref());
        s.append(&self.nonce);
    }
}

#[derive(Clone)]
pub struct SignedAuthorization {
    pub chain_id: u64,
    pub address: [u8; 20],
    pub nonce: u64,
    pub v: u8,
    pub r: [u8; 32],
    pub s: [u8; 32],
}

// secp256k1 curve order
const SECP256K1_N: [u8; 32] = [
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFE,
    0xBA, 0xAE, 0xDC, 0xE6, 0xAF, 0x48, 0xA0, 0x3B,
    0xBF, 0xD2, 0x5E, 0x8C, 0xD0, 0x36, 0x41, 0x41,
];

// secp256k1n / 2
const SECP256K1_N_HALF: [u8; 32] = [
    0x7F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0x5D, 0x57, 0x6E, 0x73, 0x57, 0xA4, 0x50, 0x1D,
    0xDF, 0xE9, 0x2F, 0x46, 0x68, 0x1B, 0x20, 0xA0,
];

pub fn normalize_phone_number(phone: &str) -> String {
    phone.chars().filter(|c| c.is_ascii_digit()).collect()
}

fn phone_to_salt(phone_number: &str, user_salt: Option<&str>) -> [u8; 32] {
    let normalized_phone = normalize_phone_number(phone_number);
    log::info!("Normalized phone: {}", normalized_phone);
    
    let combined = if let Some(salt) = user_salt {
        format!("{}||{}", normalized_phone, salt)
    } else {
        normalized_phone
    };
    
    let mut hasher = Sha256::new();
    hasher.update(combined.as_bytes());
    hasher.finalize().into()
}

fn hash_auth7702_message(chain_id: u64, delegate_to: &str, nonce: u64) -> [u8; 32] {
    let address = hex::decode(&delegate_to[2..]).expect("Invalid address");
    let mut addr_bytes = [0u8; 20];
    addr_bytes.copy_from_slice(&address);
    
    let auth = Authorization {
        chain_id,
        address: addr_bytes,
        nonce,
    };
    
    let mut stream = RlpStream::new();
    auth.rlp_append(&mut stream);
    let encoded = stream.out();
    
    let mut hasher = Keccak256::new();
    hasher.update(&[0x05]);
    hasher.update(&encoded);
    hasher.finalize().into()
}

// Compare two 32-byte arrays as big-endian integers
// Returns true if a < b
fn is_less_than(a: &[u8; 32], b: &[u8; 32]) -> bool {
    for i in 0..32 {
        if a[i] < b[i] {
            return true;
        } else if a[i] > b[i] {
            return false;
        }
    }
    false // equal
}

// Normalize s value to be in the lower half (s < secp256k1n/2)
// If s > secp256k1n/2, return (n - s) and flip v
fn normalize_s(s: [u8; 32], v: u8) -> ([u8; 32], u8) {
    if is_less_than(&s, &SECP256K1_N_HALF) || s == SECP256K1_N_HALF {
        // s is already in lower half
        return (s, v);
    }
    
    log::info!("S value too high, normalizing by computing (n - s)");
    
    // Compute n - s
    let mut result = [0u8; 32];
    let mut borrow = 0i16;
    
    for i in (0..32).rev() {
        let diff = SECP256K1_N[i] as i16 - s[i] as i16 - borrow;
        if diff < 0 {
            result[i] = (diff + 256) as u8;
            borrow = 1;
        } else {
            result[i] = diff as u8;
            borrow = 0;
        }
    }
    
    // Flip v (27 <-> 28 or 0 <-> 1)
    let new_v = if v == 27 { 28 } else if v == 28 { 27 } else if v == 0 { 1 } else { 0 };
    
    (result, new_v)
}

fn recover_address(r: &[u8; 32], s: &[u8; 32], v: u8, msg_hash: &[u8; 32]) -> Option<[u8; 20]> {
    // Construct signature from r and s
    let mut sig_bytes = [0u8; 64];
    sig_bytes[..32].copy_from_slice(r);
    sig_bytes[32..].copy_from_slice(s);
    
    let signature = Signature::from_slice(&sig_bytes).ok()?;
    let recovery_id = RecoveryId::from_byte(v.checked_sub(27)?)?;
    
    // Recover public key
    let recovered_key = VerifyingKey::recover_from_prehash(msg_hash, &signature, recovery_id).ok()?;
    
    // Get uncompressed public key bytes (65 bytes: 0x04 || x || y)
    let pubkey_bytes = recovered_key.to_encoded_point(false);
    let pubkey_uncompressed = pubkey_bytes.as_bytes();
    
    // Hash the public key (excluding the 0x04 prefix) with Keccak256
    let mut hasher = Keccak256::new();
    hasher.update(&pubkey_uncompressed[1..]); // Skip the 0x04 prefix
    let hash = hasher.finalize();
    
    // Take last 20 bytes as Ethereum address
    let mut address = [0u8; 20];
    address.copy_from_slice(&hash[12..]);
    Some(address)
}

fn nick_auth_7702(
    mut r: [u8; 32], s: [u8; 32], v: u8, msg_hash: &[u8; 32],
) -> Result<([u8; 20], [u8; 32], u8), AuthGenError> {
    let mut attempts = 0u64;
    while attempts < NICK_MAX_ATTEMPTS {
        attempts += 1;
        if let Some(address) = recover_address(&r, &s, v, msg_hash) {
            log::info!("Nick's Method derivation succeeded in {} attempts", attempts);
            return Ok((address, r, v));
        }
        let mut carry = 1u16;
        for i in (0..32).rev() {
            let sum = r[i] as u16 + carry;
            r[i] = sum as u8;
            carry = sum >> 8;
            if carry == 0 { break; }
        }
    }
    Err(AuthGenError::DerivationExhausted { attempts })
}

pub fn create_phone_permit2_authorization(
    phone_number: &str,
    chain_id: u64,
    nonce: u64,
    user_salt: Option<&str>,
    delegate_to: Option<&str>,
) -> Result<([u8; 20], SignedAuthorization), AuthGenError> {
    let binding = get_permit2_address();
    let delegate_address = delegate_to.unwrap_or(&binding);

    if !delegate_address.starts_with("0x") || delegate_address.len() != 42 {
        return Err(AuthGenError::InvalidDelegateAddress);
    }

    let phone_salt = phone_to_salt(phone_number, user_salt);
    let msg_hash = hash_auth7702_message(chain_id, delegate_address, nonce);

    let mut r = [0u8; 32];
    r.copy_from_slice(&msg_hash);

    let (s, v) = normalize_s(phone_salt, 27);
    let (authority_address, final_r, final_v) = nick_auth_7702(r, s, v, &msg_hash)?;

    let mut addr_bytes = [0u8; 20];
    let delegate_bytes = hex::decode(&delegate_address[2..])
        .map_err(|_| AuthGenError::InvalidDelegateAddress)?;
    addr_bytes.copy_from_slice(&delegate_bytes);

    let signed_auth = SignedAuthorization {
        chain_id,
        address: addr_bytes,
        nonce,
        v: final_v,
        r: final_r,
        s,
    };
    Ok((authority_address, signed_auth))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_valid_wallet_for_real_phone() {
        let phone = "+254712345678";
        let result = create_phone_permit2_authorization(
            phone,
            84532,
            0,
            None,
            Some("0x2fDdd08Fb3e796bc68B1a26f3D1a61b073860fEf"),
        );
        let (wallet, _signed) = result.expect("derivation must succeed within attempt bound");
        assert_ne!(wallet, [0u8; 20], "wallet address must be non-zero");
    }

    #[test]
    fn determinism_same_phone_same_wallet() {
        let phone = "+254712345678";
        let (w1, _) = create_phone_permit2_authorization(phone, 84532, 0, None, None).unwrap();
        let (w2, _) = create_phone_permit2_authorization(phone, 84532, 0, None, None).unwrap();
        assert_eq!(w1, w2);
    }
}