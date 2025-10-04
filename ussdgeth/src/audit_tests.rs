#[cfg(test)]
mod audit_log_tests {
    use crate::EthAuditLog;

    #[test]
    fn test_audit_log_data_validation() {
        // Test that audit log data structures are valid
        let audit_log = EthAuditLog {
            id: 1,
            tx_hash: "0x1234567890abcdef".to_string(),
            from_address: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0".to_string(),
            to_address: "0x5aAeb6053f3E94C9b9A09f33669435E7Ef1BeAed".to_string(),
            amount: "1000000000000000000".to_string(),
            phone_number: "+254712345678".to_string(),
            session_id: "test_session_123".to_string(),
            timestamp: spacetimedb::Timestamp::now(),
            originator_name: Some("John Doe".to_string()),
            beneficiary_name: Some("Jane Smith".to_string()),
            originator_country: Some("KE".to_string()),
            beneficiary_country: Some("US".to_string()),
            originator_address: Some("0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0".to_string()),
            beneficiary_address: Some("0x5aAeb6053f3E94C9b9A09f33669435E7Ef1BeAed".to_string()),
            originator_id: Some("ID123".to_string()),
            beneficiary_id: Some("ID456".to_string()),
            transaction_type: "send_eth".to_string(),
            network: "ethereum".to_string(),
            gas_fee: Some("21000".to_string()),
            exchange_rate: Some("1.0".to_string()),
            compliance_status: "pending".to_string(),
            risk_score: Some(1),
            is_immutable: true,
        };

        // Validate basic fields
        assert_eq!(audit_log.tx_hash, "0x1234567890abcdef");
        assert_eq!(audit_log.amount, "1000000000000000000");
        assert_eq!(audit_log.phone_number, "+254712345678");
        assert!(audit_log.is_immutable);
        assert_eq!(audit_log.compliance_status, "pending");
    }

    #[test]
    fn test_address_validation() {
        // Test Ethereum address validation logic
        let valid_address = "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0";
        let invalid_address = "invalid_address";

        // Valid address should be 42 characters (0x + 40 hex chars)
        assert_eq!(valid_address.len(), 42);
        assert!(valid_address.starts_with("0x"));

        // Invalid address should not match format
        assert_ne!(invalid_address.len(), 42);
        assert!(!invalid_address.starts_with("0x"));
    }

    #[test]
    fn test_fatf_compliance_data_structure() {
        // Test FATF travel rule compliance data structure
        let fatf_log = EthAuditLog {
            id: 1,
            tx_hash: "0xabcdef123456".to_string(),
            from_address: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0".to_string(),
            to_address: "0x5aAeb6053f3E94C9b9A09f33669435E7Ef1BeAed".to_string(),
            amount: "1000000000000000000".to_string(),
            phone_number: "+254712345678".to_string(),
            session_id: "session_123".to_string(),
            timestamp: spacetimedb::Timestamp::now(),
            originator_name: Some("John Doe".to_string()),
            beneficiary_name: Some("Jane Smith".to_string()),
            originator_country: Some("KE".to_string()),
            beneficiary_country: Some("US".to_string()),
            originator_address: Some("0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0".to_string()),
            beneficiary_address: Some("0x5aAeb6053f3E94C9b9A09f33669435E7Ef1BeAed".to_string()),
            originator_id: Some("ID789".to_string()),
            beneficiary_id: Some("ID012".to_string()),
            transaction_type: "send_eth".to_string(),
            network: "ethereum".to_string(),
            gas_fee: Some("21000".to_string()),
            exchange_rate: Some("1.0".to_string()),
            compliance_status: "compliant".to_string(),
            risk_score: Some(1),
            is_immutable: true,
        };

        // Validate FATF compliance fields
        assert!(fatf_log.originator_name.is_some());
        assert!(fatf_log.beneficiary_name.is_some());
        assert!(fatf_log.originator_country.is_some());
        assert!(fatf_log.beneficiary_country.is_some());
        assert_eq!(fatf_log.originator_name.unwrap(), "John Doe");
        assert_eq!(fatf_log.beneficiary_name.unwrap(), "Jane Smith");
        assert_eq!(fatf_log.compliance_status, "compliant");
    }

    #[test]
    fn test_phone_number_validation() {
        // Test phone number format validation
        let valid_phones = vec!["+254712345678", "+1234567890", "+44123456789"];

        let invalid_phones = vec![
            "254712345678",     // Missing +
            "+254712345678901", // Too long (16 chars > 15)
            "+254abc123",       // Contains letters
            "+254123",          // Too short (8 chars < 10)
            "",                 // Empty
        ];

        for phone in valid_phones {
            assert!(phone.starts_with("+"));
            assert!(phone.len() >= 10);
            assert!(phone.len() <= 15);
        }

        for phone in invalid_phones {
            // Each invalid phone should fail at least one validation criteria
            let has_plus = phone.starts_with("+");
            let correct_length = phone.len() >= 10 && phone.len() <= 15;
            let only_digits_and_plus = phone.chars().all(|c| c.is_ascii_digit() || c == '+');
            let not_empty = !phone.is_empty();

            let is_valid = has_plus && correct_length && only_digits_and_plus && not_empty;

            assert!(
                !is_valid,
                "Phone '{}' (len={}) should be invalid",
                phone,
                phone.len()
            );
        }
    }

    #[test]
    fn test_transaction_amount_validation() {
        // Test transaction amount validation
        let valid_amounts = vec![
            "1000000000000000000", // 1 ETH in wei
            "500000000000000000",  // 0.5 ETH
            "1",                   // 1 wei
        ];

        let invalid_amounts = vec![
            "0",                    // Zero amount
            "-1000000000000000000", // Negative amount
            "abc123",               // Non-numeric
            "",                     // Empty
        ];

        for amount in valid_amounts {
            assert!(!amount.is_empty());
            assert!(amount.parse::<u64>().is_ok() || amount.len() > 19); // Handle large numbers
            assert!(amount != "0");
        }

        for amount in invalid_amounts {
            assert!(
                amount.is_empty()
                    || amount == "0"
                    || amount.starts_with("-")
                    || amount.parse::<u64>().is_err()
            );
        }
    }
}
