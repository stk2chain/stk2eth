use crate::{user_pin, UserPIN};
use sha2::{Digest, Sha256};
use spacetimedb::{reducer, ReducerContext, Table};

const MAX_PIN_ATTEMPTS: u32 = 3;
const MIN_PIN_LENGTH: usize = 4;
const MAX_PIN_LENGTH: usize = 6;

pub fn hash_pin(pin: &str, salt: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(format!("{}{}", pin, salt));
    format!("{:x}", hasher.finalize())
}

pub fn generate_salt() -> String {
    use std::time::SystemTime;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let nanos = now.as_nanos();
    format!("{:x}", nanos)
}

fn validate_pin_format(pin: &str) -> bool {
    if pin.len() < MIN_PIN_LENGTH || pin.len() > MAX_PIN_LENGTH {
        return false;
    }
    pin.chars().all(|c| c.is_ascii_digit())
}

#[reducer]
pub fn create_user_pin(ctx: &ReducerContext, phone_number: String, pin: String) {
    if !validate_pin_format(&pin) {
        log::error!("Invalid PIN format for phone number: {}", phone_number);
        return;
    }

    if ctx
        .db
        .user_pin()
        .phone_number()
        .find(phone_number.clone())
        .is_some()
    {
        log::warn!("PIN already exists for phone number: {}", phone_number);
        return;
    }

    let salt = generate_salt();
    let pin_hash = hash_pin(&pin, &salt);

    ctx.db.user_pin().insert(UserPIN {
        phone_number: phone_number.clone(),
        pin_hash,
        salt,
        attempts: 0,
        locked: false,
        created_at: ctx.timestamp,
        updated_at: ctx.timestamp,
    });

    log::info!(
        "PIN created successfully for phone number: {}",
        phone_number
    );
}

#[reducer]
pub fn validate_pin(ctx: &ReducerContext, session_id: String, phone_number: String, pin: String) {
    if !validate_pin_format(&pin) {
        log::error!("Invalid PIN format for phone number: {}", phone_number);
        return;
    }

    let user_pin = match ctx.db.user_pin().phone_number().find(phone_number.clone()) {
        Some(p) => p,
        None => {
            log::warn!("User not found for phone number: {}", phone_number);
            return;
        }
    };

    if user_pin.locked {
        log::warn!("Account locked for phone number: {}", phone_number);
        return;
    }

    let computed_hash = hash_pin(&pin, &user_pin.salt);

    if computed_hash == user_pin.pin_hash {
        ctx.db.user_pin().phone_number().update(UserPIN {
            attempts: 0,
            updated_at: ctx.timestamp,
            ..user_pin
        });

        log::info!(
            "PIN validation successful for session: {} phone: {}",
            session_id,
            phone_number
        );
    } else {
        let new_attempts = user_pin.attempts + 1;
        let locked = new_attempts >= MAX_PIN_ATTEMPTS;

        ctx.db.user_pin().phone_number().update(UserPIN {
            attempts: new_attempts,
            locked,
            updated_at: ctx.timestamp,
            ..user_pin
        });

        if locked {
            log::warn!(
                "Account locked after {} failed attempts for phone: {}",
                MAX_PIN_ATTEMPTS,
                phone_number
            );
        } else {
            log::warn!(
                "Invalid PIN attempt {} of {} for phone: {}",
                new_attempts,
                MAX_PIN_ATTEMPTS,
                phone_number
            );
        }
    }
}

#[reducer]
pub fn reset_pin_attempts(ctx: &ReducerContext, phone_number: String) {
    let user_pin = match ctx.db.user_pin().phone_number().find(phone_number.clone()) {
        Some(p) => p,
        None => {
            log::warn!("User not found for phone number: {}", phone_number);
            return;
        }
    };

    ctx.db.user_pin().phone_number().update(UserPIN {
        attempts: 0,
        locked: false,
        updated_at: ctx.timestamp,
        ..user_pin
    });

    log::info!(
        "PIN attempts reset successfully for phone number: {}",
        phone_number
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pin_format_validation() {
        assert!(validate_pin_format("1234"));
        assert!(validate_pin_format("123456"));
        assert!(validate_pin_format("0000"));

        assert!(!validate_pin_format("123"));
        assert!(!validate_pin_format("1234567"));
        assert!(!validate_pin_format("12a4"));
        assert!(!validate_pin_format("abcd"));
        assert!(!validate_pin_format(""));
    }

    #[test]
    fn test_hash_pin_deterministic() {
        let pin = "1234";
        let salt = "test_salt";

        let hash1 = hash_pin(pin, salt);
        let hash2 = hash_pin(pin, salt);

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_pin_different_for_different_pins() {
        let salt = "test_salt";

        let hash1 = hash_pin("1234", salt);
        let hash2 = hash_pin("5678", salt);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hash_pin_different_for_different_salts() {
        let pin = "1234";

        let hash1 = hash_pin(pin, "salt1");
        let hash2 = hash_pin(pin, "salt2");

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_generate_salt_unique() {
        let salt1 = generate_salt();
        std::thread::sleep(std::time::Duration::from_nanos(1));
        let salt2 = generate_salt();

        assert_ne!(salt1, salt2);
    }

    #[test]
    fn test_hash_pin_produces_hex_string() {
        let hash = hash_pin("1234", "salt");
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(hash.len(), 64);
    }
}
