#[cfg(test)]
mod amount_validation_tests {
    use crate::AmountValidationResult;
    use rust_decimal::prelude::FromStr;
    use rust_decimal::Decimal;

    #[test]
    fn test_validate_amount_greater_than_or_equal_to_minimum() {
        let amount = "0.00025";
        let min = Decimal::from_str("0.00025").unwrap();
        let result = crate::reducers::amount_validation::validate_amount_value(amount, min);
        assert_eq!(result, AmountValidationResult::Valid);
    }

    #[test]
    fn test_validate_amount_too_low_and_invalid() {
        let low = "0.0001";
        let min = Decimal::from_str("0.00025").unwrap();
        assert_eq!(
            crate::reducers::amount_validation::validate_amount_value(low, min),
            AmountValidationResult::TooLow
        );

        let invalid = "abc";
        assert_eq!(
            crate::reducers::amount_validation::validate_amount_value(invalid, min),
            AmountValidationResult::Invalid
        );
    }
}
