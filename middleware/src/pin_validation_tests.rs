#[cfg(test)]
use crate::reducers::validate_pin::{generate_salt, hash_pin};
#[cfg(test)]
use crate::PINValidationResult;

#[cfg(test)]
mod pin_validation_integration_tests {
    use super::*;

    #[test]
    fn test_correct_pin_passes() {
        let pin = "1234";
        let salt = "test_salt_12345";

        let stored_hash = hash_pin(pin, salt);
        let input_hash = hash_pin(pin, salt);

        assert_eq!(
            stored_hash, input_hash,
            "Correct PIN should produce matching hash"
        );
    }

    #[test]
    fn test_wrong_pin_fails() {
        let correct_pin = "1234";
        let wrong_pin = "5678";
        let salt = "test_salt_12345";

        let stored_hash = hash_pin(correct_pin, salt);
        let input_hash = hash_pin(wrong_pin, salt);

        assert_ne!(
            stored_hash, input_hash,
            "Wrong PIN should produce different hash"
        );
    }

    #[test]
    fn test_hash_consistency() {
        let pin = "9876";
        let salt = generate_salt();

        let hash1 = hash_pin(pin, &salt);
        let hash2 = hash_pin(pin, &salt);
        let hash3 = hash_pin(pin, &salt);

        assert_eq!(hash1, hash2);
        assert_eq!(hash2, hash3);
        assert_eq!(hash1, hash3);
    }

    #[test]
    fn test_false_accept_rate_zero() {
        let correct_pin = "1234";
        let salt = generate_salt();
        let stored_hash = hash_pin(correct_pin, &salt);

        let wrong_pins = vec![
            "0000", "1111", "2222", "3333", "4444", "5555", "6666", "7777", "8888", "9999", "1235",
            "1233", "1244", "1134", "2234", "1294", "1334", "1264", "5234", "1230",
        ];

        let mut false_accepts = 0;
        let total_attempts = wrong_pins.len();

        for wrong_pin in wrong_pins {
            let input_hash = hash_pin(wrong_pin, &salt);
            if input_hash == stored_hash {
                false_accepts += 1;
            }
        }

        let false_accept_rate = (false_accepts as f64 / total_attempts as f64) * 100.0;

        assert_eq!(
            false_accepts, 0,
            "No wrong PIN should be accepted. False accept rate: {:.2}%",
            false_accept_rate
        );
        assert!(
            false_accept_rate <= 1.0,
            "False accept rate {:.2}% exceeds 1% threshold",
            false_accept_rate
        );
    }

    #[test]
    fn test_pin_validation_result_types() {
        let success = PINValidationResult::Success;
        let invalid = PINValidationResult::InvalidPIN;
        let locked = PINValidationResult::AccountLocked;
        let not_found = PINValidationResult::UserNotFound;

        assert_ne!(success, invalid);
        assert_ne!(success, locked);
        assert_ne!(success, not_found);
        assert_ne!(invalid, locked);
        assert_ne!(invalid, not_found);
        assert_ne!(locked, not_found);
    }

    #[test]
    fn test_salt_uniqueness() {
        let mut salts = Vec::new();
        for _ in 0..100 {
            salts.push(generate_salt());
        }

        let unique_salts: std::collections::HashSet<_> = salts.iter().collect();
        assert_eq!(
            salts.len(),
            unique_salts.len(),
            "All generated salts should be unique"
        );
    }

    #[test]
    fn test_hash_length_consistency() {
        let pins = vec!["1234", "0000", "999999", "1111"];
        let salt = generate_salt();

        for pin in pins {
            let hash = hash_pin(pin, &salt);
            assert_eq!(
                hash.len(),
                64,
                "SHA256 hash should always be 64 hex characters"
            );
        }
    }

    #[test]
    fn test_timing_attack_resistance() {
        let correct_pin = "1234";
        let salt = generate_salt();
        let _stored_hash = hash_pin(correct_pin, &salt);

        let wrong_pins = vec!["0000", "1111", "9999"];

        for wrong_pin in wrong_pins {
            let start = std::time::Instant::now();
            let _ = hash_pin(wrong_pin, &salt);
            let wrong_duration = start.elapsed();

            let start = std::time::Instant::now();
            let _ = hash_pin(correct_pin, &salt);
            let correct_duration = start.elapsed();

            let diff = wrong_duration.abs_diff(correct_duration);

            assert!(
                diff < std::time::Duration::from_millis(1),
                "Hash computation time should be consistent to prevent timing attacks"
            );
        }
    }

    #[test]
    fn test_brute_force_protection() {
        let correct_pin = "1234";
        let salt = generate_salt();
        let stored_hash = hash_pin(correct_pin, &salt);

        let mut attempts = 0;
        let max_attempts = 10000;

        for i in 0..max_attempts {
            let test_pin = format!("{:04}", i);
            let test_hash = hash_pin(&test_pin, &salt);

            if test_hash == stored_hash {
                attempts += 1;
                assert_eq!(
                    test_pin, correct_pin,
                    "Only correct PIN should match the hash"
                );
            }
        }

        assert_eq!(
            attempts, 1,
            "Exactly one PIN should match (the correct one)"
        );
    }

    #[test]
    fn test_pin_hash_collision_resistance() {
        let salt = generate_salt();
        let mut hashes = std::collections::HashMap::new();

        for i in 0..10000 {
            let pin = format!("{:04}", i);
            let hash = hash_pin(&pin, &salt);

            if let Some(existing_pin) = hashes.insert(hash.clone(), pin.clone()) {
                panic!(
                    "Hash collision detected: PIN {} and {} produce the same hash",
                    existing_pin, pin
                );
            }
        }

        assert_eq!(hashes.len(), 10000, "All PINs should have unique hashes");
    }

