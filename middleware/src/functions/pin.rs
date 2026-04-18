use crate::ussd::session::{ussd_session, USSDSession};
use crate::auth::pin::tables::{user_pin, UserPIN};
use rand::Rng;
use sha3::{Digest, Keccak256};
use spacetimedb::{reducer, ReducerContext, Table, Timestamp};
use std::{time::Duration, env};
use subtle::ConstantTimeEq;
use dotenv::dotenv; 




const MAX_PIN_ATTEMPTS: u32 = 3;
const MIN_PIN_LENGTH: usize = 4;
const MAX_PIN_LENGTH: usize = 6;
const SALT_LENGTH: usize = 32;
const LOCKOUT_DURATION_MINUTES: u64 = 15;



fn is_all_digits(s: &str) -> bool {
    s.chars().all(|c| c.is_ascii_digit())
}

fn is_valid_pin_length(pin: &str) -> bool {
    if pin.len() < MIN_PIN_LENGTH || pin.len() > MAX_PIN_LENGTH {
        return false;
    }
    pin.chars().all(|c| c.is_ascii_digit())
}

fn is_weak_pin(pin: &str) -> bool {
    if pin.len() < MIN_PIN_LENGTH {
        return true;
    }

    let chars: Vec<char> = pin.chars().collect();

    let all_same = chars.windows(2).all(|w| w[0] == w[1]);
    if all_same {
        return true;
    }

    let is_sequential_ascending = chars
        .windows(2)
        .all(|w| w[1].to_digit(10).unwrap() == w[0].to_digit(10).unwrap() + 1);
    if is_sequential_ascending {
        return true;
    }

    let is_sequential_descending = chars
        .windows(2)
        .all(|w| w[0].to_digit(10).unwrap() == w[1].to_digit(10).unwrap() + 1);
    if is_sequential_descending {
        return true;
    }

    false
}

pub fn validate_pin_format(pin: &str, check_weak: bool) -> Result<(), String> {
    if !is_all_digits(pin) {
        return Err("*PIN must contain only digits".to_string());
    }

    if !is_valid_pin_length(pin) {
        return Err("*Invalid PIN length (4-6 digits)".to_string());
    }
    
    if check_weak && is_weak_pin(pin) {
        return Err("*Weak PIN (no sequential or repeated digits)".to_string());
    }
    Ok(())
}




pub fn hash_pin(pin: &str, phone_number: &str, user_salt: &str) -> String {
    let mut hasher = Keccak256::new();
    //TODO: eSIMRegistry Domain Separator for versioning
    dotenv().ok();
    let domain_separator = env::var("eSIMRegistry_DOMAIN_SEPARATOR").unwrap_or_else(|_| "0x866a5aba21966af95d6c7ab78eb2b2fc913915c28be3b9aa07cc04ff903e3f28".to_string());

    hasher.update(phone_number.as_bytes());
    hasher.update(pin.as_bytes());
    hasher.update(domain_separator.as_bytes());
    hasher.update(user_salt.as_bytes());
    format!("{:x}", hasher.finalize())
}

// pub fn generate_salt() -> String {
//     let mut rng = rand::thread_rng();
//     let salt_bytes: Vec<u8> = (0..SALT_LENGTH).map(|_| rng.gen()).collect();
//     salt_bytes
//         .iter()
//         .map(|b| format!("{:02x}", b))
//         .collect::<String>()
// }

fn constant_time_compare(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.as_bytes().ct_eq(b.as_bytes()).into()
}





fn is_rate_limited(user_pin: &UserPIN, current_time: Timestamp) -> bool {
    if let Some(lockout_until) = user_pin.lockout_until {
        if current_time < lockout_until {
            return true;
        }
    }
    false
}

fn calculate_lockout_time(current_time: Timestamp) -> Timestamp {
    let lockout_duration = Duration::from_secs(LOCKOUT_DURATION_MINUTES * 60);
    current_time + lockout_duration
}

// #[reducer]
// pub fn create_user_pin(ctx: &ReducerContext, phone_number: String, pin: String) {
//     if !validate_pin_format(&pin) {
//         log::error!(
//             "Invalid PIN format for phone number: {} (must be {}-{} digits)",
//             phone_number,
//             MIN_PIN_LENGTH,
//             MAX_PIN_LENGTH
//         );
//         return;
//     }

//     if is_weak_pin(&pin) {
//         log::error!(
//             "Weak PIN rejected for phone number: {} (no sequential or repeated digits)",
//             phone_number
//         );
//         return;
//     }

//     if ctx
//         .db
//         .user_pin()
//         .phone_number()
//         .find(phone_number.clone())
//         .is_some()
//     {
//         log::warn!("PIN already exists for phone number: {}", phone_number);
//         return;
//     }

//     let salt = generate_salt();
//     let pin_hash = hash_pin(&pin, &salt);

//     ctx.db.user_pin().insert(UserPIN {
//         phone_number: phone_number.clone(),
//         pin_hash,
//         salt,
//         attempts: 0,
//         locked: false,
//         last_attempt_time: None,
//         lockout_until: None,
//         created_at: ctx.timestamp,
//         updated_at: ctx.timestamp,
//     });

