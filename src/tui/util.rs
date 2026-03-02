/// Word-wrap text to fit within a given width.
/// Returns a vector of lines, each fitting within `width` characters.
pub fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if current.is_empty() {
            current = word.to_string();
        } else if current.len() + 1 + word.len() <= width {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(current);
            current = word.to_string();
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_text_basic() {
        let result = wrap_text("hello world foo bar", 11);
        assert_eq!(result, vec!["hello world", "foo bar"]);
    }

    #[test]
    fn wrap_text_zero_width() {
        let result = wrap_text("hello world", 0);
        assert_eq!(result, vec!["hello world"]);
    }

    #[test]
    fn wrap_text_single_word() {
        let result = wrap_text("hello", 80);
        assert_eq!(result, vec!["hello"]);
    }

    #[test]
    fn wrap_text_empty() {
        let result = wrap_text("", 80);
        assert!(result.is_empty());
    }

    #[test]
    fn wrap_text_exact_width() {
        let result = wrap_text("ab cd", 5);
        assert_eq!(result, vec!["ab cd"]);
    }
}
