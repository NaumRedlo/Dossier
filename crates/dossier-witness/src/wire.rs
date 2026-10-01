use crate::stable::{Glance, Take};

pub fn quoted(said: &str) -> String {
    let mut out = String::with_capacity(said.len() + 2);
    out.push('"');
    for c in said.chars() {
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

pub fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(DIGITS[(byte >> 4) as usize] as char);
        out.push(DIGITS[(byte & 0xF) as usize] as char);
    }
    out
}

pub fn plain(event: &str) -> String {
    format!("{{\"event\":{}}}", quoted(event))
}

pub fn attached(pid: u32) -> String {
    format!("{{\"event\":\"attached\",\"pid\":{pid}}}")
}

pub fn state(seen: &Glance) -> String {
    let mode = seen.mode.map_or_else(|| format!("Other{}", seen.raw_mode), |mode| format!("{mode:?}"));
    let map = match &seen.map {
        Some(map) => format!(
            ",\"md5\":{},\"id\":{},\"set\":{},\"artist\":{},\"title\":{},\"version\":{},\"creator\":{}",
            quoted(&map.md5),
            map.id,
            map.set,
            quoted(&map.artist),
            quoted(&map.title),
            quoted(&map.version),
            quoted(&map.creator)
        ),
        None => String::new(),
    };
    format!("{{\"event\":\"state\",\"mode\":{}{map}}}", quoted(&mode))
}

pub fn playing(take: &Take, time_ms: i32) -> String {
    let counts = take.play.counts;
    format!(
        "{{\"event\":\"playing\",\"md5\":{},\"time\":{time_ms},\"frames\":{},\"score\":{},\"combo\":{},\"max_combo\":{},\"n300\":{},\"n100\":{},\"n50\":{},\"miss\":{},\"mods\":{}}}",
        quoted(&take.map.md5),
        take.frames.len(),
        take.play.score,
        take.play.combo,
        take.play.max_combo,
        counts.n300,
        counts.n100,
        counts.n50,
        counts.miss,
        take.play.mods
    )
}

pub fn kept(take: &Take, name: &str, osr: &[u8]) -> String {
    format!(
        "{{\"event\":\"kept\",\"name\":{},\"passed\":{},\"md5\":{},\"id\":{},\"set\":{},\"artist\":{},\"title\":{},\"version\":{},\"creator\":{},\"score\":{},\"frames\":{},\"watched\":{},\"osr\":\"{}\"}}",
        quoted(name),
        take.passed,
        quoted(&take.map.md5),
        take.map.id,
        take.map.set,
        quoted(&take.map.artist),
        quoted(&take.map.title),
        quoted(&take.map.version),
        quoted(&take.map.creator),
        take.play.score,
        take.frames.len(),
        take.watched.map_or_else(|| "null".to_owned(), |watched| watched.to_string()),
        hex(osr)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stable::{Map, Mode, Play};

    #[test]
    fn words_are_quoted_the_way_json_wants_them() {
        assert_eq!(quoted("plain"), "\"plain\"");
        assert_eq!(quoted("a \"b\" \\ c\n"), "\"a \\\"b\\\" \\\\ c\\n\"");
        assert_eq!(quoted("\u{1}"), "\"\\u0001\"");
        assert_eq!(quoted("песня 譜面"), "\"песня 譜面\"");
        assert_eq!(hex(&[0, 0x0f, 0xa5, 0xff]), "000fa5ff");
    }

    #[test]
    fn a_state_names_the_screen_and_the_map_when_there_is_one() {
        let map = Map { md5: "0123456789abcdef0123456789abcdef".into(), id: 7, set: 8, artist: "xi".into(), title: "FREEDOM \"DiVE\"".into(), version: "FOUR".into(), creator: "N".into(), ..Map::default() };
        let seen = Glance { raw_mode: 5, mode: Some(Mode::SelectPlay), time_ms: 1, map: Some(map), play: None, watching: None };
        assert_eq!(state(&seen), "{\"event\":\"state\",\"mode\":\"SelectPlay\",\"md5\":\"0123456789abcdef0123456789abcdef\",\"id\":7,\"set\":8,\"artist\":\"xi\",\"title\":\"FREEDOM \\\"DiVE\\\"\",\"version\":\"FOUR\",\"creator\":\"N\"}");
        let bare = Glance { raw_mode: 77, mode: None, time_ms: 0, map: None, play: None, watching: None };
        assert_eq!(state(&bare), "{\"event\":\"state\",\"mode\":\"Other77\"}");
        assert_eq!(plain("waiting"), "{\"event\":\"waiting\"}");
        assert_eq!(attached(436), "{\"event\":\"attached\",\"pid\":436}");
    }

    #[test]
    fn a_kept_play_names_its_map_and_says_whether_it_was_watched() {
        let map = Map { md5: "0123456789abcdef0123456789abcdef".into(), id: 7, set: 8, artist: "xi".into(), title: "t".into(), version: "v".into(), creator: "N".into(), ..Map::default() };
        let mut take = Take { score: 0, map, play: Play { score: 5, ..Play::default() }, frames: Vec::new(), life: Vec::new(), passed: true, watched: Some(false) };
        assert_eq!(
            kept(&take, "a.osr", &[0, 0xff]),
            "{\"event\":\"kept\",\"name\":\"a.osr\",\"passed\":true,\"md5\":\"0123456789abcdef0123456789abcdef\",\"id\":7,\"set\":8,\"artist\":\"xi\",\"title\":\"t\",\"version\":\"v\",\"creator\":\"N\",\"score\":5,\"frames\":0,\"watched\":false,\"osr\":\"00ff\"}"
        );
        take.watched = None;
        assert!(kept(&take, "a.osr", &[]).contains("\"watched\":null"));
    }
}
