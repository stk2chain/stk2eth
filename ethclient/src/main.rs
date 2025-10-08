// ethclient/src/main.rs
// Eth client using ethers-rs. Exposes EthClient struct with new(), get_balance(), send_eth().
// This file is both a binary (for quick testing) and a library entry (you can refactor to lib.rs if desired).

use anyhow::{anyhow, Result};
use dotenv::dotenv;
use ethers::prelude::*;
use std::env;
use std::time::Duration;
use tracing::{info, warn};

#[derive(Clone)]
pub struct EthClient {
    provider: Provider<Http>,
    wallet: LocalWallet,
    signer: SignerMiddleware<Provider<Http>, LocalWallet>,
}

impl EthClient {
    /// Create new EthClient from RPC URL and private key (hex or 0x...).
    pub async fn new(rpc_url: &str, private_key: &str) -> Result<Self> {
        // provider
        let provider = Provider::<Http>::try_from(rpc_url)
            .map_err(|e| anyhow!("invalid rpc url: {:?}", e))?
            .interval(Duration::from_millis(600u64));

        // wallet
        let wallet: LocalWallet = private_key
            .parse::<LocalWallet>()
            .map_err(|e| anyhow!("invalid private key: {:?}", e))?;

        // get chain id and set on wallet
        let chain_id = provider.get_chainid().await?.as_u64();
        let wallet = wallet.with_chain_id(chain_id);

        // signer middleware
        let signer = SignerMiddleware::new(provider.clone(), wallet.clone());

        Ok(Self {
            provider,
            wallet,
            signer,
        })
    }

    /// Get balance for address (wei)
    pub async fn get_balance_wei(&self, address: Address) -> Result<U256> {
        Ok(self.provider.get_balance(address, None).await?)
    }

    /// Send ETH: amount_eth is decimal ETH (e.g., 0.01)
    /// returns tx hash hex string (0x...)
    pub async fn send_eth(&self, to: Address, amount_eth: f64) -> Result<String> {
        if amount_eth <= 0.0 {
            return Err(anyhow!("amount must be > 0"));
        }

        let value = ethers::utils::parse_ether(amount_eth)?;

        let from = self.wallet.address();
        let nonce = self.provider.get_transaction_count(from, None).await?;
        let gas_price = self.provider.get_gas_price().await?;

        let txreq = TransactionRequest::new()
            .to(to)
            .value(value)
            .from(from)
            .nonce(nonce);

        let gas_estimate = match self
            .provider
            .estimate_gas(&txreq.clone().into(), None)
            .await
        {
            Ok(g) => g,
            Err(e) => {
                warn!("gas estimate failed: {:?}, falling back to 21000", e);
                U256::from(21_000u64)
            }
        };

        let tx = TransactionRequest {
            to: Some(NameOrAddress::Address(to)),
            value: Some(value),
            gas: Some(gas_estimate),
            gas_price: Some(gas_price),
            nonce: Some(nonce),
            ..Default::default()
        };

        let pending = self.signer.send_transaction(tx, None).await?;
        let tx_hash = *pending;
        // not awaiting confirmation here; controller can choose to wait
        Ok(format!("{:#x}", tx_hash))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    let rpc = env::var("INFURA_URL").expect("INFURA_URL env required");
    // prefer MASTER_PRIVATE_KEY if set (hot wallet); otherwise error
    let key = env::var("MASTER_PRIVATE_KEY").expect("MASTER_PRIVATE_KEY env required");

    let client = EthClient::new(&rpc, &key).await?;
    info!("Eth client ready.");

    // quick smoke-check
    let addr = client.wallet.address();
    let bal = client.get_balance_wei(addr).await?;
    info!("Wallet {} balance (wei): {}", addr, bal);

    Ok(())
}
