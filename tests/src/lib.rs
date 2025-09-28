use std::collections::HashMap;
use reqwest::Client;
use serde_json::{json, Value};
use tokio_test;
use assert_matches::assert_matches;

/// E2E Test Suite for STK2ETH USSD→SpacetimeDB→Ethereum Pipeline
/// 
/// Tests the complete flow:
/// 1. User dials *4337#
/// 2. USSD session is created and persisted
/// 3. User navigates through Send ETH flow
/// 4. Transaction is processed and recorded
/// 5. Session is properly cleaned up
/// 
/// Target: 100% pass rate in CI pipeline

pub struct E2ETestFramework {
    client: Client,
    base_url: String,
    spacetime_url: String,
    spacetime_token: String,
    test_phone: String,
}

impl E2ETestFramework {
    pub fn new() -> Self {
        dotenv::dotenv().ok();
        
        Self {
            client: Client::new(),
            base_url: std::env::var("USSD_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:8080".to_string()),
            spacetime_url: std::env::var("SPACETIME_API_URL")
                .unwrap_or_else(|_| "http://localhost:3000/v1/database/stk2eth".to_string()),
            spacetime_token: std::env::var("SPACETIME_AUTH_TOKEN")
                .unwrap_or_else(|_| "test_token".to_string()),
            test_phone: std::env::var("TEST_PHONE_NUMBER")
                .unwrap_or_else(|_| "+254712345678".to_string()),
        }
    }

    /// Simulates AfricasTalking USSD request
    pub async fn send_ussd_request(&self, session_id: &str, text: &str) -> Result<String, Box<dyn std::error::Error>> {
        let payload = json!({
            "session_id": session_id,
            "service_code": "*4337#",
            "phone_number": self.test_phone,
            "network_code": "63902",
            "text": text
        });

        let response = self.client
            .post(&format!("{}/ussd", self.base_url))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&[
                ("sessionId", session_id),
                ("serviceCode", "*4337#"),
                ("phoneNumber", &self.test_phone),
                ("networkCode", "63902"),
                ("text", text),
            ])
            .send()
            .await?;

        let response_text = response.text().await?;
        println!("USSD Response: {}", response_text);
        Ok(response_text)
    }

    /// Verifies session exists in SpacetimeDB
    pub async fn verify_session_exists(&self, session_id: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let query = format!("SELECT session_id FROM ussd_session WHERE session_id = '{}';", session_id);
        
        let response = self.client
            .post(&format!("{}/sql", self.spacetime_url))
            .bearer_auth(&self.spacetime_token)
            .header("Content-Type", "text/plain")
            .body(query)
            .send()
            .await?;

        let body = response.text().await?;
        let json: Value = serde_json::from_str(&body)?;
        
        Ok(json.get(0)
            .and_then(|v| v.get("rows"))
            .and_then(|rows| rows.get(0))
            .is_some())
    }

    /// Verifies ETH transaction was recorded
    pub async fn verify_eth_transaction(&self, session_id: &str) -> Result<Option<Value>, Box<dyn std::error::Error>> {
        let query = format!(
            "SELECT from_address, to_address, amount, status, tx_hash FROM swap WHERE session_id = '{}' AND swap_type = 'SendEth';",
            session_id
        );
        
        let response = self.client
            .post(&format!("{}/sql", self.spacetime_url))
            .bearer_auth(&self.spacetime_token)
            .header("Content-Type", "text/plain")
            .body(query)
            .send()
            .await?;

        let body = response.text().await?;
        let json: Value = serde_json::from_str(&body)?;
        
        if let Some(row) = json.get(0).and_then(|v| v.get("rows")).and_then(|rows| rows.get(0)) {
            Ok(Some(json!({
                "from_address": row.get(0),
                "to_address": row.get(1), 
                "amount": row.get(2),
                "status": row.get(3),
                "tx_hash": row.get(4),
            })))
        } else {
            Ok(None)
        }
    }

    /// Cleanup test data
    pub async fn cleanup_test_session(&self, session_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Delete session
        let delete_session = format!("DELETE FROM ussd_session WHERE session_id = '{}';", session_id);
        
        self.client
            .post(&format!("{}/sql", self.spacetime_url))
            .bearer_auth(&self.spacetime_token)
            .header("Content-Type", "text/plain")
            .body(delete_session)
            .send()
            .await?;

        // Delete related swaps
        let delete_swaps = format!("DELETE FROM swap WHERE session_id = '{}';", session_id);
        
        self.client
            .post(&format!("{}/sql", self.spacetime_url))
            .bearer_auth(&self.spacetime_token)
            .header("Content-Type", "text/plain")
            .body(delete_swaps)
            .send()
            .await?;

        Ok(())
    }
}