//     log::info!(
//         "PIN created successfully for phone number: {}",
//         phone_number
//     );
// }

// #[reducer]
// pub fn validate_pin(ctx: &ReducerContext, session_id: String, phone_number: String, pin: String) {
//     if !validate_pin_format(&pin) {
//         log::error!("Invalid PIN format for phone number: {}", phone_number);
//         return;
//     }

//     let user_pin = match ctx.db.user_pin().phone_number().find(phone_number.clone()) {
//         Some(p) => p,
//         None => {
//             log::warn!("User not found for phone number: {}", phone_number);
//             return;
//         }
//     };

//     if user_pin.locked {
//         log::warn!("Account locked for phone number: {}", phone_number);
//         return;
//     }

//     if is_rate_limited(&user_pin, ctx.timestamp) {
//         log::warn!(
//             "Rate limit exceeded for phone: {}. Account is locked for {} minutes",
//             phone_number,
//             LOCKOUT_DURATION_MINUTES
//         );
//         return;
//     }

//     let computed_hash = hash_pin(&pin, &user_pin.salt);

//     if constant_time_compare(&computed_hash, &user_pin.pin_hash) {
//         ctx.db.user_pin().phone_number().update(UserPIN {
//             attempts: 0,
//             locked: false,
//             last_attempt_time: Some(ctx.timestamp),
//             lockout_until: None,
//             updated_at: ctx.timestamp,
//             ..user_pin
//         });

//         if let Some(session) = ctx.db.ussd_session().session_id().find(session_id.clone()) {
//             ctx.db.ussd_session().session_id().update(USSDSession {
//                 authenticated: true,
//                 last_interaction_time: ctx.timestamp,
//                 ..session
//             });
//             log::info!(
//                 "PIN validation successful for session: {} phone: {}. Session authenticated",
//                 session_id,
//                 phone_number
//             );
//         } else {
//             log::info!(
//                 "PIN validation successful for session: {} phone: {}",
//                 session_id,
//                 phone_number
//             );
//         }
//     } else {
//         let new_attempts = user_pin.attempts + 1;
//         let locked = new_attempts >= MAX_PIN_ATTEMPTS;
//         let lockout_until = if locked {
//             Some(calculate_lockout_time(ctx.timestamp))
//         } else {
//             None
//         };

//         ctx.db.user_pin().phone_number().update(UserPIN {
//             attempts: new_attempts,
//             locked,
//             last_attempt_time: Some(ctx.timestamp),
//             lockout_until,
//             updated_at: ctx.timestamp,
//             ..user_pin
//         });

//         if locked {
//             log::warn!(
//                 "Account locked after {} failed attempts for phone: {}. Locked for {} minutes",
//                 MAX_PIN_ATTEMPTS,
//                 phone_number,
//                 LOCKOUT_DURATION_MINUTES
//             );
//         } else {
//             log::warn!(
//                 "Invalid PIN attempt {} of {} for phone: {}",
//                 new_attempts,
//                 MAX_PIN_ATTEMPTS,
//                 phone_number
//             );
//         }
//     }
// }

// #[reducer]
// pub fn reset_pin_attempts(ctx: &ReducerContext, phone_number: String) {
//     let user_pin = match ctx.db.user_pin().phone_number().find(phone_number.clone()) {
//         Some(p) => p,
//         None => {
//             log::warn!("User not found for phone number: {}", phone_number);
//             return;
//         }
//     };

//     ctx.db.user_pin().phone_number().update(UserPIN {
//         attempts: 0,
//         locked: false,
//         last_attempt_time: None,
//         lockout_until: None,
//         updated_at: ctx.timestamp,
//         ..user_pin
//     });

