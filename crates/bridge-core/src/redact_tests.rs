use super::*;
#[test]
fn keys_and_non_keys() {
    for prefix in [
        "sk-",
        "sk-ant-",
        "ghp_",
        "gho_",
        "github_pat_",
        "AKIA",
        "xoxb-",
        "xoxa-",
        "xoxp-",
        "xoxr-",
        "xoxs-",
        "AIza",
    ] {
        assert_eq!(scan(prefix).text, prefix);
        let input = format!("中文 {prefix}aB123456 end");
        let result = scan(&input);
        assert_eq!(result.text, "中文 [已打码：密钥] end", "{prefix}");
        assert_eq!(result.counts["密钥"], 1);
    }
    for input in [
        "token is a word",
        "sk-",
        "ghp_",
        "xoxz-123456",
        "desk-1234",
        "github_path",
        "AKI",
    ] {
        assert_eq!(scan(input).text, input);
        assert!(scan(input).counts.is_empty());
    }
}
#[test]
fn private_key_blocks_and_public_keys() {
    for label in [
        "PRIVATE KEY",
        "RSA PRIVATE KEY",
        "EC PRIVATE KEY",
        "OPENSSH PRIVATE KEY",
        "ENCRYPTED PRIVATE KEY",
    ] {
        for newline in ["\n", "\r\n"] {
            let source = format!(
                "before -----BEGIN {label}-----{newline}abc123{newline}-----END {label}----- after"
            );
            let result = scan(&source);
            assert_eq!(result.text, "before [已打码：私钥] after");
            assert_eq!(result.counts["私钥"], 1);
        }
    }
    assert_eq!(
        scan("-----BEGIN PRIVATE KEY-----\ntruncated").text,
        "[已打码：私钥]"
    );
    let public = "-----BEGIN PUBLIC KEY-----\naaa\n-----END PUBLIC KEY-----";
    assert_eq!(scan(public).text, public);
}
#[test]
fn assignments_quotes_case_unicode_and_empty_values() {
    for key in [
        "password", "passwd", "pwd", "secret", "token", "api_key", "apikey",
    ] {
        for sep in ["=", " : "] {
            let source = format!("{}{}\"秘密 a\\\"b\"; end", key.to_uppercase(), sep);
            let result = scan(&source);
            assert_eq!(
                result.text,
                format!("{}{sep}\"[已打码：赋值凭据]\"; end", key.to_uppercase())
            );
            assert_eq!(result.counts["赋值凭据"], 1);
        }
    }
    assert_eq!(
        scan(r#"{"pwd":"中文","token":'abc',"api_key":xyz }"#).text,
        r#"{"pwd":"[已打码：赋值凭据]","token":'[已打码：赋值凭据]',"api_key":[已打码：赋值凭据] }"#
    );
    assert_eq!(
        scan("secret='first\nsecond'").text,
        "secret='[已打码：赋值凭据]'"
    );
    for source in [
        "password",
        "token count",
        "secret=",
        "pwd=''",
        "api_key: \n",
    ] {
        assert_eq!(scan(source).text, source);
    }
}
#[test]
fn bearer_urls_and_idempotence() {
    let result = scan("Bearer abc.def-123 bEaReR aB+/= https://user:pass@example.com/a http://姓名:密码@localhost/");
    assert_eq!(result.text, "Bearer [已打码：Bearer] bEaReR [已打码：Bearer] https://[已打码：URL凭据]@example.com/a http://[已打码：URL凭据]@localhost/");
    assert_eq!(result.counts["Bearer"], 2);
    assert_eq!(result.counts["URL凭据"], 2);
    assert!(result.summary().starts_with("共打码 4 处："));
    let second = scan(&result.text);
    assert_eq!(second.text, result.text);
    assert!(second.counts.is_empty());
    for source in [
        "Bearer ",
        "Bearers abc",
        "https://example.com:8080/a",
        "git@example.com:path",
        "https://user@example.com/a",
    ] {
        assert_eq!(scan(source).text, source);
    }
    let first = scan("password='abc' token=def ghp_abcdefgh");
    assert_eq!(scan(&first.text).text, first.text);
    assert!(scan(&first.text).counts.is_empty());
}

#[test]
fn prefixed_and_suffixed_assignment_samples() {
    for (source, expected) in [
        ("GITHUB_TOKEN=one", "GITHUB_TOKEN=[已打码：赋值凭据]"),
        ("OPENAI_API_KEY=two", "OPENAI_API_KEY=[已打码：赋值凭据]"),
        (
            "AWS_SECRET_ACCESS_KEY=three",
            "AWS_SECRET_ACCESS_KEY=[已打码：赋值凭据]",
        ),
        ("DB_PASSWORD=four", "DB_PASSWORD=[已打码：赋值凭据]"),
        ("client_secret: five", "client_secret: [已打码：赋值凭据]"),
        (
            r#""access_token": "six""#,
            r#""access_token": "[已打码：赋值凭据]""#,
        ),
        (
            "export ANTHROPIC_API_KEY=seven",
            "export ANTHROPIC_API_KEY=[已打码：赋值凭据]",
        ),
        (
            r#"$env:HF_TOKEN = "eight""#,
            r#"$env:HF_TOKEN = "[已打码：赋值凭据]""#,
        ),
        (
            r#"DB_PASSWORD="a b c""#,
            r#"DB_PASSWORD="[已打码：赋值凭据]""#,
        ),
        ("apikeys=normal", "apikeys=[已打码：赋值凭据]"),
        ("passwordless=true", "passwordless=[已打码：赋值凭据]"),
        ("an_api_key=ordinary", "an_api_key=[已打码：赋值凭据]"),
    ] {
        let result = scan(source);
        assert_eq!(result.text, expected, "{source}");
        assert_eq!(result.counts["赋值凭据"], 1);
        assert_eq!(scan(expected).text, expected);
        assert!(scan(expected).counts.is_empty());
    }
}
#[test]
fn all_assignment_keywords_and_value_boundaries() {
    for keyword in [
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
        "auth",
    ] {
        for value in ["a;b<>`xyz", "a}b]c", "'a b c'", r#""a\"b""#] {
            let source = format!("prefix.{}_2-suffix = {value}, tail", keyword.to_uppercase());
            let result = scan(&source);
            assert!(!result.text.contains(value), "{source}");
            assert!(result.text.ends_with(", tail"));
            assert_eq!(result.counts["赋值凭据"], 1);
        }
    }
    assert_eq!(
        scan("db.password=abc\tnext\nclient.secret=xyz\r\n").text,
        "db.password=[已打码：赋值凭据]\tnext\nclient.secret=[已打码：赋值凭据]\r\n"
    );
    assert_eq!(
        scan("Authorization: Bearer super-value").text,
        "Authorization: Bearer [已打码：Bearer]"
    );
}
#[test]
fn ordinary_sentences_and_keywords_only_in_values_stay_readable() {
    for source in [
        "这个 token 很重要",
        "password 规则要写清楚",
        "DB_PASSWORD 需要保护",
        "export GITHUB_TOKEN",
        "label=token",
        "description: password",
        r#""label": "client_secret: not-sensitive""#,
        "value='DB_PASSWORD=example'",
        "value=auth=example",
        "$env:LABEL = 'HF_TOKEN = example'",
        "key = 'api_key credential access_key private_key password'",
    ] {
        let result = scan(source);
        assert_eq!(result.text, source);
        assert!(result.counts.is_empty());
    }
    // 普通变量的值内真实密钥和 URL 凭据仍按各自规则打码。
    assert_eq!(scan("label=ghp_abcdefgh").text, "label=[已打码：密钥]");
    assert_eq!(
        scan("url=https://user:pass@example.com").text,
        "url=https://[已打码：URL凭据]@example.com"
    );
    assert_eq!(
        scan(r#"{"config":{"client_secret":"nested"}}"#).text,
        r#"{"config":{"client_secret":"[已打码：赋值凭据]"}}"#
    );
}

#[test]
fn assignment_values_still_mask_entire_private_key_blocks() {
    let pem = "-----BEGIN RSA PRIVATE KEY-----\nprivate-material\n-----END RSA PRIVATE KEY-----";
    for key in ["material", "PRIVATE_KEY"] {
        let result = scan(&format!("{key}={pem}\nnext line"));
        assert!(!result.text.contains("private-material"));
        assert!(result.text.ends_with("\nnext line"));
        assert_eq!(result.counts.values().sum::<usize>(), 1);
    }
}

#[test]
fn auth_matches_whole_identifier_segments() {
    for key in [
        "author",
        "authority",
        "AUTHORITY",
        "authorization",
        "myAuthor",
    ] {
        let source = format!("{key}=ordinary");
        assert_eq!(scan(&source).text, source);
    }
    for key in [
        "AUTH",
        "auth_token",
        "x-auth",
        "authToken",
        "myAuthValue",
        "X.AUTH",
    ] {
        let output = scan(&format!("{key}: sample-secret")).text;
        assert!(!output.contains("sample-secret"), "{key}");
    }
}
