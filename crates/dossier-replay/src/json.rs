use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Value>),
    Object(BTreeMap<String, Value>),
}

impl Value {
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Self::Object(map) => map.get(key),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Self::Array(items) => Some(items),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&BTreeMap<String, Value>> {
        match self {
            Self::Object(map) => Some(map),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Number(n) => Some(*n as i64),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }
}

pub fn parse(text: &str) -> Option<Value> {
    let bytes = text.as_bytes();
    let mut at = 0usize;
    let value = value(bytes, &mut at, 0)?;
    skip_space(bytes, &mut at);

    (at == bytes.len()).then_some(value)
}

const MAX_DEPTH: usize = 32;

fn value(b: &[u8], at: &mut usize, depth: usize) -> Option<Value> {
    if depth > MAX_DEPTH {
        return None;
    }
    skip_space(b, at);
    match *b.get(*at)? {
        b'{' => object(b, at, depth),
        b'[' => array(b, at, depth),
        b'"' => string(b, at).map(Value::String),
        b't' => literal(b, at, "true", Value::Bool(true)),
        b'f' => literal(b, at, "false", Value::Bool(false)),
        b'n' => literal(b, at, "null", Value::Null),
        _ => number(b, at),
    }
}

fn object(b: &[u8], at: &mut usize, depth: usize) -> Option<Value> {
    *at += 1;
    let mut map = BTreeMap::new();
    skip_space(b, at);
    if *b.get(*at)? == b'}' {
        *at += 1;
        return Some(Value::Object(map));
    }
    loop {
        skip_space(b, at);
        let key = string(b, at)?;
        skip_space(b, at);
        if *b.get(*at)? != b':' {
            return None;
        }
        *at += 1;
        map.insert(key, value(b, at, depth + 1)?);
        skip_space(b, at);
        match *b.get(*at)? {
            b',' => *at += 1,
            b'}' => {
                *at += 1;
                return Some(Value::Object(map));
            }
            _ => return None,
        }
    }
}

fn array(b: &[u8], at: &mut usize, depth: usize) -> Option<Value> {
    *at += 1;
    let mut items = Vec::new();
    skip_space(b, at);
    if *b.get(*at)? == b']' {
        *at += 1;
        return Some(Value::Array(items));
    }
    loop {
        items.push(value(b, at, depth + 1)?);
        skip_space(b, at);
        match *b.get(*at)? {
            b',' => *at += 1,
            b']' => {
                *at += 1;
                return Some(Value::Array(items));
            }
            _ => return None,
        }
    }
}

fn string(b: &[u8], at: &mut usize) -> Option<String> {
    if *b.get(*at)? != b'"' {
        return None;
    }
    *at += 1;
    let mut out = String::new();
    loop {
        let c = *b.get(*at)?;
        *at += 1;
        match c {
            b'"' => return Some(out),
            b'\\' => {
                let esc = *b.get(*at)?;
                *at += 1;
                match esc {
                    b'"' => out.push('"'),
                    b'\\' => out.push('\\'),
                    b'/' => out.push('/'),
                    b'b' => out.push('\u{8}'),
                    b'f' => out.push('\u{c}'),
                    b'n' => out.push('\n'),
                    b'r' => out.push('\r'),
                    b't' => out.push('\t'),
                    b'u' => {
                        let code = hex4(b, at)?;

                        out.push(char::from_u32(u32::from(code)).unwrap_or('\u{fffd}'));
                    }
                    _ => return None,
                }
            }

            _ => {
                let start = *at - 1;
                let len = utf8_len(c)?;
                let end = start + len;
                out.push_str(std::str::from_utf8(b.get(start..end)?).ok()?);
                *at = end;
            }
        }
    }
}

fn utf8_len(first: u8) -> Option<usize> {
    match first {
        0x00..=0x7f => Some(1),
        0xc2..=0xdf => Some(2),
        0xe0..=0xef => Some(3),
        0xf0..=0xf4 => Some(4),
        _ => None,
    }
}

fn hex4(b: &[u8], at: &mut usize) -> Option<u16> {
    let mut value = 0u16;
    for _ in 0..4 {
        let c = *b.get(*at)?;
        *at += 1;
        let digit = (c as char).to_digit(16)?;
        value = value.checked_mul(16)?.checked_add(digit as u16)?;
    }
    Some(value)
}

fn number(b: &[u8], at: &mut usize) -> Option<Value> {
    let start = *at;
    if matches!(b.get(*at), Some(b'-' | b'+')) {
        *at += 1;
    }
    while matches!(b.get(*at), Some(c) if c.is_ascii_digit() || matches!(c, b'.' | b'e' | b'E' | b'-' | b'+'))
    {
        *at += 1;
    }
    if *at == start {
        return None;
    }
    std::str::from_utf8(&b[start..*at])
        .ok()?
        .parse::<f64>()
        .ok()
        .map(Value::Number)
}

fn literal(b: &[u8], at: &mut usize, word: &str, value: Value) -> Option<Value> {
    if b.get(*at..*at + word.len())? == word.as_bytes() {
        *at += word.len();
        Some(value)
    } else {
        None
    }
}

fn skip_space(b: &[u8], at: &mut usize) {
    while matches!(b.get(*at), Some(b' ' | b'\t' | b'\n' | b'\r')) {
        *at += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_reads_the_shape_lazer_actually_writes() {
        let text = r#"{
  "client_version": "2026.417.0-tachyon-linux",
  "rank": "A",
  "user_id": 17397924,
  "mods": [ { "acronym": "CL", "settings": { "no_slider_head_accuracy": false } },
            { "acronym": "HD" } ],
  "statistics": { "great": 1003, "miss": 2 }
}"#;
        let v = parse(text).expect("parses");
        assert_eq!(
            v.get("client_version").and_then(Value::as_str),
            Some("2026.417.0-tachyon-linux")
        );
        assert_eq!(v.get("user_id").and_then(Value::as_i64), Some(17_397_924));

        let mods = v.get("mods").and_then(Value::as_array).expect("an array");
        assert_eq!(mods.len(), 2);
        assert_eq!(mods[0].get("acronym").and_then(Value::as_str), Some("CL"));
        assert_eq!(
            mods[0]
                .get("settings")
                .and_then(|s| s.get("no_slider_head_accuracy"))
                .and_then(Value::as_bool),
            Some(false)
        );

        assert!(mods[1].get("settings").is_none());

        assert_eq!(
            v.get("statistics")
                .and_then(|s| s.get("great"))
                .and_then(Value::as_i64),
            Some(1003)
        );
    }

    #[test]
    fn rubbish_is_rejected_rather_than_half_read() {
        for text in [
            "",
            "{",
            "{\"a\"",
            "{\"a\":}",
            "{\"a\":1,}",
            "[1,2",
            "\"unterminated",
            "{\"a\":1} trailing",
            "{\"a\":\"\\q\"}",
            "{\"a\":\"\\u00\"}",
            "nul",
        ] {
            assert_eq!(parse(text), None, "{text:?} should not parse");
        }
    }

    #[test]
    fn nesting_is_bounded() {
        let deep = "[".repeat(10_000) + &"]".repeat(10_000);
        assert_eq!(parse(&deep), None);
    }

    #[test]
    fn escapes_and_numbers_come_back_intact() {
        let v = parse(r#"{"s":"a\"b\\c\nd\u0041","n":-1.5e3,"t":true,"z":null}"#).expect("parses");
        assert_eq!(v.get("s").and_then(Value::as_str), Some("a\"b\\c\ndA"));
        assert_eq!(v.get("n").and_then(Value::as_i64), Some(-1500));
        assert_eq!(v.get("t").and_then(Value::as_bool), Some(true));
        assert_eq!(v.get("z"), Some(&Value::Null));
    }
}
