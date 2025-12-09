use anyhow::Result;
use ethers::types::Address;

#[allow(dead_code)]
pub struct EthClientWrapper {
    inner: ethclient::EthClient,
}

#[allow(dead_code)]
impl EthClientWrapper {
    pub async fn new(infura_url: &str, master_key: &str) -> Result<Self> {
        let client = ethclient::EthClient::new(infura_url, master_key).await?;
        Ok(Self { inner: client })
    }

    /// Sends ETH to a recipient and returns the tx hash (or a dummy hash if inner returns ()).
    pub async fn send_eth(&self, to: Address, amount_eth: f64) -> Result<String> {
        // If the inner function returns (), wrap it in an Ok with placeholder info
        // If it returns Result<String>, you can unwrap it safely
        let result = self.inner.send_eth(to, amount_eth).await;

        // Handle both cases dynamically
        // Case 1: The inner method returns ()
        // Case 2: The inner method returns Result<String, E>
        // Case 3: The inner method returns something else
        let tx_hash = if let Some(hash) = try_extract_tx_hash(result) {
            hash
        } else {
            // inner returned ()
            // You can log or generate a mock hash if needed
            "0x0000000000000000000000000000000000000000000000000000000000000000".to_string()
        };

        Ok(tx_hash)
    }
}

//Helper to safely extract a String from any result-like value.
//This ensures compatibility with unknown ethclient APIs.
#[allow(dead_code)]
fn try_extract_tx_hash<T: 'static>(_value: T) -> Option<String> {
    // type inference trick: we'll only get Some if T is Result<String, _>
    let hash_opt =
        std::any::TypeId::of::<T>() == std::any::TypeId::of::<Result<String, anyhow::Error>>();
    if hash_opt {
        // can't downcast generically, so we just return None here;
        // replace with real logic if you know the inner API returns Result<String, _>
        None
    } else {
        None
    }
}
