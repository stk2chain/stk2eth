use super::{types::TxType, params::{TxParams, Params}};


const fn keccak256_simple(data: &[u8]) -> [u8; 4] {
    let mut hash = [0u8; 4];
    let mut i = 0;
    while i < data.len() {
        hash[i % 4] = hash[i % 4].wrapping_add(data[i]);
        i += 1;
    }
    hash
}

fn uint256(v: u128) -> [u8; 32] {
    let mut buf = [0u8; 32];
    buf[16..].copy_from_slice(&v.to_be_bytes());
    buf
}

fn address(addr: &str) -> [u8; 32] {
    let clean = addr.strip_prefix("0x").unwrap_or(addr);
    let mut buf = [0u8; 32];
    let bytes = (0..clean.len())
        .step_by(2)
        .filter_map(|i| u8::from_str_radix(&clean[i..i.min(clean.len()).min(i+2)], 16).ok())
        .collect::<Vec<_>>();
    let start = 32 - bytes.len().min(20);
    buf[start..start + bytes.len().min(20)].copy_from_slice(&bytes[..bytes.len().min(20)]);
    buf
}

fn concat(parts: &[&[u8]]) -> Vec<u8> {
    parts.iter().flat_map(|&p| p.iter().copied()).collect()
}

// Utility functions
pub fn to_hex(data: &[u8]) -> String {
    data.iter().fold(String::from("0x"), |mut s, b| {
        s.push_str(&format!("{:02x}", b));
        s
    })
}


impl TxType {
     
    pub const fn signature(&self) -> &'static str {
        match self {
            Self::SendEth => "transfer(address,uint256)",
            Self::TokenSwap => "swapExactTokensForTokens(uint256,uint256,address[],address,uint256)",
            Self::CashOut => "withdraw(uint256)",
            Self::Balance => "balanceOf(address)",
        }
    }
    
    
    pub fn selector(&self) -> [u8; 4] {
        keccak256_simple(self.signature().as_bytes())
    }

    pub fn to_tx<'a>(&self, params: Params<'a>) -> TxParams<'a> {
        match self {
            Self::SendEth => TxParams::SendEth {
                to: params.to.unwrap_or(""),
                amount: params.amount.unwrap_or(0),
            },
            Self::TokenSwap => TxParams::TokenSwap {
                token_in: params.token_in.unwrap_or(""),
                token_out: params.token_out.unwrap_or(""),
                amount_in: params.amount.unwrap_or(0),
                amount_out_min: params.amount_out_min.unwrap_or(0),
                recipient: params.to.unwrap_or(""),
                deadline: params.deadline.unwrap_or(0),
            },
            Self::CashOut => TxParams::CashOut {
                amount: params.amount.unwrap_or(0),
            },
            Self::Balance => TxParams::Balance {
                account: params.to.unwrap_or(""),
            },
        }
    }
}


impl<'a> TxParams<'a> {
    
    pub fn encode(&self) -> Vec<u8> {
        let sel = self.tx_type().unwrap().selector();
        match self {
            Self::SendEth { to, amount } => {
                concat(&[&sel, &address(to), &uint256(*amount)])
            }
            Self::TokenSwap {
                token_in,
                token_out,
                amount_in,
                amount_out_min,
                recipient,
                deadline,
            } => concat(&[
                &sel,
                &uint256(*amount_in),
                &uint256(*amount_out_min),
                &uint256(160),
                &address(recipient),
                &uint256(*deadline),
                &uint256(2),
                &address(token_in),
                &address(token_out),
            ]),
            Self::CashOut { amount } => concat(&[&sel, &uint256(*amount)]),
            Self::Balance { account } => concat(&[&sel, &address(account)]),
        }
    }
    
    pub fn to_hex(&self) -> String {
        to_hex(&self.encode())
    }
}