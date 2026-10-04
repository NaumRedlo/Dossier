#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Beatmap { id: u64, mode: Option<String> },
    Set { id: u64, beatmap: Option<u64>, mode: Option<String> },
    Hash(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    NotALink,
    Mode(String),
}

fn digits(text: &str) -> Option<u64> {
    let head: String = text.chars().take_while(char::is_ascii_digit).collect();
    if head.is_empty() {
        None
    } else {
        head.parse().ok()
    }
}

fn mode_word(code: &str) -> Option<&'static str> {
    match code.to_ascii_lowercase().as_str() {
        "osu" | "0" => Some("osu"),
        "taiko" | "1" => Some("taiko"),
        "fruits" | "catch" | "2" => Some("fruits"),
        "mania" | "3" => Some("mania"),
        _ => None,
    }
}

pub fn mode_name(mode: &str) -> &'static str {
    match mode {
        "taiko" => "osu!taiko",
        "fruits" => "osu!catch",
        "mania" => "osu!mania",
        _ => "osu!standard",
    }
}

pub fn parse(text: &str) -> Result<Target, Refusal> {
    let text = text.trim();
    if text.is_empty() || text.contains(char::is_whitespace) {
        return Err(Refusal::NotALink);
    }
    if text.len() == 32 && text.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Ok(Target::Hash(text.to_ascii_lowercase()));
    }
    let without_scheme = text.strip_prefix("https://").or_else(|| text.strip_prefix("http://")).unwrap_or(text);
    let (host, path) = without_scheme.split_once('/').ok_or(Refusal::NotALink)?;
    let host = host.to_ascii_lowercase();
    let host = host.strip_prefix("www.").unwrap_or(&host);
    if !["osu.ppy.sh", "old.ppy.sh", "osu.direct", "catboy.best"].contains(&host) {
        return Err(Refusal::NotALink);
    }
    let (path, fragment) = match path.split_once('#') {
        Some((path, fragment)) => (path, Some(fragment)),
        None => (path, None),
    };
    let path = path.split('?').next().unwrap_or(path).trim_end_matches('/');
    let parts: Vec<&str> = path.split('/').collect();
    let mode_of_fragment = |fragment: Option<&str>| -> (Option<String>, Option<u64>) {
        let Some(fragment) = fragment else { return (None, None) };
        let (mode, id) = fragment.split_once('/').map_or((fragment, None), |(mode, id)| (mode, digits(id)));
        (mode_word(mode).map(str::to_owned), id)
    };
    let target = match parts.as_slice() {
        ["beatmapsets", set] | ["s", set] => {
            let (mode, beatmap) = mode_of_fragment(fragment);
            match (digits(set), beatmap) {
                (Some(id), beatmap) => Target::Set { id, beatmap, mode },
                (None, _) => return Err(Refusal::NotALink),
            }
        }
        ["beatmaps", id] | ["b", id] => {
            let (mode, _) = mode_of_fragment(fragment);
            Target::Beatmap { id: digits(id).ok_or(Refusal::NotALink)?, mode }
        }
        _ => return Err(Refusal::NotALink),
    };
    let refused = match &target {
        Target::Beatmap { mode: Some(mode), .. } | Target::Set { mode: Some(mode), beatmap: Some(_), .. } if mode != "osu" => Some(mode.clone()),
        _ => None,
    };
    match refused {
        Some(mode) => Err(Refusal::Mode(mode)),
        None => Ok(target),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Difficulty {
    pub id: u64,
    pub hash: String,
    pub version: String,
    pub stars: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Found {
    pub set: u64,
    pub artist: String,
    pub title: String,
    pub difficulties: Vec<Difficulty>,
    pub picked: Option<usize>,
}

impl Found {
    pub fn cover(&self) -> String {
        format!("https://assets.ppy.sh/beatmaps/{}/covers/cover.jpg", self.set)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Why {
    Nowhere,
    Mode(String),
    Silent(String),
}

#[derive(Debug, Clone, PartialEq)]
struct Beat {
    id: u64,
    set: Option<u64>,
    hash: String,
    version: String,
    mode: String,
    stars: f64,
}

fn parse_beat(body: &serde_json::Value) -> Option<Beat> {
    Some(Beat {
        id: body.get("id")?.as_u64()?,
        set: body.get("beatmapset_id").and_then(serde_json::Value::as_u64),
        hash: body.get("checksum")?.as_str()?.to_ascii_lowercase(),
        version: body.get("version").and_then(serde_json::Value::as_str).unwrap_or_default().trim().to_owned(),
        mode: body.get("mode").and_then(serde_json::Value::as_str).unwrap_or("osu").to_owned(),
        stars: body.get("difficulty_rating").and_then(serde_json::Value::as_f64).unwrap_or(0.0),
    })
}

fn parse_set(body: &serde_json::Value) -> Option<(u64, String, String, Vec<Beat>)> {
    let set = body.get("id")?.as_u64()?;
    let text = |key: &str| crate::maps::tidy(body.get(key).and_then(serde_json::Value::as_str).unwrap_or_default());
    let beats = body.get("beatmaps")?.as_array()?.iter().filter_map(parse_beat).collect();
    Some((set, text("artist"), text("title"), beats))
}

fn http(seconds: u64) -> Result<reqwest::blocking::Client, String> {
    crate::net::client(8, seconds, &format!("Dossier/{}", crate::bot::BUILD), None)
}

fn get(bases: &[&str], path: &str) -> Result<Option<serde_json::Value>, String> {
    let client = http(15)?;
    let mut silence = None;
    for base in bases {
        match client.get(format!("{base}{path}")).send() {
            Ok(reply) if reply.status() == reqwest::StatusCode::NOT_FOUND => {}
            Ok(reply) if reply.status().is_success() => match reply.json::<serde_json::Value>() {
                Ok(body) => return Ok(Some(body)),
                Err(why) => silence = Some(why.to_string()),
            },
            Ok(reply) => silence = Some(format!("answered {}", reply.status())),
            Err(why) => silence = Some(why.to_string()),
        }
    }
    match silence {
        Some(why) => Err(why),
        None => Ok(None),
    }
}

pub fn resolve(target: &Target) -> Result<Found, Why> {
    let bases: Vec<&str> = crate::maps::MIRRORS.iter().map(|mirror| mirror.api).collect();
    resolve_with(&bases, target)
}

pub fn resolve_with(bases: &[&str], target: &Target) -> Result<Found, Why> {
    let silent = Why::Silent;
    let (set, wanted) = match target {
        Target::Hash(hash) => {
            let beat = get(bases, &format!("md5/{hash}")).map_err(silent)?.and_then(|body| parse_beat(&body)).ok_or(Why::Nowhere)?;
            (beat.set.ok_or(Why::Nowhere)?, Some(beat))
        }
        Target::Beatmap { id, .. } => {
            let beat = get(bases, &format!("b/{id}")).map_err(silent)?.and_then(|body| parse_beat(&body)).ok_or(Why::Nowhere)?;
            (beat.set.ok_or(Why::Nowhere)?, Some(beat))
        }
        Target::Set { id, beatmap, .. } => (*id, beatmap.map(|beatmap| Beat { id: beatmap, set: Some(*id), hash: String::new(), version: String::new(), mode: "osu".into(), stars: 0.0 })),
    };
    if let Some(beat) = &wanted {
        if beat.mode != "osu" {
            return Err(Why::Mode(beat.mode.clone()));
        }
    }
    let body = get(bases, &format!("s/{set}")).map_err(Why::Silent)?.ok_or(Why::Nowhere)?;
    let (set, artist, title, beats) = parse_set(&body).ok_or(Why::Nowhere)?;
    let others = beats.iter().find(|beat| beat.mode != "osu").map(|beat| beat.mode.clone());
    let mut difficulties: Vec<Difficulty> = beats
        .into_iter()
        .filter(|beat| beat.mode == "osu")
        .map(|beat| Difficulty { id: beat.id, hash: beat.hash, version: beat.version, stars: beat.stars })
        .collect();
    if difficulties.is_empty() {
        return Err(Why::Mode(others.unwrap_or_else(|| "osu".to_owned())));
    }
    difficulties.sort_by(|a, b| a.stars.total_cmp(&b.stars).then_with(|| a.id.cmp(&b.id)));
    let picked = wanted.as_ref().and_then(|beat| difficulties.iter().position(|difficulty| difficulty.id == beat.id));
    if wanted.is_some() && picked.is_none() {
        return Err(Why::Nowhere);
    }
    Ok(Found { set, artist, title, difficulties, picked })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_beatmap_page_with_a_difficulty_names_the_set_and_the_difficulty() {
        assert_eq!(parse("https://osu.ppy.sh/beatmapsets/2370103#osu/5114204"), Ok(Target::Set { id: 2370103, beatmap: Some(5114204), mode: Some("osu".into()) }));
        assert_eq!(parse("  https://osu.ppy.sh/beatmapsets/2370103#osu/5114204?foo=1 "), Ok(Target::Set { id: 2370103, beatmap: Some(5114204), mode: Some("osu".into()) }));
    }

    #[test]
    fn a_set_page_alone_names_only_the_set() {
        assert_eq!(parse("https://osu.ppy.sh/beatmapsets/2370103"), Ok(Target::Set { id: 2370103, beatmap: None, mode: None }));
        assert_eq!(parse("https://osu.ppy.sh/beatmapsets/2370103/"), Ok(Target::Set { id: 2370103, beatmap: None, mode: None }));
        assert_eq!(parse("https://osu.ppy.sh/s/2370103"), Ok(Target::Set { id: 2370103, beatmap: None, mode: None }));
        assert_eq!(parse("https://osu.ppy.sh/beatmapsets/2370103#osu"), Ok(Target::Set { id: 2370103, beatmap: None, mode: Some("osu".into()) }));
    }

    #[test]
    fn the_short_forms_and_the_old_site_name_a_beatmap() {
        assert_eq!(parse("https://osu.ppy.sh/beatmaps/5114204"), Ok(Target::Beatmap { id: 5114204, mode: None }));
        assert_eq!(parse("https://osu.ppy.sh/b/5114204"), Ok(Target::Beatmap { id: 5114204, mode: None }));
        assert_eq!(parse("http://old.ppy.sh/b/5114204?m=0"), Ok(Target::Beatmap { id: 5114204, mode: None }));
        assert_eq!(parse("osu.ppy.sh/b/5114204"), Ok(Target::Beatmap { id: 5114204, mode: None }));
    }

    #[test]
    fn a_bare_md5_is_a_hash_in_lower_case() {
        assert_eq!(parse("6D1A1FFCFF39147FFA907A0683514135"), Ok(Target::Hash("6d1a1ffcff39147ffa907a0683514135".into())));
        assert_eq!(parse("6d1a1ffcff39147ffa907a068351413"), Err(Refusal::NotALink), "thirty-one symbols are not a hash");
        assert_eq!(parse("zd1a1ffcff39147ffa907a0683514135"), Err(Refusal::NotALink));
    }

    #[test]
    fn another_mode_is_refused_with_its_name() {
        assert_eq!(parse("https://osu.ppy.sh/beatmapsets/100#taiko/200"), Err(Refusal::Mode("taiko".into())));
        assert_eq!(parse("https://osu.ppy.sh/beatmapsets/100#mania/200"), Err(Refusal::Mode("mania".into())));
        assert_eq!(parse("https://osu.ppy.sh/beatmapsets/100#fruits/200"), Err(Refusal::Mode("fruits".into())));
        assert_eq!(parse("https://osu.ppy.sh/beatmapsets/100#taiko"), Ok(Target::Set { id: 100, beatmap: None, mode: Some("taiko".into()) }), "a set page may still hold standard difficulties");
        assert_eq!(mode_name("taiko"), "osu!taiko");
        assert_eq!(mode_name("osu"), "osu!standard");
    }

    #[test]
    fn anything_else_is_not_a_link() {
        for text in ["", "   ", "hello", "https://example.com/not-a-map", "https://osu.ppy.sh/users/2", "https://osu.ppy.sh/beatmapsets/abc", "two words", "https://osu.ppy.sh/b/"] {
            assert_eq!(parse(text), Err(Refusal::NotALink), "{text:?}");
        }
    }

    fn serve(routes: Vec<(&'static str, &'static str)>) -> (String, std::thread::JoinHandle<()>) {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}/api/v2/", listener.local_addr().unwrap());
        let handle = std::thread::spawn(move || {
            listener.set_nonblocking(true).unwrap();
            let started = std::time::Instant::now();
            while started.elapsed() < std::time::Duration::from_secs(3) {
                let Ok((mut stream, _)) = listener.accept() else {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                    continue;
                };
                stream.set_nonblocking(false).unwrap();
                let mut buffer = [0u8; 2048];
                let read = stream.read(&mut buffer).unwrap_or(0);
                let request = String::from_utf8_lossy(&buffer[..read]).into_owned();
                let path = request.split_whitespace().nth(1).unwrap_or("").to_owned();
                let reply = match routes.iter().find(|(route, _)| *route == path) {
                    Some((_, body)) => format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()),
                    None => "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_owned(),
                };
                let _ = stream.write_all(reply.as_bytes());
            }
        });
        (base, handle)
    }

    const BEAT: &str = r#"{"id":5114204,"beatmapset_id":2370103,"checksum":"6D1A1FFCFF39147FFA907A0683514135","version":"From Hell I Rose","mode":"osu","difficulty_rating":6.19467}"#;
    const SET: &str = r#"{"id":2370103,"artist":"Batushka","title":"Pesn' 8","beatmaps":[{"id":5145185,"version":"From Heaven I Fell","mode":"osu","checksum":"2840ee0a5a7de95b74a40fa282a0c9fe","difficulty_rating":6.51449},{"id":5114204,"version":"From Hell I Rose","mode":"osu","checksum":"6d1a1ffcff39147ffa907a0683514135","difficulty_rating":6.19467},{"id":9,"version":"Taiko","mode":"taiko","checksum":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","difficulty_rating":3.0}]}"#;

    #[test]
    fn a_beatmap_link_finds_its_set_and_points_at_its_difficulty() {
        let (base, server) = serve(vec![("/api/v2/b/5114204", BEAT), ("/api/v2/s/2370103", SET)]);
        let found = resolve_with(&[base.as_str()], &Target::Beatmap { id: 5114204, mode: None }).unwrap();
        assert_eq!((found.set, found.artist.as_str(), found.title.as_str()), (2370103, "Batushka", "Pesn' 8"));
        assert_eq!(found.difficulties.len(), 2, "another mode's difficulty is left out");
        assert_eq!(found.difficulties[0].version, "From Hell I Rose", "easiest first");
        assert_eq!(found.picked, Some(0));
        assert_eq!(found.difficulties[0].hash, "6d1a1ffcff39147ffa907a0683514135");
        assert_eq!(found.cover(), "https://assets.ppy.sh/beatmaps/2370103/covers/cover.jpg");
        server.join().unwrap();
    }

    #[test]
    fn a_set_link_offers_every_standard_difficulty_and_picks_none() {
        let (base, server) = serve(vec![("/api/v2/s/2370103", SET)]);
        let found = resolve_with(&[base.as_str()], &Target::Set { id: 2370103, beatmap: None, mode: None }).unwrap();
        assert_eq!(found.difficulties.iter().map(|d| d.version.as_str()).collect::<Vec<_>>(), ["From Hell I Rose", "From Heaven I Fell"]);
        assert_eq!(found.picked, None);
        server.join().unwrap();
    }

    #[test]
    fn a_hash_is_resolved_through_the_md5_route() {
        let (base, server) = serve(vec![("/api/v2/md5/6d1a1ffcff39147ffa907a0683514135", BEAT), ("/api/v2/s/2370103", SET)]);
        let found = resolve_with(&[base.as_str()], &Target::Hash("6d1a1ffcff39147ffa907a0683514135".into())).unwrap();
        assert_eq!(found.picked, Some(0));
        server.join().unwrap();
    }

    #[test]
    fn a_map_no_mirror_knows_is_nowhere_and_the_next_mirror_is_tried() {
        let (empty, first) = serve(vec![]);
        let (full, second) = serve(vec![("/api/v2/s/2370103", SET)]);
        let found = resolve_with(&[empty.as_str(), full.as_str()], &Target::Set { id: 2370103, beatmap: None, mode: None });
        assert!(found.is_ok(), "the second mirror answers when the first does not know the set");
        let (nothing, third) = serve(vec![]);
        assert_eq!(resolve_with(&[nothing.as_str()], &Target::Set { id: 1, beatmap: None, mode: None }), Err(Why::Nowhere));
        first.join().unwrap();
        second.join().unwrap();
        third.join().unwrap();
    }

    #[test]
    fn a_mirror_that_does_not_answer_is_silent_not_nowhere() {
        let found = resolve_with(&["http://127.0.0.1:9/api/v2/"], &Target::Set { id: 1, beatmap: None, mode: None });
        assert!(matches!(found, Err(Why::Silent(_))), "{found:?}");
    }

    #[test]
    fn a_beatmap_of_another_mode_is_refused_by_its_mode() {
        let taiko = r#"{"id":9,"beatmapset_id":2370103,"checksum":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","version":"Taiko","mode":"taiko","difficulty_rating":3.0}"#;
        let (base, server) = serve(vec![("/api/v2/b/9", taiko)]);
        assert_eq!(resolve_with(&[base.as_str()], &Target::Beatmap { id: 9, mode: None }), Err(Why::Mode("taiko".into())));
        server.join().unwrap();
    }
}
