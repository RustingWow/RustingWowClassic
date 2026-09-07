use super::*;

#[test]
fn normalizes_alphanumeric_names() {
    assert_eq!(normalize_username("alice").unwrap(), "ALICE");
    assert_eq!(normalize_username("User1").unwrap(), "USER1");
}

#[test]
fn rejects_invalid_names() {
    assert!(normalize_username("a").is_err());
    assert!(normalize_username("thisusernameistoolong").is_err());
    assert!(normalize_username("user_name").is_err());
    assert!(normalize_username("").is_err());
}
