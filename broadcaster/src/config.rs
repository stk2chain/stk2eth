// broadcaster/src/config.rs
use crate::error::BroadcasterError;
use alloy::primitives::{Address, U256};
use secrecy::SecretString;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug)]
pub struct Config {
    pub rpc_url: String,
    pub rpc_ws_url: String,
    pub chain_id: u64,
    pub burner7702_address: Address,

    pub operator_keystore_path: PathBuf,
    pub operator_keystore_passphrase: SecretString,

    pub spacetime_host: String,
    pub spacetime_db_name: String,
    pub spacetime_auth_token: SecretString,

    pub balance_reserve_wei: U256,
    pub reconcile_scan_blocks: u64,
    pub rate_limit_backoff_max_secs: u64,
}

impl Config {
    pub fn from_env() -> Result<Self, BroadcasterError> {
        let req = |name: &str| -> Result<String, BroadcasterError> {
            std::env::var(name).map_err(|_| BroadcasterError::Config(format!("missing {name}")))
        };
        let opt = |name: &str, default: &str| -> String {
            std::env::var(name).unwrap_or_else(|_| default.to_string())
        };

        let rpc_url = req("RPC_URL")?;
        let rpc_ws_url = req("RPC_WS_URL")?;
        let chain_id: u64 = req("CHAIN_ID")?.parse()
            .map_err(|e| BroadcasterError::Config(format!("CHAIN_ID: {e}")))?;
        let burner7702_address = Address::from_str(&req("BURNER7702_ADDRESS")?)
            .map_err(|e| BroadcasterError::Config(format!("BURNER7702_ADDRESS: {e}")))?;

        let operator_keystore_path: PathBuf = req("OPERATOR_KEYSTORE_PATH")?.into();
        if !operator_keystore_path.is_file() {
            return Err(BroadcasterError::Config(format!(
                "OPERATOR_KEYSTORE_PATH not a readable file: {}", operator_keystore_path.display())));
        }
        let operator_keystore_passphrase = SecretString::new(req("OPERATOR_KEYSTORE_PASSPHRASE")?.into());

        let spacetime_host = req("SPACETIME_HOST")?;
        let spacetime_db_name = req("SPACETIME_DB_NAME")?;
        let spacetime_auth_token = SecretString::new(req("SPACETIME_AUTH_TOKEN")?.into());

        let balance_reserve_wei = U256::from_str_radix(
            &opt("BALANCE_RESERVE_WEI", "500000000000000"), 10)
            .map_err(|e| BroadcasterError::Config(format!("BALANCE_RESERVE_WEI: {e}")))?;
        let reconcile_scan_blocks: u64 = opt("RECONCILE_SCAN_BLOCKS", "20").parse()
            .map_err(|e| BroadcasterError::Config(format!("RECONCILE_SCAN_BLOCKS: {e}")))?;
        let rate_limit_backoff_max_secs: u64 = opt("RATE_LIMIT_BACKOFF_MAX_SECS", "30").parse()
            .map_err(|e| BroadcasterError::Config(format!("RATE_LIMIT_BACKOFF_MAX_SECS: {e}")))?;

        Ok(Config {
            rpc_url, rpc_ws_url, chain_id, burner7702_address,
            operator_keystore_path, operator_keystore_passphrase,
            spacetime_host, spacetime_db_name, spacetime_auth_token,
            balance_reserve_wei, reconcile_scan_blocks, rate_limit_backoff_max_secs,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn clear_broadcaster_env() {
        for k in ["RPC_URL","RPC_WS_URL","CHAIN_ID","BURNER7702_ADDRESS",
                  "OPERATOR_KEYSTORE_PATH","OPERATOR_KEYSTORE_PASSPHRASE",
                  "SPACETIME_HOST","SPACETIME_DB_NAME","SPACETIME_AUTH_TOKEN",
                  "BALANCE_RESERVE_WEI","RECONCILE_SCAN_BLOCKS","RATE_LIMIT_BACKOFF_MAX_SECS"] {
            std::env::remove_var(k);
        }
    }

    #[test]
    fn missing_required_returns_config_error() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        clear_broadcaster_env();
        let err = Config::from_env().unwrap_err();
        assert!(matches!(err, BroadcasterError::Config(ref m) if m.contains("RPC_URL")),
            "expected config error for RPC_URL, got {err:?}");
    }

    #[test]
    fn all_required_present_parses_ok() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        clear_broadcaster_env();
        let ks = tempfile::NamedTempFile::new().unwrap();
        std::env::set_var("RPC_URL", "http://rpc");
        std::env::set_var("RPC_WS_URL", "ws://rpc");
        std::env::set_var("CHAIN_ID", "84532");
        std::env::set_var("BURNER7702_ADDRESS", "0x1ff0A24D1145d58Ea6F190569C10916E1a78F013");
        std::env::set_var("OPERATOR_KEYSTORE_PATH", ks.path());
        std::env::set_var("OPERATOR_KEYSTORE_PASSPHRASE", "pw");
        std::env::set_var("SPACETIME_HOST", "http://localhost:3000");
        std::env::set_var("SPACETIME_DB_NAME", "gateway2");
        std::env::set_var("SPACETIME_AUTH_TOKEN", "tok");
        let cfg = Config::from_env().unwrap();
        assert_eq!(cfg.chain_id, 84532);
        assert_eq!(cfg.reconcile_scan_blocks, 20);
    }
}