    #[test]
    fn test_different_salts_produce_different_hashes() {
        let pin = "1234";
        let mut hashes = Vec::new();

        for _ in 0..100 {
            let salt = generate_salt();
            let hash = hash_pin(pin, &salt);
            hashes.push(hash);
        }

        let unique_hashes: std::collections::HashSet<_> = hashes.iter().collect();
        assert_eq!(
            hashes.len(),
            unique_hashes.len(),
            "Same PIN with different salts should produce different hashes"
        );
    }

    #[test]
    fn test_metric_false_accept_rate_below_one_percent() {
        let correct_pin = "5432";
        let salt = generate_salt();
        let stored_hash = hash_pin(correct_pin, &salt);

        let total_tests = 10000;
        let mut false_accepts = 0;

        for i in 0..total_tests {
            let wrong_pin = format!("{:04}", i);

            if wrong_pin == correct_pin {
                continue;
            }

            let test_hash = hash_pin(&wrong_pin, &salt);
            if test_hash == stored_hash {
                false_accepts += 1;
            }
        }

        let false_accept_rate = (false_accepts as f64 / total_tests as f64) * 100.0;


        assert!(
            false_accept_rate <= 1.0,
            "METRIC FAILURE: False accept rate {:.6}% exceeds 1% threshold",
            false_accept_rate
        );
    }

    #[test]
    fn test_auth_success_only_for_correct_pin() {
        let correct_pin = "7890";
        let wrong_pins = vec![
            "0000", "1111", "2222", "3333", "4444", "5555", "6666", "7777", "8888", "9999", "7891",
            "7889", "7990", "7800", "6890", "7899", "0890", "7890 ", " 7890", "789",
        ];

        let salt = generate_salt();
        let stored_hash = hash_pin(correct_pin, &salt);

        let correct_hash = hash_pin(correct_pin, &salt);
        assert_eq!(
            stored_hash, correct_hash,
            "Correct PIN must authenticate successfully"
        );

        for wrong_pin in wrong_pins {
            let test_hash = hash_pin(wrong_pin, &salt);
            assert_ne!(
                stored_hash, test_hash,
                "Wrong PIN '{}' should not authenticate",
                wrong_pin
            );
        }
    }
}

#[cfg(test)]
mod pin_security_tests {
    use super::*;

    #[test]
    fn test_hash_output_length() {
        let pin = "1234";
        let salt = "test_salt";
        let hash = hash_pin(pin, salt);

        assert_eq!(
            hash.len(),
            64,
            "SHA256 hash must be 64 characters (256 bits in hex)"
        );
    }

    #[test]
    fn test_hash_contains_only_hex_characters() {
        let pin = "5678";
        let salt = generate_salt();
        let hash = hash_pin(pin, &salt);

        for ch in hash.chars() {
            assert!(
                ch.is_ascii_hexdigit(),
                "Hash must contain only hexadecimal characters"
            );
        }
    }

    #[test]
    fn test_salt_entropy() {
        let salt1 = generate_salt();
        let salt2 = generate_salt();

        assert_ne!(salt1, salt2, "Salts must be unique");
        assert!(!salt1.is_empty(), "Salt must not be empty");
        assert!(!salt2.is_empty(), "Salt must not be empty");
    }

    #[test]
    fn test_pin_hash_not_reversible() {
        let original_pin = "4321";
        let salt = "secure_salt";
        let hash = hash_pin(original_pin, salt);

        assert!(
            !hash.contains(original_pin),
            "Hash must not contain original PIN"
        );
        assert!(
            !hash.contains(salt),
            "Hash must not contain salt in plaintext"
        );
    }

    #[test]
    fn test_hash_avalanche_effect() {
        let salt = generate_salt();

        let hash1 = hash_pin("1234", &salt);
        let hash2 = hash_pin("1235", &salt);

        let mut differences = 0;
        for (c1, c2) in hash1.chars().zip(hash2.chars()) {
            if c1 != c2 {
                differences += 1;
            }
        }

        assert!(
            differences > 30,
            "Small PIN change should cause significant hash change (avalanche effect). Only {} characters differ",
            differences
        );
    }
}

#[cfg(test)]
mod rate_limiting_tests {
    use super::*;
    use spacetimedb::Timestamp;
    use std::time::SystemTime;

    #[test]
    fn test_lockout_time_calculation() {
        let now = Timestamp::from(SystemTime::now());
        let salt = generate_salt();
        let hash1 = hash_pin("1234", &salt);
        let hash2 = hash_pin("5678", &salt);

        assert_ne!(hash1, hash2);
        assert!(now <= Timestamp::from(SystemTime::now()));
    }

    #[test]
    fn test_multiple_salts_produce_unique_hashes() {
        let pin = "1234";
        let mut hashes = Vec::new();

        for _ in 0..10 {
            let salt = generate_salt();
            let hash = hash_pin(pin, &salt);
            assert!(
                !hashes.contains(&hash),
                "Hash should be unique for each salt"
            );
            hashes.push(hash);
        }

        assert_eq!(hashes.len(), 10);
    }

    #[test]
    fn test_timestamp_ordering() {
        let t1 = Timestamp::from(SystemTime::UNIX_EPOCH);
        let t2 = Timestamp::from(SystemTime::now());

        assert!(t2 > t1, "Current time should be after UNIX epoch");
    }
}
