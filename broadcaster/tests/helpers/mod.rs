// broadcaster/tests/helpers/mod.rs

use std::process::{Child, Command, Stdio};
use std::time::Duration;

pub struct Anvil {
    pub endpoint: String,
    pub ws_endpoint: String,
    pub chain_id: u64,
    _child: Child,
}

impl Anvil {
    pub fn spawn_base_sepolia_fork(port: u16) -> Self {
        let child = Command::new("anvil")
            .args([
                "--fork-url",
                "https://sepolia.base.org",
                "--port",
                &port.to_string(),
                "--chain-id",
                "84532",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("anvil not found on PATH");
        std::thread::sleep(Duration::from_secs(2));
        Anvil {
            endpoint: format!("http://127.0.0.1:{port}"),
            ws_endpoint: format!("ws://127.0.0.1:{port}"),
            chain_id: 84532,
            _child: child,
        }
    }

    pub async fn set_balance(&self, addr: &str, wei: &str) {
        let client = reqwest::Client::new();
        let body = serde_json::json!({
            "jsonrpc": "2.0", "method": "anvil_setBalance",
            "params": [addr, wei], "id": 1,
        });
        client
            .post(&self.endpoint)
            .json(&body)
            .send()
            .await
            .unwrap();
    }

    pub async fn set_code(&self, addr: &str, code: &str) {
        let client = reqwest::Client::new();
        let body = serde_json::json!({
            "jsonrpc": "2.0", "method": "anvil_setCode",
            "params": [addr, code], "id": 1,
        });
        client
            .post(&self.endpoint)
            .json(&body)
            .send()
            .await
            .unwrap();
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

pub fn stdb_available() -> bool {
    Command::new("spacetime").arg("--version").output().is_ok()
}

pub fn stdb_reset(db: &str) {
    let _ = Command::new("spacetime").args(["delete", db]).output();
    Command::new("spacetime")
        .args(["publish", "--project-path", "../middleware", db])
        .status()
        .expect("spacetime publish failed");
    Command::new("spacetime")
        .args(["call", db, "claim_gateway_identity"])
        .status()
        .expect("claim_gateway_identity failed");
}

pub fn stdb_sql(db: &str, query: &str) -> String {
    let out = Command::new("spacetime")
        .args(["sql", db, query])
        .output()
        .expect("spacetime sql failed");
    String::from_utf8_lossy(&out.stdout).to_string()
}

pub fn stdb_call(db: &str, reducer: &str, args: &[&str]) {
    let mut cmd = Command::new("spacetime");
    cmd.args(["call", db, reducer]);
    for a in args {
        cmd.arg(a);
    }
    cmd.status().expect("spacetime call failed");
}
