use anyhow::Result;
use ethers::prelude::*;

pub struct EthClient {
    provider: Provider<Http>,
}

impl EthClient {
    pub async fn new(infura_url: &str, _master_key: &str) -> Result<Self> {
        let provider = Provider::<Http>::try_from(infura_url)?;
        Ok(Self { provider })
    }

    // Example method
    pub async fn get_balance(&self, address: &str) -> Result<U256> {
        let addr: Address = address.parse()?;
        let balance = self.provider.get_balance(addr, None).await?;
        Ok(balance)
    }
}
