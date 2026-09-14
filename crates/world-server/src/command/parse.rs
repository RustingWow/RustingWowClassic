pub fn is_command(text: &str) -> bool {
    let trimmed = text.trim_start();
    trimmed.starts_with('.') || trimmed.starts_with('!')
}

pub fn tokenize(text: &str) -> Vec<String> {
    let trimmed = text.trim();
    let body = trimmed
        .strip_prefix('.')
        .or_else(|| trimmed.strip_prefix('!'))
        .unwrap_or(trimmed)
        .trim();
    if body.is_empty() {
        return Vec::new();
    }
    body.split_whitespace().map(str::to_string).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_dot_and_bang() {
        assert!(is_command(".gps"));
        assert!(is_command("  !tele Stormwind"));
        assert!(!is_command("hello"));
        assert!(!is_command(""));
    }

    #[test]
    fn splits_command_and_args() {
        let tokens = tokenize(".tele Stormwind Keep");
        assert_eq!(tokens, ["tele", "Stormwind", "Keep"]);
        assert!(tokenize(".").is_empty());
    }
}
