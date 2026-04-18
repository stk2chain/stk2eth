#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserIntent {
    RegisterPin { pin: String },
    ConfirmRegisterPin { pin: String, confirm: String },
    SendEthPhone { phone: String },
    SendEthAmount { phone: String, amount: String },
    SendEthPin { phone: String, amount: String, pin: String },
    SendEthConfirm { phone: String, amount: String, pin: String, confirm: ConfirmDecision },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfirmDecision {
    Confirm,
    Cancel,
}

pub fn parse_intent(screen_name: &str, data: &str) -> Result<UserIntent, String> {
    let parts: Vec<&str> = data.split('*').collect();

    match screen_name {
        "RegisterPinScreen" => {
            if parts.len() != 2 { return Err("expected 1*PIN".to_string()); }
            Ok(UserIntent::RegisterPin { pin: parts[1].to_string() })
        }
        "ConfirmRegisterPinScreen" => {
            if parts.len() != 3 { return Err("expected 1*PIN*CONFIRM".to_string()); }
            Ok(UserIntent::ConfirmRegisterPin {
                pin: parts[1].to_string(),
                confirm: parts[2].to_string(),
            })
        }
        "ToNumberScreen" => {
            if parts.len() != 2 { return Err("expected 1*PHONE".to_string()); }
            Ok(UserIntent::SendEthPhone { phone: parts[1].to_string() })
        }
        "ToAmountScreen" => {
            if parts.len() != 3 { return Err("expected 1*PHONE*AMOUNT".to_string()); }
            Ok(UserIntent::SendEthAmount {
                phone: parts[1].to_string(),
                amount: parts[2].to_string(),
            })
        }
        "PINScreen" => {
            if parts.len() != 4 { return Err("expected 1*PHONE*AMOUNT*PIN".to_string()); }
            Ok(UserIntent::SendEthPin {
                phone: parts[1].to_string(),
                amount: parts[2].to_string(),
                pin: parts[3].to_string(),
            })
        }
        "CancelTXScreen" => {
            if parts.len() != 5 {
                return Err("expected 1*PHONE*AMOUNT*PIN*CONFIRM".to_string());
            }
            let confirm = match parts[4] {
                "1" => ConfirmDecision::Confirm,
                "2" => ConfirmDecision::Cancel,
                other => return Err(format!("invalid confirm value '{}'", other)),
            };
            Ok(UserIntent::SendEthConfirm {
                phone: parts[1].to_string(),
                amount: parts[2].to_string(),
                pin: parts[3].to_string(),
                confirm,
            })
        }
        other => Err(format!("no intent parser for screen '{}'", other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_register_pin() {
        let got = parse_intent("RegisterPinScreen", "1*1234").unwrap();
        assert_eq!(got, UserIntent::RegisterPin { pin: "1234".to_string() });
    }

    #[test]
    fn parses_confirm_register_pin() {
        let got = parse_intent("ConfirmRegisterPinScreen", "1*1234*1234").unwrap();
        assert_eq!(got, UserIntent::ConfirmRegisterPin {
            pin: "1234".to_string(),
            confirm: "1234".to_string(),
        });
    }

    #[test]
    fn parses_send_eth_pin() {
        let got = parse_intent("PINScreen", "1*254712345678*0.5*9876").unwrap();
        assert_eq!(got, UserIntent::SendEthPin {
            phone: "254712345678".to_string(),
            amount: "0.5".to_string(),
            pin: "9876".to_string(),
        });
    }

    #[test]
    fn parses_send_eth_confirm_accept() {
        let got = parse_intent("CancelTXScreen", "1*254712345678*0.5*9876*1").unwrap();
        match got {
            UserIntent::SendEthConfirm { confirm: ConfirmDecision::Confirm, .. } => {}
            other => panic!("expected Confirm, got {:?}", other),
        }
    }

    #[test]
    fn parses_send_eth_confirm_cancel() {
        let got = parse_intent("CancelTXScreen", "1*254712345678*0.5*9876*2").unwrap();
        match got {
            UserIntent::SendEthConfirm { confirm: ConfirmDecision::Cancel, .. } => {}
            other => panic!("expected Cancel, got {:?}", other),
        }
    }

    #[test]
    fn rejects_wrong_part_count() {
        assert!(parse_intent("PINScreen", "1*254712345678*0.5").is_err());
    }

    #[test]
    fn rejects_unknown_screen() {
        assert!(parse_intent("NonexistentScreen", "1").is_err());
    }

    #[test]
    fn rejects_invalid_confirm_value() {
        assert!(parse_intent("CancelTXScreen", "1*254*0.5*1234*9").is_err());
    }
}
