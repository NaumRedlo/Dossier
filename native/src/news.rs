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

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Story {
    pub title: String,
    pub url: String,
    pub at: i64,
    pub lead: String,
    pub image: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Thread {
    pub title: String,
    pub url: String,
    pub author: String,
    pub at: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Post {
    pub channel: String,
    pub name: String,
    pub text: String,
    pub url: String,
    pub at: i64,
    pub image: Option<String>,
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
            Some(Story { title, url, at, lead, image })
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
            Some(Thread {
                title: plain(between(entry, "<title>", "</title>")?),
                url: link_of(entry)?,
                author: author.trim_start_matches("/u/").to_owned(),
                at: unix_of(between(entry, "<published>", "</published>")?)?,
            })
        })
        .collect()
}

pub fn posts_from(channel: &str, page: &str) -> Vec<Post> {
    let name = between(page, "<meta property=\"og:title\" content=\"", "\"").map(unescaped).unwrap_or_else(|| channel.to_owned());
    let mut posts: Vec<Post> = pieces(page, "<div class=\"tgme_widget_message_wrap", "<div class=\"tgme_widget_message_wrap")
        .chain(after(page, "<div class=\"tgme_widget_message_wrap").map(|last| last.rsplit("<div class=\"tgme_widget_message_wrap").next().unwrap_or(last)))
        .filter_map(|message| {
            let id = between(message, "data-post=\"", "\"")?;
            let text = between(message, "js-message_text\" dir=\"auto\">", "</div>").map(plain).unwrap_or_default();
            let image = between(message, "tgme_widget_message_photo_wrap", ">")
                .and_then(|tag| between(tag, "background-image:url('", "')"))
                .map(unescaped);
            if text.is_empty() && image.is_none() {
                return None;
            }
            Some(Post {
                channel: channel.to_owned(),
                name: name.clone(),
                text,
                url: format!("https://t.me/{id}"),
                at: between(message, "<time datetime=\"", "\"").and_then(unix_of).unwrap_or(0),
                image,
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
