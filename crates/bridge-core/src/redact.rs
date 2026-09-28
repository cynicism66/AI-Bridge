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
#[path = "redact_assignment.rs"]
mod assignment;
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
    scan_text(s, true)
}
fn scan_text(s: &str, assignments: bool) -> Redacted {
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
        if found.is_none() && assignments {
            if let Some(value) = assignment::parse(s, i) {
                out.text.push_str(&s[i..value.start]);
                if value.sensitive && value.end > value.start {
                    out.text.push_str("[已打码：赋值凭据]");
                    *out.counts.entry("赋值凭据").or_default() += 1;
                } else {
                    // 普通变量的值仍检查密钥等规则，但不把值内文字当成变量名。
                    let nested = scan_text(&s[value.start..value.end], false);
                    out.text.push_str(&nested.text);
                    for (kind, count) in nested.counts {
                        *out.counts.entry(kind).or_default() += count;
                    }
                }
                out.text.push_str(&s[value.end..value.next]);
                i = value.next;
                continue;
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
