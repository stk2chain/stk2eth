use ethclient::EthClient;
use ethers::types::U256;
use spacetimedb::Swaps;
use ussdgeth::controller::send_eth_reducer;

#[tokio::test]
async fn test_send_eth_reducer_fails_initially() {
    // Arrange
    let sender = "0xSenderAddress".to_string();
    let recipient = "0xRecipientAddress".to_string();
    let amount = U256::from(1_000_000_000_000_000_000u64); // 1 ETH

    // Check initial balances
    let client = EthClient::new("http://localhost:8545").unwrap();
    let initial_sender_balance = client.get_balance(sender.clone()).await.unwrap();
    let initial_recipient_balance = client.get_balance(recipient.clone()).await.unwrap();

    // Act
    let result = send_eth_reducer(SendEthInput {
        from: sender.clone(),
        to: recipient.clone(),
        amount,
    })
    .await;

    // Assert
    assert!(result.is_err(), "Reducer not yet implemented, should fail");

    let final_sender_balance = client.get_balance(sender).await.unwrap();
    let final_recipient_balance = client.get_balance(recipient).await.unwrap();

    // The balances should remain unchanged
    assert_eq!(initial_sender_balance, final_sender_balance);
    assert_eq!(initial_recipient_balance, final_recipient_balance);
}
