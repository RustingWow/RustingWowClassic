use super::parse_bool_value;

#[test]
fn parses_bool_flags() {
    assert_eq!(parse_bool_value("1"), Some(true));
    assert_eq!(parse_bool_value("TRUE"), Some(true));
    assert_eq!(parse_bool_value("off"), Some(false));
    assert_eq!(parse_bool_value("maybe"), None);
}
