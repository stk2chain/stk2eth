
// helper function to check if string contains only digits
fn is_all_digits(s: &str) -> bool {
    s.chars().all(|c| c.is_ascii_digit())
}


pub fn register_pin(user_input: &str) -> Result<(), String> {
    user_input
        .chars()
        .count()
        .eq(&4)
        .then(|| ())
        .ok_or_else(|| "PIN must be exactly 4 digits".to_string())
        .and_then(|_| {
            if is_all_digits(user_input) {
                Ok(())
            } else {
                Err("PIN must contain only digits".to_string())
            }
        })    
}

pub fn confirm_register_pin(user_input: &str) -> Result<(), String> {
    Ok(())
    
}
