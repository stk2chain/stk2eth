// File: tests/integration/send_eth_test.rs

use spacetimedb::{ReducerContext, Identity, Timestamp};
use spacetimedb_testing::{TestDb, TestReducerContext};
use ussdgeth::{send_eth, USSDSession, Swap};
use std::str::FromStr;

#[cfg(test)]
mod send_eth_tests {
    use super::*;

    fn create_test_context() -> TestReducerContext {
        let mut test_db = TestDb::new();
        TestReducerContext::new(test_db, Identity::from_str("test_user").unwrap())
    }

    fn create_test_session(ctx: &ReducerContext) -> USSDSession {
        let session = USSDSession {
            session_id: "test_session_001".to_string(),
            phone_number: "+1234567890".to_string(),
            network_code: "TEST_NET".to_string(),
            service_code: "*ETH#".to_string(),
            data: "".to_string(),
            current_screen: "send_eth_confirm".to_string(),
            visited_screens: vec!["main_menu".to_string(), "send_amount".to_string()],
            last_interaction_time: ctx.timestamp,
            end_session: false,
            sender: ctx.sender,
            online: true,
        };
        
        ctx.db.ussd_session().insert(session.clone());
        session
    }

    #[test]
    fn test_send_eth_reducer_fails_initially() {
        // Arrange
        let ctx = create_test_context();
        let session = create_test_session(&ctx);
        
        let from_address = "0x742d35Cc6634C0532925a3b8D0A9E9B5F8C8C4C1".to_string();
        let to_address = "0x8ba1f109551bD432803012645Hac136c6b3d283c".to_string();
        let amount_eth = "1.5".to_string(); // 1.5 ETH
        
        // Act & Assert - This should fail because reducer is not implemented yet
        let result = std::panic::catch_unwind(|| {
            send_eth(&ctx, session.session_id, from_address, to_address, amount_eth)
        });
        
        assert!(result.is_err(), "send_eth reducer should fail - not implemented yet");
    }

    #[test] 
    fn test_send_eth_creates_swap_transaction() {
        // This test will pass once we implement the reducer
        let ctx = create_test_context();
        let session = create_test_session(&ctx);
        
        let from_address = "0x742d35Cc6634C0532925a3b8D0A9E9B5F8C8C4C1".to_string();
        let to_address = "0x8ba1f109551bD432803012645Hac136c6b3d283c".to_string();
        let amount_eth = "1.5".to_string();
        
        // Act
        send_eth(&ctx, session.session_id.clone(), from_address.clone(), to_address.clone(), amount_eth.clone());
        
        // Assert - Check that a Swap was created
        let swaps: Vec<Swap> = ctx.db.swap().iter().collect();
        assert_eq!(swaps.len(), 1, "Should create exactly one swap");
        
        let swap = &swaps[0];
        assert_eq!(swap.session_id, session.session_id);
        assert_eq!(swap.from_address, from_address);
        assert_eq!(swap.to_address, to_address);
        assert_eq!(swap.amount, amount_eth);
        assert_eq!(swap.token_in, "ETH");
        assert_eq!(swap.token_out, "ETH"); // Send ETH is ETH->ETH swap with different recipient
        assert_eq!(swap.status, "pending");
    }

    #[test]
    fn test_send_eth_validates_addresses() {
        let ctx = create_test_context();
        let session = create_test_session(&ctx);
        
        // Test invalid from address
        let result = std::panic::catch_unwind(|| {
            send_eth(&ctx, session.session_id.clone(), "invalid_address".to_string(), 
                    "0x8ba1f109551bD432803012645Hac136c6b3d283c".to_string(), "1.0".to_string())
        });
        assert!(result.is_err(), "Should fail with invalid from address");
        
        // Test invalid to address  
        let result = std::panic::catch_unwind(|| {
            send_eth(&ctx, session.session_id.clone(), "0x742d35Cc6634C0532925a3b8D0A9E9B5F8C8C4C1".to_string(),
                    "invalid_address".to_string(), "1.0".to_string())
        });
        assert!(result.is_err(), "Should fail with invalid to address");
    }

    #[test]
    fn test_send_eth_validates_amount() {
        let ctx = create_test_context();
        let session = create_test_session(&ctx);
        
        let from_address = "0x742d35Cc6634C0532925a3b8D0A9E9B5F8C8C4C1".to_string();
        let to_address = "0x8ba1f109551bD432803012645Hac136c6b3d283c".to_string();
        
        // Test zero amount
        let result = std::panic::catch_unwind(|| {
            send_eth(&ctx, session.session_id.clone(), from_address.clone(), to_address.clone(), "0".to_string())
        });
        assert!(result.is_err(), "Should fail with zero amount");
        
        // Test negative amount
        let result = std::panic::catch_unwind(|| {
            send_eth(&ctx, session.session_id.clone(), from_address.clone(), to_address.clone(), "-1.5".to_string())
        });
        assert!(result.is_err(), "Should fail with negative amount");
        
        // Test invalid format
        let result = std::panic::catch_unwind(|| {
            send_eth(&ctx, session.session_id.clone(), from_address.clone(), to_address.clone(), "not_a_number".to_string())
        });
        assert!(result.is_err(), "Should fail with invalid amount format");
    }

    #[test]
    fn test_send_eth_updates_session_state() {
        let ctx = create_test_context();
        let session = create_test_session(&ctx);
        
        let from_address = "0x742d35Cc6634C0532925a3b8D0A9E9B5F8C8C4C1".to_string();
        let to_address = "0x8ba1f109551bD432803012645Hac136c6b3d283c".to_string();
        let amount_eth = "1.5".to_string();
        
        // Act
        send_eth(&ctx, session.session_id.clone(), from_address, to_address, amount_eth);
        
        // Assert - Session should be updated to show transaction is processing
        let updated_session = ctx.db.ussd_session().session_id().find(session.session_id.clone()).unwrap();
        assert_eq!(updated_session.current_screen, "transaction_processing");
        assert!(updated_session.visited_screens.contains(&"send_eth_confirm".to_string()));
    }
}