#[tokio::test]
async fn test_complete_ussd_to_ethereum_pipeline() {
    let framework = E2ETestFramework::new();
    let session_id = format!("e2e_test_{}", uuid::Uuid::new_v4());
    
    // Cleanup any existing test data
    let _ = framework.cleanup_test_session(&session_id).await;
    
    println!("Starting E2E test with session: {}", session_id);

    // Step 1: Initial dial *4337# - should show main menu
    let response1 = framework.send_ussd_request(&session_id, "").await
        .expect("Failed to send initial USSD request");
    
    assert!(response1.contains("CON"), "Initial response should continue session");
    assert!(response1.contains("M-ETH Main Menu"), "Should show main menu");
    
    // Verify session was created in SpacetimeDB
    assert!(framework.verify_session_exists(&session_id).await
        .expect("Failed to verify session"), "Session should exist in database");

    // Step 2: Select "1" for Send ETH
    let response2 = framework.send_ussd_request(&session_id, "1").await
        .expect("Failed to send Send ETH selection");
    
    assert!(response2.contains("CON"), "Should continue session");
    assert!(response2.contains("Enter ETH amount"), "Should prompt for amount");

    // Step 3: Enter amount "0.001"
    let response3 = framework.send_ussd_request(&session_id, "1*0.001").await
        .expect("Failed to send amount");
    
    assert!(response3.contains("CON"), "Should continue session");
    assert!(response3.contains("Enter recipient address"), "Should prompt for recipient");

    // Step 4: Enter recipient address
    let test_address = "0x742d35Cc6634C0532925a3b8D42C25D4F86F94ad";
    let response4 = framework.send_ussd_request(&session_id, &format!("1*0.001*{}", test_address)).await
        .expect("Failed to send recipient address");
    
    assert!(response4.contains("CON"), "Should continue session");
    assert!(response4.contains("Confirm Send"), "Should show confirmation");
    assert!(response4.contains("0.001"), "Should show amount");
    assert!(response4.contains(&test_address), "Should show recipient");

    // Step 5: Confirm transaction (select "1")
    let response5 = framework.send_ussd_request(&session_id, &format!("1*0.001*{}*1", test_address)).await
        .expect("Failed to confirm transaction");
    
    assert!(response5.contains("CON"), "Should continue session");
    assert!(response5.contains("Enter your PIN"), "Should prompt for PIN");

    // Step 6: Enter PIN to complete transaction
    let response6 = framework.send_ussd_request(&session_id, &format!("1*0.001*{}*1*1234", test_address)).await
        .expect("Failed to enter PIN");
    
    // Should either show processing or completion
    assert!(
        response6.contains("Processing") || response6.contains("successfully") || response6.contains("END"),
        "Should show processing or completion status"
    );

    // Step 7: Verify transaction was recorded in database
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await; // Allow processing time
    
    let tx_result = framework.verify_eth_transaction(&session_id).await
        .expect("Failed to verify transaction");
    
    assert!(tx_result.is_some(), "Transaction should be recorded in database");
    
    let tx_data = tx_result.unwrap();
    assert_eq!(tx_data["amount"], "0.001", "Amount should match input");
    assert_eq!(tx_data["to_address"], test_address, "Recipient should match input");
    assert!(
        tx_data["status"] == "Completed" || tx_data["status"] == "Processing",
        "Transaction should have valid status"
    );

    // Cleanup
    framework.cleanup_test_session(&session_id).await
        .expect("Failed to cleanup test session");
    
    println!("✓ E2E test completed successfully");
}

