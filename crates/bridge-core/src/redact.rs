use std::collections::BTreeMap;
#[derive(Default)]
pub struct Redacted {
    pub text: String,
    pub counts: BTreeMap<&'static str, usize>,
}
fn word(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}
fn token(c: char) -> bool {
    c.is_ascii_alphanumeric() || "_-./+=~".contains(c)
}
fn end_token(s: &str) -> usize {
    s.find(|c| !token(c)).unwrap_or(s.len())
}
fn boundary(s: &str, i: usize) -> bool {
    i == 0 || !word(s.as_bytes()[i - 1])
}
fn assignment(s: &str, i: usize) -> Option<(usize, usize)> {
    if !boundary(s, i) {
        return None;
    }
    for name in [
        "password", "passwd", "pwd", "secret", "token", "api_key", "apikey",
    ] {
        let tail = &s[i..];
        let Some(key) = tail.get(..name.len()) else {
            continue;
        };
        if !key.eq_ignore_ascii_case(name) {
            continue;
        }
        let mut at = i + name.len();
        if s.as_bytes().get(at).is_some_and(|b| word(*b)) {
            continue;
        }
        if matches!(s.as_bytes().get(at), Some(b'"' | b'\'')) {
            at += 1;
        }
        while matches!(s.as_bytes().get(at), Some(b' ' | b'\t')) {
            at += 1;
        }
        if !matches!(s.as_bytes().get(at), Some(b'=' | b':')) {
            continue;
        }
        at += 1;
        while matches!(s.as_bytes().get(at), Some(b' ' | b'\t')) {
            at += 1;
        }
        if matches!(s.as_bytes().get(at), Some(b'"' | b'\'')) {
            let quote = s.as_bytes()[at];
            at += 1;
            if s[at..].starts_with("[已打码：") {
                return None;
            }
            let start = at;
            let mut escaped = false;
            for (off, c) in s[at..].char_indices() {
                if c as u32 == u32::from(quote) && !escaped {
                    return Some((start, at + off));
                }
                escaped = c == '\\' && !escaped;
            }
            return Some((start, s.len()));
        }
        if s[at..].starts_with("[已打码：") {
            return None;
        }
        let end = s[at..]
            .find(|c: char| c.is_whitespace() || r#",;}]"'<>`"#.contains(c))
            .map_or(s.len(), |n| at + n);
        if end > at {
            return Some((at, end));
        }
    }
    None
}
fn private_key(s: &str) -> Option<usize> {
    if !s.starts_with("-----BEGIN ") {
        return None;
    }
    let header = s[11..].find("-----")? + 11;
    let label = &s[11..header];
    if !label.ends_with("PRIVATE KEY") {
        return None;
    }
    let footer = format!("-----END {label}-----");
    Some(s.find(&footer).map_or(s.len(), |n| n + footer.len()))
}
pub fn scan(s: &str) -> Redacted {
    let mut out = Redacted::default();
    let mut i = 0;
    while i < s.len() {
        if s[i..].starts_with("[已打码：") {
            if let Some(end) = s[i..].find(']') {
                out.text.push_str(&s[i..=i + end]);
                i += end + 1;
                continue;
            }
        }
        let mut found = None;
        if let Some(n) = private_key(&s[i..]) {
            found = Some((i, i + n, "私钥"));
        }
        if found.is_none() && boundary(s, i) {
            for prefix in [
                "sk-ant-",
                "sk-",
                "github_pat_",
                "ghp_",
                "gho_",
                "AKIA",
                "xoxb-",
                "xoxa-",
                "xoxp-",
                "xoxr-",
                "xoxs-",
                "AIza",
            ] {
                if s[i..].starts_with(prefix) {
                    let n = end_token(&s[i..]);
                    if n >= prefix.len() + 4 {
                        found = Some((i, i + n, "密钥"));
                    }
                    break;
                }
            }
            if s[i..]
                .get(..7)
                .is_some_and(|v| v.eq_ignore_ascii_case("Bearer "))
            {
                let start = i + 7;
                let n = end_token(&s[start..]);
                if n > 0 {
                    found = Some((start, start + n, "Bearer"));
                }
            }
        }
        if found.is_none() {
            if let Some((start, end)) = assignment(s, i) {
                if end > start {
                    found = Some((start, end, "赋值凭据"));
                }
            }
        }
        if found.is_none() && s[i..].starts_with("://") {
            let start = i + 3;
            let end = s[start..]
                .find(|c: char| c.is_whitespace() || "/?#".contains(c))
                .map_or(s.len(), |n| start + n);
            if let Some(at) = s[start..end].rfind('@') {
                if s[start..start + at].contains(':') {
                    found = Some((start, start + at, "URL凭据"));
                }
            }
        }
        if let Some((start, end, kind)) = found {
            out.text.push_str(&s[i..start]);
            out.text.push_str(&format!("[已打码：{kind}]"));
            *out.counts.entry(kind).or_default() += 1;
            i = end;
        } else {
            let c = s[i..].chars().next().unwrap();
            out.text.push(c);
            i += c.len_utf8();
        }
    }
    out
}
impl Redacted {
    pub fn summary(&self) -> String {
        format!(
            "共打码 {} 处：{}",
            self.counts.values().sum::<usize>(),
            if self.counts.is_empty() {
                "无".into()
            } else {
                self.counts
                    .iter()
                    .map(|(k, v)| format!("{k} {v}"))
                    .collect::<Vec<_>>()
                    .join("、")
            }
        )
    }
}

#[cfg(test)]
#[path = "redact_tests.rs"]
mod tests;
