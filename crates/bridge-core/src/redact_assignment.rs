pub(super) struct Value {
    pub start: usize,
    pub end: usize,
    pub next: usize,
    pub sensitive: bool,
}
fn identifier(c: char) -> bool {
    c.is_alphanumeric() || "_-.".contains(c)
}
fn spaces(s: &str, at: &mut usize) {
    while matches!(s.as_bytes().get(*at), Some(b' ' | b'\t')) {
        *at += 1;
    }
}
pub(super) fn parse(s: &str, i: usize) -> Option<Value> {
    if s[..i].chars().next_back().is_some_and(identifier) {
        return None;
    }
    let mut at = i;
    if s[at..]
        .get(..5)
        .is_some_and(|v| v.eq_ignore_ascii_case("$env:"))
    {
        at += 5;
    }
    let key_quote = s
        .as_bytes()
        .get(at)
        .copied()
        .filter(|b| matches!(b, b'"' | b'\''));
    if key_quote.is_some() {
        at += 1;
    }
    let start = at;
    at += s[at..].find(|c| !identifier(c)).unwrap_or(s.len() - at);
    if start == at {
        return None;
    }
    let raw_key = &s[start..at];
    let key = raw_key.to_lowercase();
    if let Some(quote) = key_quote {
        if s.as_bytes().get(at) != Some(&quote) {
            return None;
        }
        at += 1;
    }
    spaces(s, &mut at);
    if !matches!(s.as_bytes().get(at), Some(b'=' | b':')) {
        return None;
    }
    at += 1;
    spaces(s, &mut at);
    // URL 的冒号不是赋值；对象、数组继续由外层逐字扫描其内层字段。
    if s[at..].starts_with("//")
        || s[at..].starts_with(['{', '[']) && !s[at..].starts_with("[已打码：")
    {
        return None;
    }
    let sensitive = [
        "password",
        "passwd",
        "pwd",
        "secret",
        "token",
        "api_key",
        "apikey",
        "api-key",
        "access_key",
        "private_key",
        "credential",
    ]
    .iter()
    .any(|keyword| key.contains(keyword))
        || auth_segment(raw_key);
    let quote = s
        .as_bytes()
        .get(at)
        .copied()
        .filter(|b| matches!(b, b'"' | b'\''));
    if quote.is_some() {
        at += 1;
    }
    let start = at;
    let (end, next) = if let Some(quote) = quote {
        let mut escaped = false;
        let mut end = s.len();
        for (offset, c) in s[at..].char_indices() {
            if c as u32 == u32::from(quote) && !escaped {
                end = at + offset;
                break;
            }
            escaped = c == '\\' && !escaped;
        }
        (end, end + usize::from(end < s.len()))
    } else if let Some(length) = super::private_key(&s[at..]) {
        (at + length, at + length)
    } else if s[at..].starts_with("[已打码：") {
        let end = at + s[at..].find(']')? + 1;
        (end, end)
    } else {
        let mut end = at
            + s[at..]
                .find(|c: char| c.is_whitespace() || c == ',')
                .unwrap_or(s.len() - at);
        // Authorization: Bearer xxx 的完整凭据仍由既有 Bearer 规则覆盖。
        if s[at..end].eq_ignore_ascii_case("Bearer") && s.as_bytes().get(end) == Some(&b' ') {
            end += 1;
            end += super::end_token(&s[end..]);
        }
        (end, end)
    };
    Some(Value {
        start,
        end,
        next,
        sensitive: sensitive && !s[start..end].starts_with("[已打码："),
    })
}

fn auth_segment(key: &str) -> bool {
    let chars: Vec<char> = key.chars().collect();
    let mut parts = String::new();
    for (i, &c) in chars.iter().enumerate() {
        if "_-.".contains(c)
            || (i > 0
                && c.is_uppercase()
                && (chars[i - 1].is_lowercase()
                    || chars.get(i + 1).is_some_and(|c| c.is_lowercase())))
        {
            parts.push(' ');
        }
        if !"_-.".contains(c) {
            parts.extend(c.to_lowercase());
        }
    }
    parts.split_whitespace().any(|s| s == "auth")
}
