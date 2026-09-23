use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};

const AGENT: &str = concat!("Dossier/", env!("CARGO_PKG_VERSION"), " (+https://github.com/NaumRedlo/Dossier)");
const PATIENCE: Duration = Duration::from_secs(15);
pub const STALE_AFTER: i64 = 10 * 60;
pub const REDDIT_STALE_AFTER: i64 = 30 * 60;
pub const UPDATES: &str = "updates";
pub const STORIES: &str = "news";
pub const THREADS: &str = "reddit";
pub const CHANGELOG: &str = "https://osu.ppy.sh/home/changelog";
pub const NEWS: &str = "https://osu.ppy.sh/home/news?format=atom";
pub const REDDIT: &str = "https://www.reddit.com/r/osugame/.rss";

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Change {
    pub category: String,
    pub title: String,
    pub major: bool,
    pub kind: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Build {
    pub stream: String,
    pub version: String,
    pub at: i64,
    pub url: String,
    pub changes: Vec<Change>,
}

fn is_false(said: &bool) -> bool {
    !*said
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Span {
    pub text: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub bold: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub italic: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub code: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
}

impl Span {
    pub fn plain(text: &str) -> Span {
        Span { text: text.to_owned(), ..Span::default() }
    }

    fn same_look(&self, other: &Span) -> bool {
        self.bold == other.bold && self.italic == other.italic && self.code == other.code && self.link == other.link
    }
}

pub fn words_of(spans: &[Span]) -> String {
    spans.iter().map(|span| span.text.as_str()).collect()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Block {
    Heading(Vec<Span>),
    Text(Vec<Span>),
    Item(Vec<Span>),
    Quote(Vec<Span>),
    Image(String),
}

impl Block {
    pub fn words(&self) -> String {
        match self {
            Block::Heading(spans) | Block::Text(spans) | Block::Item(spans) | Block::Quote(spans) => words_of(spans),
            Block::Image(_) => String::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flow {
    Article,
    Post,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Story {
    pub title: String,
    pub url: String,
    pub at: i64,
    pub lead: String,
    pub image: Option<String>,
    #[serde(default)]
    pub body: Vec<Block>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Thread {
    pub title: String,
    pub url: String,
    pub author: String,
    pub at: i64,
    #[serde(default)]
    pub body: Vec<Block>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Video {
    #[serde(default)]
    pub thumb: Option<String>,
    #[serde(default)]
    pub src: Option<String>,
    #[serde(default)]
    pub duration: String,
    #[serde(default)]
    pub link: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Post {
    pub channel: String,
    pub name: String,
    pub text: String,
    pub url: String,
    pub at: i64,
    pub image: Option<String>,
    #[serde(default)]
    pub body: Vec<Block>,
    #[serde(default)]
    pub images: Vec<String>,
    #[serde(default)]
    pub videos: Vec<Video>,
}

impl Post {
    pub fn cover(&self) -> Option<&str> {
        self.image.as_deref().or_else(|| self.videos.iter().find_map(|video| video.thumb.as_deref()))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct News {
    pub builds: Vec<Build>,
    pub stories: Vec<Story>,
    pub threads: Vec<Thread>,
    pub posts: Vec<Post>,
    pub fetched: std::collections::HashMap<String, i64>,
}

pub fn channel_source(channel: &str) -> String {
    format!("channel:{}", channel.to_ascii_lowercase())
}

impl News {
    fn path() -> PathBuf {
        crate::sources::own_root().join("news.json")
    }

    pub fn load() -> News {
        std::fs::read(Self::path()).ok().and_then(|bytes| serde_json::from_slice(&bytes).ok()).unwrap_or_default()
    }

    pub fn save(&self) {
        if let Ok(bytes) = serde_json::to_vec(self) {
            let _ = std::fs::create_dir_all(crate::sources::own_root());
            let _ = std::fs::write(Self::path(), bytes);
        }
    }

    pub fn stale(&self, source: &str, now: i64) -> bool {
        let after = if source == THREADS { REDDIT_STALE_AFTER } else { STALE_AFTER };
        self.fetched.get(source).is_none_or(|at| now - at > after)
    }

    pub fn fetched_at(&self, source: &str) -> Option<i64> {
        self.fetched.get(source).copied()
    }

    pub fn mark(&mut self, source: &str, now: i64) {
        self.fetched.insert(source.to_owned(), now);
    }

    pub fn take_posts(&mut self, channel: &str, mut fresh: Vec<Post>) {
        self.posts.retain(|post| !post.channel.eq_ignore_ascii_case(channel));
        self.posts.append(&mut fresh);
        self.posts.sort_by(|a, b| b.at.cmp(&a.at));
    }
}

fn client() -> Result<reqwest::blocking::Client, String> {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(reqwest::header::ACCEPT, reqwest::header::HeaderValue::from_static("text/html,application/atom+xml,application/xml;q=0.9,*/*;q=0.8"));
    reqwest::blocking::Client::builder().timeout(PATIENCE).user_agent(AGENT).default_headers(headers).build().map_err(|e| e.to_string())
}

pub fn page(url: &str) -> Result<String, String> {
    fetched(url)
}

fn fetched(url: &str) -> Result<String, String> {
    let response = client()?.get(url).send().map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("{} answered {}", url, response.status().as_u16()));
    }
    response.text().map_err(|e| e.to_string())
}

pub fn picture(url: &str) -> Option<Vec<u8>> {
    let response = client().ok()?.get(url).send().ok()?;
    response.status().is_success().then(|| response.bytes().ok().map(|bytes| bytes.to_vec())).flatten()
}

pub fn save_to(url: &str, path: &std::path::Path) -> Result<(), String> {
    let client = reqwest::blocking::Client::builder().timeout(Duration::from_secs(300)).user_agent(AGENT).build().map_err(|e| e.to_string())?;
    let response = client.get(url).send().map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("{} answered {}", url, response.status().as_u16()));
    }
    let bytes = response.bytes().map_err(|e| e.to_string())?;
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder).map_err(|e| e.to_string())?;
    }
    let part = path.with_extension("part");
    std::fs::write(&part, &bytes).map_err(|e| e.to_string())?;
    std::fs::rename(&part, path).map_err(|e| e.to_string())
}

pub fn clip_path(link: &str) -> std::path::PathBuf {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in link.bytes() {
        hash = (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3);
    }
    crate::sources::own_root().join("cache").join("clips").join(format!("{hash:016x}.mp4"))
}

pub fn fetch_builds() -> Result<Vec<Build>, String> {
    builds_from(&fetched(CHANGELOG)?)
}

pub fn fetch_stories() -> Result<Vec<Story>, String> {
    Ok(stories_from(&fetched(NEWS)?))
}

pub fn fetch_threads() -> Result<Vec<Thread>, String> {
    Ok(threads_from(&fetched(REDDIT)?))
}

pub fn fetch_posts(channel: &str) -> Result<Vec<Post>, String> {
    let name = channel_name(channel).ok_or_else(|| format!("{channel} is not a channel name"))?;
    Ok(posts_from(&name, &fetched(&format!("https://t.me/s/{name}"))?))
}

pub fn channel_name(said: &str) -> Option<String> {
    let said = said.trim();
    let said = said.strip_prefix("https://").or_else(|| said.strip_prefix("http://")).unwrap_or(said);
    let said = said.strip_prefix("t.me/s/").or_else(|| said.strip_prefix("t.me/")).unwrap_or(said);
    let name = said.trim_start_matches('@').trim_end_matches('/');
    let fits = (4..=32).contains(&name.len()) && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') && name.chars().next().is_some_and(|c| c.is_ascii_alphabetic());
    fits.then(|| name.to_owned())
}

fn after<'a>(text: &'a str, mark: &str) -> Option<&'a str> {
    text.find(mark).map(|at| &text[at + mark.len()..])
}

fn between<'a>(text: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let rest = after(text, open)?;
    rest.find(close).map(|end| &rest[..end])
}

fn pieces<'a>(text: &'a str, open: &'a str, close: &'a str) -> impl Iterator<Item = &'a str> + 'a {
    let mut rest = text;
    std::iter::from_fn(move || {
        let body = after(rest, open)?;
        let end = body.find(close)?;
        rest = &body[end + close.len()..];
        Some(&body[..end])
    })
}

pub fn unescaped(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        let tail = &rest[at..];
        let Some(end) = tail.find(';').filter(|end| *end <= 10) else {
            out.push('&');
            rest = &tail[1..];
            continue;
        };
        let name = &tail[1..end];
        let made = match name {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            "nbsp" => Some(' '),
            _ => name
                .strip_prefix("#x")
                .and_then(|hex| u32::from_str_radix(hex, 16).ok())
                .or_else(|| name.strip_prefix('#').and_then(|dec| dec.parse().ok()))
                .and_then(char::from_u32),
        };
        match made {
            Some(ch) => {
                out.push(ch);
                rest = &tail[end + 1..];
            }
            None => {
                out.push('&');
                rest = &tail[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

pub fn plain(markup: &str) -> String {
    let spaced = markup.replace("<br/>", "\n").replace("<br>", "\n").replace("<br />", "\n").replace("</p>", "\n");
    let mut out = String::with_capacity(spaced.len());
    let mut inside = false;
    for ch in spaced.chars() {
        match ch {
            '<' => inside = true,
            '>' => inside = false,
            _ if !inside => out.push(ch),
            _ => {}
        }
    }
    let words = unescaped(&out);
    words.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn unix_of(stamp: &str) -> Option<i64> {
    let stamp = stamp.trim();
    let number = |from: usize, to: usize| stamp.get(from..to)?.parse::<i64>().ok();
    let (year, month, day) = (number(0, 4)?, number(5, 7)?, number(8, 10)?);
    let (hour, minute, second) = (number(11, 13)?, number(14, 16)?, number(17, 19)?);
    let zone = stamp[19..].trim_start_matches(|c: char| c == '.' || c.is_ascii_digit());
    let offset = match zone {
        "" | "Z" => 0,
        _ => {
            let sign = if zone.starts_with('-') { -1 } else { 1 };
            let hours: i64 = zone.get(1..3)?.parse().ok()?;
            let minutes: i64 = zone.get(4..6)?.parse().ok()?;
            sign * (hours * 3600 + minutes * 60)
        }
    };
    let (y, m) = if month <= 2 { (year - 1, month + 9) } else { (year, month - 3) };
    let era = y.div_euclid(400);
    let of_era = y - era * 400;
    let of_year = (153 * m + 2) / 5 + day - 1;
    let of_cycle = of_era * 365 + of_era / 4 - of_era / 100 + of_year;
    let days = era * 146_097 + of_cycle - 719_468;
    Some(days * 86_400 + hour * 3600 + minute * 60 + second - offset)
}

fn tag_name(tag: &str) -> (bool, String) {
    let closing = tag.starts_with('/');
    let name: String = tag.trim_start_matches('/').chars().take_while(|c| c.is_ascii_alphanumeric()).collect::<String>().to_ascii_lowercase();
    (closing, name)
}

pub fn resolved(href: &str, base: &str) -> Option<String> {
    let href = unescaped(href.trim());
    if href.is_empty() || href.starts_with('#') || href.to_ascii_lowercase().starts_with("javascript:") {
        return None;
    }
    if href.starts_with("http://") || href.starts_with("https://") || href.starts_with("mailto:") {
        return Some(href);
    }
    if let Some(rest) = href.strip_prefix("//") {
        return Some(format!("https://{rest}"));
    }
    let scheme_end = base.find("://").map_or(0, |at| at + 3);
    let origin = match base[scheme_end..].find('/') {
        Some(slash) => &base[..scheme_end + slash],
        None => base,
    };
    if href.starts_with('/') {
        return Some(format!("{origin}{href}"));
    }
    if href.starts_with('?') {
        let page = base.split(['?', '#']).next().unwrap_or(base);
        return Some(format!("{page}{href}"));
    }
    Some(format!("{origin}/{href}"))
}

fn attribute(tag: &str, name: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    let mut from = 0;
    while let Some(found) = lower[from..].find(name) {
        let at = from + found;
        let before = lower[..at].chars().last();
        let rest = lower[at + name.len()..].trim_start();
        if before.is_some_and(|c| c.is_whitespace()) && rest.starts_with('=') {
            let offset = tag.len() - rest.len() + 1;
            let value = tag[offset..].trim_start();
            let quote = value.chars().next()?;
            if quote == '"' || quote == '\'' {
                return value[1..].find(quote).map(|end| value[1..1 + end].to_owned());
            }
            return Some(value.split(|c: char| c.is_whitespace() || c == '>').next().unwrap_or_default().to_owned());
        }
        from = at + name.len();
    }
    None
}

fn collapsed(words: &str) -> String {
    let mut out = String::with_capacity(words.len());
    let mut space = false;
    for ch in words.chars() {
        if ch.is_whitespace() {
            space = true;
            continue;
        }
        if space {
            out.push(' ');
            space = false;
        }
        out.push(ch);
    }
    if space {
        out.push(' ');
    }
    out
}

fn tidied(spans: Vec<Span>) -> Vec<Span> {
    let mut chars: Vec<(char, usize)> = Vec::new();
    for (at, span) in spans.iter().enumerate() {
        for ch in span.text.chars() {
            match ch {
                ' ' => {
                    if chars.last().is_some_and(|(last, _)| *last != ' ' && *last != '\n') {
                        chars.push((' ', at));
                    }
                }
                '\n' => {
                    while chars.last().is_some_and(|(last, _)| *last == ' ') {
                        chars.pop();
                    }
                    let breaks = chars.iter().rev().take_while(|(last, _)| *last == '\n').count();
                    if !chars.is_empty() && breaks < 2 {
                        chars.push(('\n', at));
                    }
                }
                _ => chars.push((ch, at)),
            }
        }
    }
    while chars.last().is_some_and(|(last, _)| last.is_whitespace()) {
        chars.pop();
    }
    let mut out: Vec<Span> = Vec::new();
    for (ch, at) in chars {
        let like = &spans[at];
        match out.last_mut() {
            Some(last) if last.same_look(like) => last.text.push(ch),
            _ => out.push(Span { text: ch.to_string(), ..like.clone() }),
        }
    }
    out
}

fn paragraphs(spans: Vec<Span>) -> Vec<Vec<Span>> {
    let mut out: Vec<Vec<Span>> = vec![Vec::new()];
    for span in tidied(spans) {
        let mut parts = span.text.split("\n\n");
        if let (Some(first), Some(current)) = (parts.next(), out.last_mut()) {
            current.push(Span { text: first.to_owned(), ..span.clone() });
        }
        for part in parts {
            out.push(vec![Span { text: part.to_owned(), ..span.clone() }]);
        }
    }
    out.into_iter().map(tidied).filter(|spans| !spans.is_empty()).collect()
}

pub fn blocks_of(markup: &str, base: &str, flow: Flow) -> Vec<Block> {
    #[derive(Clone, Copy, PartialEq)]
    enum Kind {
        Text,
        Heading,
        Item,
        Quote,
    }
    #[derive(Clone, Copy, PartialEq)]
    enum Effect {
        Bold,
        Italic,
        Code,
        Link,
        Emoji,
        Nothing,
    }
    let mut out = Vec::new();
    let mut spans: Vec<Span> = Vec::new();
    let mut kind = Kind::Text;
    let mut hidden = 0usize;
    let mut quoted = 0usize;
    let mut open: Vec<(String, Effect)> = Vec::new();
    let mut links: Vec<String> = Vec::new();
    let look = |open: &[(String, Effect)], links: &[String]| Span {
        text: String::new(),
        bold: open.iter().any(|(_, effect)| *effect == Effect::Bold),
        italic: open.iter().any(|(_, effect)| *effect == Effect::Italic),
        code: open.iter().any(|(_, effect)| *effect == Effect::Code),
        link: links.last().cloned(),
    };
    let add = |spans: &mut Vec<Span>, words: String, like: Span| {
        if words.is_empty() {
            return;
        }
        match spans.last_mut() {
            Some(last) if last.same_look(&like) => last.text.push_str(&words),
            _ => spans.push(Span { text: words, ..like }),
        }
    };
    let flush = |out: &mut Vec<Block>, spans: &mut Vec<Span>, kind: Kind| {
        let taken = std::mem::take(spans);
        let pieces = match (flow, kind) {
            (Flow::Post, Kind::Text | Kind::Quote) => paragraphs(taken),
            _ => vec![tidied(taken)],
        };
        for piece in pieces.into_iter().filter(|piece| !piece.is_empty()) {
            out.push(match kind {
                Kind::Heading => Block::Heading(piece),
                Kind::Item => Block::Item(piece),
                Kind::Quote => Block::Quote(piece),
                Kind::Text => Block::Text(piece),
            });
        }
    };
    let mut rest = markup;
    while let Some(at) = rest.find('<') {
        if hidden == 0 {
            add(&mut spans, collapsed(&unescaped(&rest[..at])), look(&open, &links));
        }
        let Some(close) = rest[at..].find('>') else {
            break;
        };
        let tag = &rest[at + 1..at + close];
        rest = &rest[at + close + 1..];
        if tag.starts_with('!') {
            continue;
        }
        let (closing, name) = tag_name(tag);
        match (name.as_str(), closing) {
            ("script" | "style" | "iframe" | "figcaption", false) => hidden += 1,
            ("script" | "style" | "iframe" | "figcaption", true) => hidden = hidden.saturating_sub(1),
            _ if hidden > 0 => {}
            ("h1" | "h2" | "h3" | "h4" | "h5" | "h6", false) => {
                flush(&mut out, &mut spans, kind);
                kind = Kind::Heading;
            }
            ("li", false) => {
                flush(&mut out, &mut spans, kind);
                kind = Kind::Item;
            }
            ("blockquote", false) => {
                flush(&mut out, &mut spans, kind);
                quoted += 1;
                kind = Kind::Quote;
            }
            ("blockquote", true) => {
                flush(&mut out, &mut spans, kind);
                quoted = quoted.saturating_sub(1);
                kind = if quoted > 0 { Kind::Quote } else { Kind::Text };
            }
            ("p" | "div" | "ul" | "ol" | "table" | "tr", _) | ("h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "li", true) => {
                flush(&mut out, &mut spans, kind);
                kind = if quoted > 0 { Kind::Quote } else { Kind::Text };
            }
            ("br", _) => add(&mut spans, "\n".to_owned(), look(&open, &links)),
            ("img", false) => {
                if let Some(src) = attribute(tag, "src").and_then(|src| resolved(&src, base)) {
                    flush(&mut out, &mut spans, kind);
                    if src.starts_with("http") {
                        out.push(Block::Image(src));
                    }
                }
            }
            ("b" | "strong" | "i" | "em" | "code" | "pre" | "a" | "span" | "tg-emoji" | "u" | "s" | "del" | "tg-spoiler", false) if !tag.ends_with('/') => {
                let inside_emoji = open.last().is_some_and(|(_, effect)| *effect == Effect::Emoji);
                let effect = match name.as_str() {
                    _ if inside_emoji => Effect::Nothing,
                    "i" if attribute(tag, "class").is_some_and(|class| class.contains("emoji")) => Effect::Emoji,
                    "b" | "strong" => Effect::Bold,
                    "i" | "em" => Effect::Italic,
                    "code" | "pre" => Effect::Code,
                    "a" => match attribute(tag, "href").and_then(|href| resolved(&href, base)) {
                        Some(link) => {
                            links.push(link);
                            Effect::Link
                        }
                        None => Effect::Nothing,
                    },
                    _ => Effect::Nothing,
                };
                open.push((name, effect));
            }
            ("b" | "strong" | "i" | "em" | "code" | "pre" | "a" | "span" | "tg-emoji" | "u" | "s" | "del" | "tg-spoiler", true) => {
                if let Some(found) = open.iter().rposition(|(opened, _)| *opened == name) {
                    for (_, effect) in open.drain(found..) {
                        if effect == Effect::Link {
                            links.pop();
                        }
                    }
                }
            }
            _ => {}
        }
    }
    if hidden == 0 {
        add(&mut spans, collapsed(&unescaped(rest)), look(&open, &links));
    }
    flush(&mut out, &mut spans, kind);
    out
}

pub fn builds_from(page: &str) -> Result<Vec<Build>, String> {
    let json = between(page, "<script id=\"json-index\" type=\"application/json\">", "</script>").ok_or("the changelog page has no index")?;
    let index: serde_json::Value = serde_json::from_str(json.trim()).map_err(|e| e.to_string())?;
    let builds = index.get("builds").and_then(|b| b.as_array()).ok_or("the changelog index has no builds")?;
    Ok(builds
        .iter()
        .filter_map(|build| {
            let stream = build.pointer("/update_stream/display_name")?.as_str()?.to_owned();
            let slug = build.pointer("/update_stream/name")?.as_str()?.to_owned();
            let version = build.get("display_version").or_else(|| build.get("version"))?.as_str()?.to_owned();
            let at = unix_of(build.get("created_at")?.as_str()?)?;
            let mut changes: Vec<Change> = build
                .get("changelog_entries")
                .and_then(|entries| entries.as_array())
                .map(|entries| {
                    entries
                        .iter()
                        .filter_map(|entry| {
                            Some(Change {
                                category: entry.get("category")?.as_str().unwrap_or_default().to_owned(),
                                title: entry.get("title")?.as_str()?.to_owned(),
                                major: entry.get("major").and_then(|m| m.as_bool()).unwrap_or(false),
                                kind: entry.get("type").and_then(|t| t.as_str()).unwrap_or_default().to_owned(),
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();
            changes.sort_by_key(|change| !change.major);
            Some(Build { url: format!("{CHANGELOG}/{slug}/{version}"), stream, version, at, changes })
        })
        .collect())
}

fn link_of(entry: &str) -> Option<String> {
    let tag = between(entry, "<link", ">")?;
    between(tag, "href=\"", "\"").map(unescaped)
}

pub fn stories_from(atom: &str) -> Vec<Story> {
    pieces(atom, "<entry>", "</entry>")
        .filter_map(|entry| {
            let title = plain(between(entry, "<title>", "</title>")?);
            let url = link_of(entry)?;
            let at = unix_of(between(entry, "<published>", "</published>")?)?;
            let content = between(entry, "<content type=\"html\">", "</content>").map(unescaped).unwrap_or_default();
            let lead = between(&content, "<p class=\"osu-md__paragraph\">", "</p>").map(plain).unwrap_or_default();
            let image = between(&content, "<img", ">").and_then(|tag| between(tag, "src=\"", "\"")).map(unescaped);
            let body = blocks_of(&content, &url, Flow::Article);
            Some(Story { title, url, at, lead, image, body })
        })
        .collect()
}

pub fn threads_from(atom: &str) -> Vec<Thread> {
    pieces(atom, "<entry>", "</entry>")
        .filter_map(|entry| {
            let author = between(entry, "<name>", "</name>").map(plain).unwrap_or_default();
            if author.ends_with("AutoModerator") {
                return None;
            }
            let content = between(entry, "<content type=\"html\">", "</content>").map(unescaped).unwrap_or_default();
            let content = content.split("submitted by").next().unwrap_or_default();
            let url = link_of(entry)?;
            Some(Thread {
                title: plain(between(entry, "<title>", "</title>")?),
                body: blocks_of(content, &url, Flow::Article),
                url,
                author: author.trim_start_matches("/u/").to_owned(),
                at: unix_of(between(entry, "<published>", "</published>")?)?,
            })
        })
        .collect()
}

fn photos_in(message: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = message;
    while let Some(at) = rest.find("tgme_widget_message_photo_wrap") {
        rest = &rest[at + "tgme_widget_message_photo_wrap".len()..];
        let tag = rest.split('>').next().unwrap_or_default();
        if let Some(url) = between(tag, "background-image:url('", "')").map(unescaped) {
            if !out.contains(&url) {
                out.push(url);
            }
        }
    }
    out
}

fn videos_in(message: &str) -> Vec<Video> {
    let mut out = Vec::new();
    for class in ["tgme_widget_message_video_player", "tgme_widget_message_roundvideo_player"] {
        let mut from = 0;
        while let Some(found) = message[from..].find(class) {
            let at = from + found;
            let start = message[..at].rfind("<a").unwrap_or(at);
            let tag_end = message[at..].find('>').map_or(message.len(), |end| at + end + 1);
            let block_end = message[tag_end..].find("</a>").map_or(message.len(), |end| tag_end + end);
            let tag = &message[start..tag_end];
            let block = &message[tag_end..block_end];
            let duration = between(block, "message_video_duration", "</time>")
                .and_then(|said| said.split('>').nth(1))
                .map(|said| plain(said))
                .unwrap_or_default();
            out.push(Video {
                thumb: between(block, "background-image:url('", "')").map(unescaped),
                src: between(block, "<video src=\"", "\"").map(unescaped),
                duration,
                link: attribute(tag, "href").map(|href| unescaped(&href)).unwrap_or_default(),
            });
            from = block_end;
        }
    }
    out
}

pub fn posts_from(channel: &str, page: &str) -> Vec<Post> {
    let name = between(page, "<meta property=\"og:title\" content=\"", "\"").map(unescaped).unwrap_or_else(|| channel.to_owned());
    let mut posts: Vec<Post> = pieces(page, "<div class=\"tgme_widget_message_wrap", "<div class=\"tgme_widget_message_wrap")
        .chain(after(page, "<div class=\"tgme_widget_message_wrap").map(|last| last.rsplit("<div class=\"tgme_widget_message_wrap").next().unwrap_or(last)))
        .filter_map(|message| {
            let id = between(message, "data-post=\"", "\"")?;
            let markup = between(message, "js-message_text\" dir=\"auto\">", "</div>").unwrap_or_default();
            let text = plain(markup);
            let body = blocks_of(markup, &format!("https://t.me/s/{channel}"), Flow::Post);
            let images = photos_in(message);
            let videos = videos_in(message);
            let image = images.first().cloned();
            if text.is_empty() && image.is_none() && videos.is_empty() {
                return None;
            }
            Some(Post {
                channel: channel.to_owned(),
                name: name.clone(),
                text,
                url: format!("https://t.me/{id}"),
                at: between(message, "<time datetime=\"", "\"").and_then(unix_of).unwrap_or(0),
                image,
                body,
                images,
                videos,
            })
        })
        .collect();
    posts.sort_by(|a, b| b.at.cmp(&a.at));
    posts.dedup_by(|a, b| a.url == b.url);
    posts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn every_source_answers_through_the_applications_own_client() {
        let builds = fetch_builds().expect("the changelog");
        let stories = fetch_stories().expect("the news");
        let posts = fetch_posts("osunewsru").expect("the channel");
        assert!(!builds.is_empty() && !stories.is_empty() && !posts.is_empty());
        assert!(stories.iter().filter(|story| story.image.is_some()).count() > stories.len() / 2);
        assert!(stories.iter().filter(|story| story.body.len() > 3).count() > stories.len() / 2, "the news came without its articles");
        assert!(posts.iter().all(|post| post.at > 0));
    }

    #[test]
    fn a_source_is_fresh_for_a_while_and_the_subreddit_for_longer() {
        let mut news = News::default();
        assert!(news.stale(UPDATES, 1_000));
        news.mark(UPDATES, 1_000);
        news.mark(THREADS, 1_000);
        assert!(!news.stale(UPDATES, 1_000 + STALE_AFTER));
        assert!(news.stale(UPDATES, 1_000 + STALE_AFTER + 1));
        assert!(!news.stale(THREADS, 1_000 + STALE_AFTER + 1));
    }

    #[test]
    fn an_article_comes_apart_into_what_a_reader_shows() {
        let html = r#"<div class='osu-md'><h2>The final</h2><p class="x">It was <strong>close</strong> &amp; loud.</p>
        <p><img src="https://i.ppy.sh/a.jpg" alt=""></p><ul><li>First</li><li>Second</li></ul>
        <blockquote><p>A quote</p></blockquote><script>ignored()</script><p>After</p></div>"#;
        let blocks = blocks_of(html, "https://osu.ppy.sh/home/news/x", Flow::Article);
        let words: Vec<(&str, String)> = blocks
            .iter()
            .map(|block| {
                let kind = match block {
                    Block::Heading(_) => "heading",
                    Block::Text(_) => "text",
                    Block::Item(_) => "item",
                    Block::Quote(_) => "quote",
                    Block::Image(_) => "image",
                };
                (kind, if let Block::Image(url) = block { url.clone() } else { block.words() })
            })
            .collect();
        assert_eq!(
            words,
            vec![
                ("heading", "The final".to_owned()),
                ("text", "It was close & loud.".to_owned()),
                ("image", "https://i.ppy.sh/a.jpg".to_owned()),
                ("item", "First".to_owned()),
                ("item", "Second".to_owned()),
                ("quote", "A quote".to_owned()),
                ("text", "After".to_owned()),
            ]
        );
        let Block::Text(spans) = &blocks[1] else { panic!("a paragraph") };
        assert_eq!(spans.iter().map(|span| (span.text.as_str(), span.bold)).collect::<Vec<_>>(), vec![("It was ", false), ("close", true), (" & loud.", false)]);
    }

    #[test]
    fn a_link_in_an_article_leads_somewhere_whole() {
        let html = r##"<p>See <a href="/wiki/Tournaments">the wiki</a>, <a href="https://x.com/a">this</a> and <a href="#top">nothing</a>.</p>"##;
        let blocks = blocks_of(html, "https://osu.ppy.sh/home/news/2026-09-22-x", Flow::Article);
        let Block::Text(spans) = &blocks[0] else { panic!("a paragraph") };
        let links: Vec<(&str, Option<&str>)> = spans.iter().map(|span| (span.text.as_str(), span.link.as_deref())).collect();
        assert_eq!(
            links,
            vec![
                ("See ", None),
                ("the wiki", Some("https://osu.ppy.sh/wiki/Tournaments")),
                (", ", None),
                ("this", Some("https://x.com/a")),
                (" and nothing.", None),
            ]
        );
    }

    #[test]
    fn a_post_keeps_its_lines_and_paragraphs_as_written() {
        let markup = r#"<a href="https://osu.ppy.sh/users/1" onclick="return confirm('Open?');"><b>quaccking</b></a> (#27<i class="emoji" style="background-image:url('//telegram.org/e.png')"><b>🇦🇹</b></i>) поставил <b>первое DT FC </b>на карте<br/>вторая строка<br/><br/><a href="?q=%23скор"><b>#скор</b></a><b><br/><br/></b>подпись"#;
        let blocks = blocks_of(markup, "https://t.me/s/osunewsru", Flow::Post);
        assert_eq!(blocks.len(), 3, "{blocks:?}");
        assert_eq!(blocks[0].words(), "quaccking (#27🇦🇹) поставил первое DT FC на карте\nвторая строка");
        let Block::Text(first) = &blocks[0] else { panic!("text") };
        assert_eq!(first[0].link.as_deref(), Some("https://osu.ppy.sh/users/1"));
        assert!(first[0].bold);
        assert!(first.iter().all(|span| !span.italic), "an emoji is not italic");
        let Block::Text(tag) = &blocks[1] else { panic!("text") };
        assert_eq!(tag[0].link.as_deref(), Some("https://t.me/s/osunewsru?q=%23скор"));
        assert_eq!(blocks[2].words(), "подпись");
    }

    #[test]
    fn a_post_brings_its_album_and_its_videos() {
        let page = r#"<meta property="og:title" content="news">
        <div class="tgme_widget_message_wrap js-widget_message_wrap"><div class="tgme_widget_message" data-post="news/7">
        <a class="tgme_widget_message_photo_wrap grouped" style="width:1px;background-image:url('https://cdn/a.jpg')"></a>
        <a class="tgme_widget_message_photo_wrap grouped" style="width:1px;background-image:url('https://cdn/b.jpg')"></a>
        <a class="tgme_widget_message_video_player js-message_video_player" href="https://t.me/news/8"><i class="tgme_widget_message_video_thumb" style="background-image:url('https://cdn/v.jpg')"></i>
        <div class="tgme_widget_message_video_wrap"><video src="https://cdn/v.mp4?token=a&amp;b=1" class="tgme_widget_message_video js-message_video"></video></div>
        <time class="message_video_duration js-message_video_duration">0:20</time></a>
        <a class="tgme_widget_message_video_player not_supported js-message_video_player" href="https://t.me/news/9"><i class="tgme_widget_message_video_thumb" style="background-image:url('https://cdn/w.jpg')"></i>
        <time class="message_video_duration js-message_video_duration">7:21</time><div class="message_media_not_supported_label">Media is too big</div></a>
        <time datetime="2026-09-23T10:00:00+00:00" class="time"></time></div></div>"#;
        let posts = posts_from("news", page);
        assert_eq!(posts.len(), 1, "a post of pictures and videos alone is still a post");
        let post = &posts[0];
        assert_eq!(post.images, vec!["https://cdn/a.jpg".to_owned(), "https://cdn/b.jpg".to_owned()]);
        assert_eq!(post.videos.len(), 2);
        assert_eq!(post.videos[0].src.as_deref(), Some("https://cdn/v.mp4?token=a&b=1"));
        assert_eq!(post.videos[0].duration, "0:20");
        assert_eq!(post.videos[0].link, "https://t.me/news/8");
        assert_eq!(post.videos[1].src, None, "a video too big for the page is only a link");
        assert_eq!(post.videos[1].thumb.as_deref(), Some("https://cdn/w.jpg"));
        assert_eq!(post.cover(), Some("https://cdn/a.jpg"));
    }

    #[test]
    fn spaces_around_breaks_and_between_pieces_are_one_space() {
        let blocks = blocks_of("  a <b> b </b>  c <br/>  d  ", "https://x.org", Flow::Post);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].words(), "a b c\nd");
    }

    #[test]
    fn a_stamp_becomes_the_second_it_names() {
        assert_eq!(unix_of("1970-01-01T00:00:00+00:00"), Some(0));
        assert_eq!(unix_of("2026-09-22T21:45:00+00:00"), Some(1_790_113_500));
        assert_eq!(unix_of("2026-09-23T00:45:00+03:00"), Some(1_790_113_500));
        assert_eq!(unix_of("2026-09-22T21:45:00.123Z"), Some(1_790_113_500));
        assert_eq!(unix_of("not a date"), None);
    }

    #[test]
    fn markup_comes_out_as_words() {
        assert_eq!(unescaped("Q&amp;A &#39;quoted&#039; &#x41;&lt;b&gt; a&b"), "Q&A 'quoted' A<b> a&b");
        assert_eq!(plain("<b>13 years</b> of<br/>features &amp; more"), "13 years of features & more");
    }

    #[test]
    fn a_channel_is_named_however_it_is_written() {
        assert_eq!(channel_name("@osunewsru").as_deref(), Some("osunewsru"));
        assert_eq!(channel_name("https://t.me/osunewsru").as_deref(), Some("osunewsru"));
        assert_eq!(channel_name("t.me/s/osunewsru/").as_deref(), Some("osunewsru"));
        assert_eq!(channel_name("../../etc"), None);
        assert_eq!(channel_name("ab"), None);
    }

    #[test]
    fn the_changelog_page_gives_its_builds_major_changes_first() {
        let page = r#"<html><script id="json-index" type="application/json">
        {"builds":[{"created_at":"2026-09-20T07:25:37+00:00","display_version":"2026.920.0","version":"2026.920.0",
        "update_stream":{"name":"lazer","display_name":"Lazer"},
        "changelog_entries":[{"category":"Gameplay","title":"Small fix","major":false,"type":"fix"},
        {"category":"Platform","title":"Big change","major":true,"type":"add"}]}]}
        </script></html>"#;
        let builds = builds_from(page).expect("builds");
        assert_eq!(builds.len(), 1);
        assert_eq!(builds[0].stream, "Lazer");
        assert_eq!(builds[0].url, "https://osu.ppy.sh/home/changelog/lazer/2026.920.0");
        assert_eq!(builds[0].changes[0].title, "Big change");
    }

    #[test]
    fn a_news_entry_gives_its_title_lead_and_picture() {
        let atom = r#"<feed><entry>
            <published>2026-09-22T21:45:00+00:00</published>
            <link rel="alternate" type="text/html" href="https://osu.ppy.sh/home/news/2026-09-22-recap" />
            <title>World Cup: Quarterfinals &amp; Semifinals Recap</title>
            <content type="html">&lt;div&gt;&lt;p class=&quot;osu-md__paragraph&quot;&gt;Enjoy a recap!&lt;/p&gt;&lt;img class=&quot;x&quot; src=&quot;https://i.ppy.sh/a.jpg&quot;&gt;</content>
        </entry></feed>"#;
        let stories = stories_from(atom);
        assert_eq!(stories.len(), 1);
        assert_eq!(stories[0].title, "World Cup: Quarterfinals & Semifinals Recap");
        assert_eq!(stories[0].lead, "Enjoy a recap!");
        assert_eq!(stories[0].image.as_deref(), Some("https://i.ppy.sh/a.jpg"));
        assert_eq!(stories[0].at, 1_790_113_500);
    }

    #[test]
    fn a_subreddit_gives_its_threads_without_the_robot() {
        let atom = r#"<feed>
        <entry><author><name>/u/AutoModerator</name></author><link href="https://www.reddit.com/r/osugame/comments/1/" /><published>2026-09-01T21:00:32+00:00</published><title>Monthly thread</title></entry>
        <entry><author><name>/u/player</name></author><link href="https://www.reddit.com/r/osugame/comments/2/" /><published>2026-09-22T10:00:00+00:00</published><title>New &amp; shiny</title></entry>
        </feed>"#;
        let threads = threads_from(atom);
        assert_eq!(threads.len(), 1);
        assert_eq!(threads[0].author, "player");
        assert_eq!(threads[0].title, "New & shiny");
    }

    #[test]
    fn a_channel_page_gives_its_posts_newest_first() {
        let page = r#"<meta property="og:title" content="осу&#33;новостник">
        <div class="tgme_widget_message_wrap js-widget_message_wrap"><div class="tgme_widget_message" data-post="osunewsru/1">
        <div class="tgme_widget_message_text js-message_text" dir="auto">first <b>post</b></div><time datetime="2026-09-22T10:00:00+00:00" class="time"></time></div></div>
        <div class="tgme_widget_message_wrap js-widget_message_wrap"><div class="tgme_widget_message" data-post="osunewsru/2">
        <a class="tgme_widget_message_photo_wrap" style="width:1px;background-image:url('https://cdn/p.jpg')"></a>
        <div class="tgme_widget_message_text js-message_text" dir="auto">second<br/>line</div><time datetime="2026-09-23T10:00:00+00:00" class="time"></time></div></div>"#;
        let posts = posts_from("osunewsru", page);
        assert_eq!(posts.len(), 2);
        assert_eq!(posts[0].name, "осу!новостник");
        assert_eq!(posts[0].text, "second line");
        assert_eq!(posts[0].url, "https://t.me/osunewsru/2");
        assert_eq!(posts[0].image.as_deref(), Some("https://cdn/p.jpg"));
        assert_eq!(posts[1].text, "first post");
    }
}
