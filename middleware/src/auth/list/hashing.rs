use sha3::{Digest, Keccak256};
use sha2::Sha256;
use rlp::{RlpStream, Encodable};
use k256::ecdsa::{RecoveryId, Signature, VerifyingKey};
use std::env;

fn get_permit2_address() -> String {
    env::var("PERMIT2_7702_ADDRESS")
        .unwrap_or_else(|_| "0x000000000022D473030F116dDEE9F6B43aC78BA3".to_string())
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

fn normalize_phone_number(phone: &str) -> String {
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

fn recover_address(r: &[u8; 32], s: &[u8; 32], v: u8, msg_hash: &[u8; 32]) -> Option<[u8; 20]> {
    // Construct signature from r and s
    let mut sig_bytes = [0u8; 64];
    sig_bytes[..32].copy_from_slice(r);
    sig_bytes[32..].copy_from_slice(s);
    
    let signature = Signature::from_slice(&sig_bytes).ok()?;
    let recovery_id = RecoveryId::from_byte(v - 27)?;
    
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

fn nick_auth_7702(mut r: [u8; 32], s: [u8; 32], v: u8, msg_hash: &[u8; 32]) -> ([u8; 20], [u8; 32]) {
    log::info!("Searching for valid signature with phone-derived salt...");
    let mut attempts = 0u64;
    
    loop {
        attempts += 1;
        
        if let Some(address) = recover_address(&r, &s, v, msg_hash) {
            log::info!("✓ Found valid signature after {} attempts", attempts);
            return (address, r);
        }
        
        let mut carry = 1u16;
        for i in (0..32).rev() {
            let sum = r[i] as u16 + carry;
            r[i] = sum as u8;
            carry = sum >> 8;
            if carry == 0 {
                break;
            }
        }
        
        if attempts % 1000 == 0 {
            log::info!("  ... {} attempts", attempts);
        }
    }
}

pub fn create_phone_permit2_authorization(
    phone_number: &str,
    chain_id: u64,
    nonce: u64,
    user_salt: Option<&str>,
    delegate_to: Option<&str>,
) -> ([u8; 20], SignedAuthorization) {
    let binding = get_permit2_address();
    let delegate_address = delegate_to.unwrap_or(&binding);
    let phone_salt = phone_to_salt(phone_number, user_salt);
    let msg_hash = hash_auth7702_message(chain_id, delegate_address, nonce);
    
    let mut r = [0u8; 32];
    r.copy_from_slice(&msg_hash);
    let s = phone_salt;
    let v = 27;
    
    let (authority_address, final_r) = nick_auth_7702(r, s, v, &msg_hash);
    
    let mut addr_bytes = [0u8; 20];
    let delegate_bytes = hex::decode(&delegate_address[2..]).expect("Invalid address");
    addr_bytes.copy_from_slice(&delegate_bytes);
    
    let signed_auth = SignedAuthorization {
        chain_id,
        address: addr_bytes,
        nonce,
        v,
        r: final_r,
        s,
    };
    
    (authority_address, signed_auth)
}