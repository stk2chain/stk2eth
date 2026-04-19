// broadcaster/tests/helpers/mod.rs
// Spawns anvil as a forked Base Sepolia chain and provides a helper to set
// code + balance on burner addresses, plus a local SpacetimeDB harness.

use std::process::{Child, Command, Stdio};
use std::time::Duration;

pub struct Anvil {
    pub endpoint: String,
    pub ws_endpoint: String,
    pub chain_id: u64,
    _child: Child,
}

impl Anvil {
    pub fn spawn_base_sepolia_fork() -> Self {
        let port = 8545;
        let child = Command::new("anvil")
            .args([
                "--fork-url",
                "https://sepolia.base.org",
                "--port",
                &port.to_string(),
                "--chain-id",
                "84532",
                "--no-mining",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("anvil not found on PATH — install Foundry");

        std::thread::sleep(Duration::from_secs(2));

        Anvil {
            endpoint: format!("http://127.0.0.1:{port}"),
            ws_endpoint: format!("ws://127.0.0.1:{port}"),
            chain_id: 84532,
            _child: child,
        }
    }
}

impl Drop for Anvil {
    fn drop(&mut self) {
        let _ = self._child.kill();
    }
}

pub fn anvil_available() -> bool {
    Command::new("anvil").arg("--version").output().is_ok()
}
