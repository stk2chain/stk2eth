use super::{types::TxType, params::{TxParams, Params}};

impl TxType {
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