//     log::info!(
//         "PIN attempts reset successfully for phone number: {}",
//         phone_number
//     );
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pin_format_validation() {
        assert!(validate_pin_format("1234", false).is_ok());
        assert!(validate_pin_format("123456", false).is_ok());
        assert!(validate_pin_format("0000", false).is_ok());

        assert!(validate_pin_format("123", false).is_err());
        assert!(validate_pin_format("1234567", false).is_err());
        assert!(validate_pin_format("12a4", false).is_err());
        assert!(validate_pin_format("abcd", false).is_err());
        assert!(validate_pin_format("", false).is_err());
    }

    // #[test]
    // fn test_hash_pin_deterministic() {
    //     let pin = "1234";
    //     let salt = "test_salt";

    //     let hash1 = hash_pin(pin, salt);
    //     let hash2 = hash_pin(pin, salt);

    //     assert_eq!(hash1, hash2);
    // }

    // #[test]
    // fn test_hash_pin_different_for_different_pins() {
    //     let salt = "test_salt";

    //     let hash1 = hash_pin("1234", salt);
    //     let hash2 = hash_pin("5678", salt);

    //     assert_ne!(hash1, hash2);
    // }

    // #[test]
    // fn test_hash_pin_different_for_different_salts() {
    //     let pin = "1234";

    //     let hash1 = hash_pin(pin, "salt1");
    //     let hash2 = hash_pin(pin, "salt2");

    //     assert_ne!(hash1, hash2);
    // }

    // #[test]
    // fn test_generate_salt_unique() {
    //     let salt1 = generate_salt();
    //     std::thread::sleep(std::time::Duration::from_nanos(1));
    //     let salt2 = generate_salt();

    //     assert_ne!(salt1, salt2);
    // }

    #[test]
    fn test_hash_pin_produces_hex_string() {
        let hash = hash_pin("1234", "+254700000000", "salt");
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn test_constant_time_compare_equal() {
        let a = "abc123def456";
        let b = "abc123def456";
        assert!(constant_time_compare(a, b));
    }

    #[test]
    fn test_constant_time_compare_not_equal() {
        let a = "abc123def456";
        let b = "abc123def457";
        assert!(!constant_time_compare(a, b));
    }

    #[test]
    fn test_constant_time_compare_different_lengths() {
        let a = "abc123";
        let b = "abc123def";
        assert!(!constant_time_compare(a, b));
    }

    #[test]
    fn test_weak_pin_repeated_digits() {
        assert!(is_weak_pin("1111"));
        assert!(is_weak_pin("0000"));
        assert!(is_weak_pin("9999"));
        assert!(is_weak_pin("555555"));
    }

    #[test]
    fn test_weak_pin_sequential_ascending() {
        assert!(is_weak_pin("1234"));
        assert!(is_weak_pin("0123"));
        assert!(is_weak_pin("56789"));
        assert!(is_weak_pin("123456"));
    }

    #[test]
    fn test_weak_pin_sequential_descending() {
        assert!(is_weak_pin("4321"));
        assert!(is_weak_pin("9876"));
        assert!(is_weak_pin("654321"));
    }

    #[test]
    fn test_strong_pin_accepted() {
        assert!(!is_weak_pin("1357"));
        assert!(!is_weak_pin("2468"));
        assert!(!is_weak_pin("1593"));
        assert!(!is_weak_pin("9182"));
        assert!(!is_weak_pin("4826"));
    }

    #[test]
    fn test_calculate_lockout_time_adds_duration() {
        use std::time::SystemTime;

        let now = Timestamp::from(SystemTime::now());
        let lockout_time = calculate_lockout_time(now);

        assert!(lockout_time > now, "Lockout time should be in the future");
    }

    #[test]
    fn test_is_rate_limited_returns_false_when_no_lockout() {
        use std::time::SystemTime;

        let user_pin = UserPIN {
            phone_number: "+254712345678".to_string(),
            pin_hash: "hash".to_string(),
            salt: "salt".to_string(),
            attempts: 0,
            locked: false,
            last_attempt_time: None,
            lockout_until: None,
            created_at: Timestamp::from(SystemTime::now()),
            updated_at: Timestamp::from(SystemTime::now()),
        };

        let current_time = Timestamp::from(SystemTime::now());
        assert!(!is_rate_limited(&user_pin, current_time));
    }

    #[test]
    fn test_is_rate_limited_returns_true_during_active_lockout() {
        use std::time::SystemTime;

        let current_time = Timestamp::from(SystemTime::now());
        let future_lockout = calculate_lockout_time(current_time);

        let user_pin = UserPIN {
            phone_number: "+254712345678".to_string(),
            pin_hash: "hash".to_string(),
            salt: "salt".to_string(),
            attempts: 3,
            locked: true,
            last_attempt_time: Some(current_time),
            lockout_until: Some(future_lockout),
            created_at: current_time,
            updated_at: current_time,
        };

        assert!(
            is_rate_limited(&user_pin, current_time),
            "Should be rate limited when lockout is in the future"
        );
    }

    #[test]
    fn test_is_rate_limited_returns_false_after_lockout_expires() {
        use std::time::SystemTime;

        let past_time = Timestamp::from(SystemTime::UNIX_EPOCH);
        let past_lockout = calculate_lockout_time(past_time);
        let current_time = Timestamp::from(SystemTime::now());

        let user_pin = UserPIN {
            phone_number: "+254712345678".to_string(),
            pin_hash: "hash".to_string(),
            salt: "salt".to_string(),
            attempts: 3,
            locked: true,
            last_attempt_time: Some(past_time),
            lockout_until: Some(past_lockout),
            created_at: past_time,
            updated_at: past_time,
        };

        assert!(
            !is_rate_limited(&user_pin, current_time),
            "Should not be rate limited when lockout has expired"
        );
    }

    #[test]
    fn test_lockout_duration_is_15_minutes() {
        use std::time::SystemTime;

        let start = Timestamp::from(SystemTime::now());
        let lockout = calculate_lockout_time(start);

        let expected_duration = Duration::from_secs(15 * 60);
        let actual = lockout;
        let expected = start + expected_duration;

        assert_eq!(
            actual, expected,
            "Lockout duration should be exactly 15 minutes"
        );
    }
}
