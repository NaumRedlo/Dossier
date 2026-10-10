use crate::beatmaps::Facts;
use crate::client::Client;
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

pub fn attached(pid: u32, client: &Client) -> String {
    format!("{{\"event\":\"attached\",\"pid\":{pid},\"build\":{},\"player\":{}}}", quoted(&client.build), quoted(&client.player))
}

fn number(value: f32) -> String {
    if value.is_finite() {
        format!("{value}")
    } else {
        "0".to_owned()
    }
}

pub fn facts_of(facts: &Facts) -> String {
    format!(
        "{{\"id\":{},\"set\":{},\"status\":{},\"stars\":{},\"base_stars\":{},\"ar\":{},\"cs\":{},\"od\":{},\"hp\":{},\"bpm\":{},\"length\":{},\"objects\":{}}}",
        facts.id,
        facts.set,
        quoted(facts.status),
        number(facts.stars),
        number(facts.base_stars),
        number(facts.ar),
        number(facts.cs),
        number(facts.od),
        number(facts.hp),
        number(facts.bpm),
        facts.length,
        facts.objects
    )
}

fn facts_part(facts: Option<&Facts>) -> String {
    facts.map_or_else(String::new, |facts| format!(",\"facts\":{}", facts_of(facts)))
}

pub fn state(seen: &Glance, facts: Option<&Facts>) -> String {
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
    format!("{{\"event\":\"state\",\"mode\":{}{map}{}}}", quoted(&mode), facts_part(facts))
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

pub fn kept(take: &Take, name: &str, osr: &[u8], facts: Option<&Facts>) -> String {
    format!(
        "{{\"event\":\"kept\",\"name\":{},\"passed\":{},\"failed\":{},\"md5\":{},\"id\":{},\"set\":{},\"artist\":{},\"title\":{},\"version\":{},\"creator\":{},\"score\":{},\"frames\":{},\"watched\":{}{},\"osr\":\"{}\"}}",
        quoted(name),
        take.passed,
        take.failed && !take.passed,
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
        facts_part(facts),
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
        assert_eq!(state(&seen, None), "{\"event\":\"state\",\"mode\":\"SelectPlay\",\"md5\":\"0123456789abcdef0123456789abcdef\",\"id\":7,\"set\":8,\"artist\":\"xi\",\"title\":\"FREEDOM \\\"DiVE\\\"\",\"version\":\"FOUR\",\"creator\":\"N\"}");
        let bare = Glance { raw_mode: 77, mode: None, time_ms: 0, map: None, play: None, watching: None };
        assert_eq!(state(&bare, None), "{\"event\":\"state\",\"mode\":\"Other77\"}");
        assert_eq!(plain("waiting"), "{\"event\":\"waiting\"}");
        assert_eq!(attached(436, &Client::default()), "{\"event\":\"attached\",\"pid\":436,\"build\":\"\",\"player\":\"\"}");
        assert_eq!(
            attached(436, &Client { build: "b20260924cuttingedge".into(), player: "Naum \"N\" Redlo".into(), ..Client::default() }),
            "{\"event\":\"attached\",\"pid\":436,\"build\":\"b20260924cuttingedge\",\"player\":\"Naum \\\"N\\\" Redlo\"}"
        );
    }

    #[test]
    fn a_kept_play_names_its_map_and_says_whether_it_was_watched() {
        let map = Map { md5: "0123456789abcdef0123456789abcdef".into(), id: 7, set: 8, artist: "xi".into(), title: "t".into(), version: "v".into(), creator: "N".into(), ..Map::default() };
        let mut take = Take { score: 0, map, play: Play { score: 5, ..Play::default() }, frames: Vec::new(), life: Vec::new(), passed: true, failed: false, watched: Some(false) };
        assert_eq!(
            kept(&take, "a.osr", &[0, 0xff], None),
            "{\"event\":\"kept\",\"name\":\"a.osr\",\"passed\":true,\"failed\":false,\"md5\":\"0123456789abcdef0123456789abcdef\",\"id\":7,\"set\":8,\"artist\":\"xi\",\"title\":\"t\",\"version\":\"v\",\"creator\":\"N\",\"score\":5,\"frames\":0,\"watched\":false,\"osr\":\"00ff\"}"
        );
        take.watched = None;
        assert!(kept(&take, "a.osr", &[], None).contains("\"watched\":null"));
        let facts = Facts { id: 7, set: 8, status: "ranked", stars: 6.25, base_stars: 4.5, ar: 9.0, cs: 4.0, od: 8.5, hp: 5.0, bpm: 180.0, length: 120, objects: 300 };
        let said = kept(&take, "a.osr", &[], Some(&facts));
        assert!(said.contains("\"watched\":null,\"facts\":{\"id\":7,\"set\":8,\"status\":\"ranked\",\"stars\":6.25,\"base_stars\":4.5,\"ar\":9,\"cs\":4,\"od\":8.5,\"hp\":5,\"bpm\":180,\"length\":120,\"objects\":300},\"osr\":\"\""), "{said}");
        let seen = Glance { raw_mode: 5, mode: Some(Mode::SelectPlay), time_ms: 1, map: Some(take.map.clone()), play: None, watching: None };
        assert!(state(&seen, Some(&facts)).ends_with("\"creator\":\"N\",\"facts\":{\"id\":7,\"set\":8,\"status\":\"ranked\",\"stars\":6.25,\"base_stars\":4.5,\"ar\":9,\"cs\":4,\"od\":8.5,\"hp\":5,\"bpm\":180,\"length\":120,\"objects\":300}}"));
        let odd = Facts { stars: f32::NAN, bpm: f32::INFINITY, ..facts };
        assert!(facts_of(&odd).contains("\"stars\":0,") && facts_of(&odd).contains("\"bpm\":0,"), "a number that is no number is told as nothing");
    }
}
