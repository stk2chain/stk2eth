#[derive(Debug, Default, Clone)]
pub struct Params<'a> {
    pub to: Option<&'a str>,
    pub amount: Option<u128>,
}

#[derive(Debug, Clone)]
pub enum TxParams<'a> {
    SendEth { to: &'a str, amount: u128 },
    WithdrawEscrow { to: &'a str, amount: u128 },
    WithdrawRefund { to: &'a str, amount: u128 },
}
