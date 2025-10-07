#[cfg(test)]
mod amount_validation_tests {
    use crate::AmountValidationResult;

    #[test]
    fn test_validate_amount_greater_than_or_equal_to_minimum() {
        let amount = "0.00025";
        let result = crate::reducers::amount_validation::validate_amount_value(amount, 0.00025);
        assert_eq!(result, AmountValidationResult::Valid);
    }

    #[test]
    fn test_validate_amount_too_low_and_invalid() {
        let low = "0.0001";
        assert_eq!(crate::reducers::amount_validation::validate_amount_value(low, 0.00025), AmountValidationResult::TooLow);

        let invalid = "abc";
        assert_eq!(crate::reducers::amount_validation::validate_amount_value(invalid, 0.00025), AmountValidationResult::Invalid);
    }
}
