use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mark {
    Done,
    Bad,
    Plain,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Link {
    OpenVideo(PathBuf),
    RenderAgain(PathBuf),
    Update,
    Page(String),
    Replay(PathBuf),
    Received(u64),
    None,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Notice {
    pub id: u64,
    pub mark: Mark,
    pub words: String,
    pub detail: String,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub map_hash: String,
    pub at: i64,
    pub link: Link,
    pub seen: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Queue {
    pub notices: Vec<Notice>,
    next: u64,
    #[serde(skip)]
    at: PathBuf,
}

pub const KEEP: usize = 100;

pub fn path() -> PathBuf {
    crate::sources::own_root().join("notices.json")
}

impl Queue {
    pub fn load() -> Queue {
        Queue::at(path())
    }

    pub fn at(index: PathBuf) -> Queue {
        let mut queue: Queue = std::fs::read(&index)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        queue.at = index;
        queue
    }

    pub fn save(&self) {
        if self.at.as_os_str().is_empty() {
            return;
        }
        if let Some(dir) = self.at.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(text) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(&self.at, text);
        }
    }

    pub fn push(&mut self, mark: Mark, words: String, detail: String, note: String, map_hash: String, link: Link) -> u64 {
        let id = self.next;
        self.next += 1;
        self.notices.insert(0, Notice { id, mark, words, detail, note, map_hash, at: chrono::Utc::now().timestamp(), link, seen: false });
        self.notices.truncate(KEEP);
        self.save();
        id
    }

    pub fn unseen(&self) -> usize {
        self.notices.iter().filter(|n| !n.seen).count()
    }

    pub fn see_all(&mut self) {
        for notice in &mut self.notices {
            notice.seen = true;
        }
        self.save();
    }

    pub fn clear(&mut self) {
        self.notices.clear();
        self.save();
    }

    pub fn remove(&mut self, id: u64) {
        self.notices.retain(|n| n.id != id);
        self.save();
    }

    pub fn get(&self, id: u64) -> Option<&Notice> {
        self.notices.iter().find(|n| n.id == id)
    }
}

pub const LOG_MOST: u64 = 512 * 1024;

pub fn log_path() -> std::path::PathBuf {
    crate::sources::own_root().join("errors.log")
}

pub fn log_line(at_secs: u64, words: &str, detail: &str, note: &str) -> String {
    let flat = |said: &str| said.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut line = format!("{at_secs} {}", flat(words));
    for part in [detail, note] {
        if !part.trim().is_empty() {
            line.push_str(" | ");
            line.push_str(&flat(part));
        }
    }
    line
}

pub fn log_to(file: &std::path::Path, line: &str) {
    use std::io::Write;
    if std::fs::metadata(file).is_ok_and(|held| held.len() > LOG_MOST) {
        let _ = std::fs::rename(file, file.with_extension("old.log"));
    }
    if let Some(dir) = file.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(mut out) = std::fs::OpenOptions::new().create(true).append(true).open(file) {
        let _ = writeln!(out, "{line}");
    }
}

pub fn log(words: &str, detail: &str, note: &str) {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |since| since.as_secs());
    log_to(&log_path(), &log_line(now, words, detail, note));
}

#[cfg(test)]
mod tests {
    #[test]
    fn an_error_becomes_one_line_of_the_log_and_an_overgrown_log_is_set_aside() {
        assert_eq!(super::log_line(1_790_000_000, "Could not send the video", "Sayuri - Heikousen\n[Meg's Taboo]", "not there"), "1790000000 Could not send the video | Sayuri - Heikousen [Meg's Taboo] | not there");
        assert_eq!(super::log_line(5, "Saved badly", "", " "), "5 Saved badly");
        let file = std::env::temp_dir().join(format!("dossier-errors-{}.log", std::process::id()));
        let _ = std::fs::remove_file(&file);
        super::log_to(&file, "1 first");
        super::log_to(&file, "2 second");
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "1 first\n2 second\n");
        std::fs::write(&file, vec![b'x'; super::LOG_MOST as usize + 1]).unwrap();
        super::log_to(&file, "3 third");
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "3 third\n");
        assert!(file.with_extension("old.log").is_file());
        let _ = std::fs::remove_file(&file);
        let _ = std::fs::remove_file(file.with_extension("old.log"));
    }

    use super::*;

    #[test]
    fn the_queue_keeps_the_newest_hundred_and_counts_the_unseen() {
        let dir = std::env::temp_dir().join(format!("dossier-notices-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut queue = Queue::at(dir.join("notices.json"));
        for i in 0..(KEEP + 5) {
            queue.push(Mark::Plain, format!("n{i}"), String::new(), String::new(), String::new(), Link::None);
        }
        assert_eq!(queue.notices.len(), KEEP);
        assert_eq!(queue.notices[0].words, format!("n{}", KEEP + 4));
        assert_eq!(queue.unseen(), KEEP);
        queue.see_all();
        let again = Queue::at(dir.join("notices.json"));
        assert_eq!(again.unseen(), 0);
        assert_eq!(again.notices.len(), KEEP);
        queue.clear();
        assert!(queue.notices.is_empty());
        assert!(Queue::at(dir.join("notices.json")).notices.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
