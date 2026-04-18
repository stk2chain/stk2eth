use super::{types::TxType, params::TxParams};

impl<'a> TxParams<'a> {
    pub const fn tx_type(&self) -> TxType {
        match self {
            Self::SendEth { .. } => TxType::SendEth,
            Self::WithdrawEscrow { .. } => TxType::WithdrawEscrow,
            Self::WithdrawRefund { .. } => TxType::WithdrawRefund,
        }
    }
}

impl TxType {
    pub fn from_ussd_op(s: &str) -> Option<Self> {
        match s {
            "1" => Some(Self::SendEth),
            "3" => Some(Self::WithdrawEscrow),
            _ => None,
        }
    }
}
