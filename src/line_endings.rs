use std::borrow::Cow;

/// 비교에서만 CRLF를 LF와 동일하게 취급하고 마지막 개행은 유지합니다.
pub fn comparison_text(text: &str) -> Cow<'_, str> {
    if text.contains("\r\n") {
        Cow::Owned(text.replace("\r\n", "\n"))
    } else {
        Cow::Borrowed(text)
    }
}

pub fn preferred(primary: &str, fallback: &str) -> &'static str {
    let total = primary.bytes().filter(|byte| *byte == b'\n').count();
    if total == 0 {
        if fallback.bytes().any(|byte| byte == b'\n') {
            return preferred(fallback, "");
        }
        return "\n";
    }
    let crlf = primary.matches("\r\n").count();
    if crlf > total - crlf { "\r\n" } else { "\n" }
}

pub fn push_line(result: &mut String, raw: &str, ending: &str) {
    if let Some(text) = raw.strip_suffix("\r\n").or_else(|| raw.strip_suffix('\n')) {
        result.push_str(text);
        result.push_str(ending);
    } else {
        result.push_str(raw);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn normalization_preserves_spaces_tabs_bare_cr_and_missing_final_newline() {
        assert_eq!(comparison_text("a \r\n\tlast"), "a \n\tlast");
        assert_eq!(comparison_text("a\rb"), "a\rb");
        assert!(matches!(comparison_text("a\nlast"), Cow::Borrowed(_)));
        assert_eq!(preferred("a\r\nb\nc\r\n", ""), "\r\n");
        assert_eq!(preferred("", "a\r\n"), "\r\n");
        let mut result = String::new();
        push_line(&mut result, "one\n", "\r\n");
        push_line(&mut result, "last", "\r\n");
        assert_eq!(result, "one\r\nlast");
    }
}
