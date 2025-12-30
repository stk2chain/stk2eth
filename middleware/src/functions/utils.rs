pub fn parse_input(user_input: &str) -> Vec<&str> {
    user_input.split('*').collect()
}