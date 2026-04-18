use super::{types::TxType, params::{TxParams, Params}};
use super::selector::keccak_selector;

fn uint256(v: u128) -> [u8; 32] {
    let mut buf = [0u8; 32];
    buf[16..].copy_from_slice(&v.to_be_bytes());
    buf
}

fn address(addr: &str) -> [u8; 32] {
    let clean = addr.strip_prefix("0x").unwrap_or(addr);
    let mut buf = [0u8; 32];
    if clean.len() >= 40 {
        if let Ok(bytes) = hex::decode(&clean[..40]) {
            buf[12..32].copy_from_slice(&bytes);
        }
    }
    buf
}

fn concat(parts: &[&[u8]]) -> Vec<u8> {
    parts.iter().flat_map(|&p| p.iter().copied()).collect()
}

pub fn to_hex(data: &[u8]) -> String {
    data.iter().fold(String::from("0x"), |mut s, b| {
        s.push_str(&format!("{:02x}", b));
        s
    })
}

impl TxType {
    pub const fn signature(&self) -> &'static str {
        match self {
            Self::SendEth => "",
            Self::WithdrawEscrow => "",
            Self::WithdrawRefund => "",
        }
    }

    pub fn selector(&self) -> Option<[u8; 4]> {
        let sig = self.signature();
        if sig.is_empty() {
            None
        } else {
            Some(keccak_selector(sig))
        }
    }

    pub fn to_tx<'a>(&self, params: Params<'a>) -> TxParams<'a> {
        let to = params.to.unwrap_or("");
        let amount = params.amount.unwrap_or(0);
        match self {
            Self::SendEth => TxParams::SendEth { to, amount },
            Self::WithdrawEscrow => TxParams::WithdrawEscrow { to, amount },
            Self::WithdrawRefund => TxParams::WithdrawRefund { to, amount },
        }
    }
}

impl<'a> TxParams<'a> {
    pub fn encode(&self) -> Vec<u8> {
        Vec::new()
    }

    pub fn to_hex(&self) -> String {
        to_hex(&self.encode())
    }
}
