// Generic parameters for SwapType construction
#[derive(Debug, Default, Clone)]
pub struct Params<'a> {
    pub to: Option<&'a str>,
    pub amount: Option<u128>,
    pub token_in: Option<&'a str>,
    pub token_out: Option<&'a str>,
    pub amount_out_min: Option<u128>,
    pub deadline: Option<u128>,
}

#[derive(Debug, Clone)]
pub enum TxParams<'a> {
    SendEth { to: &'a str, amount: u128 },
    TokenSwap {
        token_in: &'a str,
        token_out: &'a str,
        amount_in: u128,
        amount_out_min: u128,
        recipient: &'a str,
        deadline: u128,
    },
    CashOut { amount: u128 },
    Balance { account: &'a str },
}