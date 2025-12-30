use super::{types::TxType, params::TxParams};


impl<'a> TxParams<'a> {
    pub const fn tx_type(&self) -> Option<TxType> {
        match self {
            Self::SendEth { .. } => Some(TxType::SendEth),
            Self::TokenSwap { .. } => Some(TxType::TokenSwap),
            Self::CashOut { .. } => Some(TxType::CashOut),
            Self::Balance { .. } => Some(TxType::Balance),
        }
    }
}
 

impl TxType {
    pub fn from_ussd_op(s: &str) -> Option<Self> {
        match s {
            "1" => Some(Self::SendEth),
            "2" => Some(Self::TokenSwap),
            "3" => Some(Self::CashOut),
            "4" => Some(Self::Balance),
            _ => None,
        }
    }
}