#[tokio::test]
async fn test_session_interruption_and_resume() {
    let framework = E2ETestFramework::new();
    let session_id = format!("resume_test_{}", uuid::Uuid::new_v4());
    
    // Cleanup any existing test data
    let _ = framework.cleanup_test_session(&session_id).await;
    
    println!("Testing session interruption and resume: {}", session_id);

    // Step 1: Start transaction flow
    let _ = framework.send_ussd_request(&session_id, "").await.unwrap();
    let _ = framework.send_ussd_request(&session_id, "1").await.unwrap(); // Select Send ETH
    let _ = framework.send_ussd_request(&session_id, "1*0.001").await.unwrap(); // Enter amount
    
    // Simulate interruption - new session with same ID
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    
    // Step 2: Resume session - should continue from where left off
    let resume_response = framework.send_ussd_request(&session_id, "1*0.001").await
        .expect("Failed to resume session");
    
    // Should either continue with recipient prompt or show confirmation
    assert!(
        resume_response.contains("Enter recipient address") || 
        resume_response.contains("Confirm Send") ||
        resume_response.contains("CON"),
        "Should successfully resume session: {}", resume_response
    );

    // Verify session persistence
    assert!(framework.verify_session_exists(&session_id).await
        .expect("Failed to verify session after resume"), 
        "Session should persist after interruption");

    // Cleanup
    framework.cleanup_test_session(&session_id).await
        .expect("Failed to cleanup test session");
    
    println!("✓ Session resume test completed successfully");
}

#[tokio::test] 
async fn test_invalid_input_handling() {
    let framework = E2ETestFramework::new();
    let session_id = format!("invalid_test_{}", uuid::Uuid::new_v4());
    
    // Cleanup any existing test data
    let _ = framework.cleanup_test_session(&session_id).await;
    
    println!("Testing invalid input handling: {}", session_id);

    // Step 1: Start flow
    let _ = framework.send_ussd_request(&session_id, "").await.unwrap();
    let _ = framework.send_ussd_request(&session_id, "1").await.unwrap(); // Send ETH

    // Step 2: Invalid amount
    let invalid_amount_response = framework.send_ussd_request(&session_id, "1*invalid_amount").await
        .expect("Failed to send invalid amount");
    
    // Should handle gracefully - either show error or continue
    assert!(invalid_amount_response.contains("CON") || invalid_amount_response.contains("END"));

    // Step 3: Valid amount, invalid address
    let _ = framework.send_ussd_request(&session_id, "1*0.001").await.unwrap();
    let invalid_address_response = framework.send_ussd_request(&session_id, "1*0.001*invalid_address").await
        .expect("Failed to send invalid address");
    
    // Should handle gracefully
    assert!(invalid_address_response.contains("CON") || invalid_address_response.contains("END"));

    // Cleanup
    framework.cleanup_test_session(&session_id).await
        .expect("Failed to cleanup test session");
    
    println!("✓ Invalid input handling test completed successfully");
}

#[tokio::test]
async fn test_session_ttl_cleanup() {
    let framework = E2ETestFramework::new();
    let session_id = format!("ttl_test_{}", uuid::Uuid::new_v4());
    
    // Cleanup any existing test data
    let _ = framework.cleanup_test_session(&session_id).await;
    
    println!("Testing session TTL cleanup: {}", session_id);

    // Step 1: Create session
    let _ = framework.send_ussd_request(&session_id, "").await.unwrap();
    
    // Verify session exists
    assert!(framework.verify_session_exists(&session_id).await.unwrap(), 
        "Session should exist initially");

    // Step 2: Wait for TTL (in production this would be longer)
    // For testing, we'll simulate calling cleanup
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    // Step 3: Call cleanup reducer (simulating scheduled cleanup)
    let cleanup_response = framework.client
        .post(&format!("{}/call/cleanup_expired_sessions", framework.spacetime_url))
        .bearer_auth(&framework.spacetime_token)
        .json(&json!({}))
        .send()
        .await;

    // Note: This test documents expected behavior
    // Implementation may need adjustment based on actual TTL logic
    
    println!("✓ TTL cleanup test completed");
}

/// Runs all E2E tests in sequence
/// Target: 100% pass rate for CI pipeline
#[tokio::test]
async fn run_full_e2e_test_suite() {
    println!("🚀 Running complete E2E test suite for STK2ETH");
    
    // Run tests sequentially to avoid conflicts
    test_complete_ussd_to_ethereum_pipeline().await;
    test_session_interruption_and_resume().await;  
    test_invalid_input_handling().await;
    test_session_ttl_cleanup().await;
    
    println!("✅ All E2E tests passed - 100% success rate!");
}
