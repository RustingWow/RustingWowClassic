use super::*;

#[test]
fn accepts_user_n_pairs() {
    let account = parse_account("user1").unwrap();
    assert_eq!(account.number, 1);
    assert_eq!(account.username, "USER1");
    assert_eq!(account.password, "PASS1");
    account.verify_password("pass1").unwrap();
    account.verify_password("PASS1").unwrap();
}

#[test]
fn rejects_unknown_names() {
    assert!(parse_account("admin").is_err());
    assert!(parse_account("user").is_err());
    assert!(parse_account("user0").is_err());
    assert!(parse_account("user01").is_err());
}
