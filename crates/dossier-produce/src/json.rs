//! Just enough JSON to say what happened.
//!
//! Both the machine-readable render events and the command line's own
//! reports write objects by hand rather than through a serialiser: the shapes
//! are fixed, small, and read more clearly written out than assembled. What
//! they share is the escaping, which is here so that the two cannot disagree
//! about it.

/// Minimal JSON string escaping — enough for filenames, titles and player
/// names, which is all this program emits.
pub fn quote(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
