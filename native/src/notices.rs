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

    pub fn remove(&mut self, id: u64) {
        self.notices.retain(|n| n.id != id);
        self.save();
    }

    pub fn get(&self, id: u64) -> Option<&Notice> {
        self.notices.iter().find(|n| n.id == id)
    }
}

#[cfg(test)]
mod tests {
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
        let _ = std::fs::remove_dir_all(&dir);
    }
}
