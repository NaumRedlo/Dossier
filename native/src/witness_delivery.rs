use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::witness::{Kept, Sitting, Told};

const BETWEEN: Duration = Duration::from_secs(6);
const RETRY_MAX: Duration = Duration::from_secs(300);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "payload", rename_all = "snake_case")]
pub enum Request {
    Play(Told),
    Session(Sitting),
    Deferred { kept: Kept, ended: i64 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Delivery {
    pub owner: String,
    pub request: Request,
}

impl Delivery {
    fn key(&self) -> String {
        match &self.request {
            Request::Play(play) => format!("{}:play:{}", self.owner, play.replay),
            Request::Session(sitting) => format!("{}:session:{}", self.owner, sitting.started_at),
            Request::Deferred { kept, .. } => format!("{}:deferred:{}", self.owner, kept.osr),
        }
    }

    fn same_key(&self, other: &Self) -> bool {
        if self.owner != other.owner {
            return false;
        }
        match (&self.request, &other.request) {
            (Request::Play(a), Request::Play(b)) => a.replay == b.replay,
            (Request::Session(a), Request::Session(b)) => a.started_at == b.started_at,
            (Request::Deferred { kept: a, .. }, Request::Deferred { kept: b, .. }) => {
                a.osr == b.osr
            }
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Queue {
    path: PathBuf,
    pending: Vec<Delivery>,
    broken: Option<String>,
    in_flight: Option<Delivery>,
    next: Instant,
    retries: HashMap<String, (u32, Instant)>,
}

impl Queue {
    pub fn load(path: PathBuf) -> Self {
        let (pending, broken) = match fs::read(&path) {
            Ok(bytes) => match serde_json::from_slice::<Vec<Delivery>>(&bytes) {
                Ok(pending) => (pending, None),
                Err(why) => (Vec::new(), Some(why.to_string())),
            },
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => (Vec::new(), None),
            Err(why) => (Vec::new(), Some(why.to_string())),
        };
        Self {
            path,
            pending,
            broken,
            in_flight: None,
            next: Instant::now(),
            retries: HashMap::new(),
        }
    }

    pub fn own() -> Self {
        Self::load(crate::sources::own_root().join("witness-outbox.json"))
    }

    pub fn pending(&self) -> usize {
        self.pending.len() + usize::from(self.broken.is_some())
    }

    pub fn broken(&self) -> bool {
        self.broken.is_some()
    }

    pub fn has_deferred(&self, owner: &str) -> bool {
        self.pending
            .iter()
            .any(|item| item.owner == owner && matches!(item.request, Request::Deferred { .. }))
    }

    pub fn resolve(&mut self, owner: &str, player: &str) -> Result<(), String> {
        if let Some(why) = &self.broken {
            return Err(why.clone());
        }
        let mut pending = Vec::with_capacity(self.pending.len());
        let mut changed = false;
        for item in &self.pending {
            if item.owner != owner {
                pending.push(item.clone());
                continue;
            }
            match &item.request {
                Request::Deferred { kept, ended } => {
                    changed = true;
                    if let Some(play) = crate::witness::told(kept, player, *ended) {
                        let converted = Delivery {
                            owner: owner.to_owned(),
                            request: Request::Play(play),
                        };
                        if !pending
                            .iter()
                            .chain(self.pending.iter())
                            .any(|other| other.same_key(&converted))
                        {
                            pending.push(converted);
                        }
                    }
                }
                _ => pending.push(item.clone()),
            }
        }
        if changed {
            save(&self.path, &pending)?;
            self.pending = pending;
        }
        Ok(())
    }

    pub fn enqueue(&mut self, delivery: Delivery) -> Result<(), String> {
        if let Some(why) = &self.broken {
            return Err(why.clone());
        }
        let mut pending = self.pending.clone();
        if let Some(existing) = pending.iter_mut().find(|item| item.same_key(&delivery)) {
            match (&existing.request, &delivery.request) {
                (Request::Session(before), Request::Session(after))
                    if after.ended_at < before.ended_at =>
                {
                    return Ok(())
                }
                _ => *existing = delivery,
            }
        } else {
            pending.push(delivery);
        }
        save(&self.path, &pending)?;
        self.pending = pending;
        Ok(())
    }

    pub fn ready(&mut self, owner: &str, now: Instant) -> Option<Delivery> {
        if self.broken.is_some() || self.in_flight.is_some() || now < self.next {
            return None;
        }
        let delivery = self
            .pending
            .iter()
            .find(|item| {
                item.owner == owner
                    && !matches!(item.request, Request::Deferred { .. })
                    && self
                        .retries
                        .get(&item.key())
                        .is_none_or(|(_, at)| now >= *at)
            })?
            .clone();
        self.in_flight = Some(delivery.clone());
        Some(delivery)
    }

    pub fn finish(
        &mut self,
        delivery: &Delivery,
        reached: bool,
        now: Instant,
    ) -> Result<bool, String> {
        if self.in_flight.as_ref() != Some(delivery) {
            return Ok(false);
        }
        self.in_flight = None;
        let key = delivery.key();
        if !reached {
            let failures = self
                .retries
                .get(&key)
                .map_or(1, |(failures, _)| failures.saturating_add(1));
            self.retries
                .insert(key, (failures, now + retry_after(failures)));
            self.next = now + BETWEEN;
            return Ok(false);
        }
        self.retries.remove(&key);
        self.next = now + BETWEEN;
        let mut pending = self.pending.clone();
        let before = pending.len();
        pending.retain(|item| item != delivery);
        if pending.len() == before {
            return Ok(false);
        }
        if let Err(why) = save(&self.path, &pending) {
            self.retries.insert(key, (1, now + retry_after(1)));
            return Err(why);
        }
        self.pending = pending;
        Ok(true)
    }
}

pub fn owner(server: &str, token: &str) -> String {
    let mut hash = Sha256::new();
    hash.update(server.as_bytes());
    hash.update([0]);
    hash.update(token.as_bytes());
    format!("{:x}", hash.finalize())
}

fn retry_after(failures: u32) -> Duration {
    Duration::from_secs((10u64 << failures.min(5)).min(RETRY_MAX.as_secs()))
}

fn save(path: &Path, pending: &[Delivery]) -> Result<(), String> {
    let bytes = serde_json::to_vec(pending).map_err(|why| why.to_string())?;
    let parent = path
        .parent()
        .ok_or_else(|| format!("{} has no parent", path.display()))?;
    fs::create_dir_all(parent).map_err(|why| why.to_string())?;
    let staged = path.with_extension("json.tmp");
    let mut file = File::create(&staged).map_err(|why| why.to_string())?;
    file.write_all(&bytes).map_err(|why| why.to_string())?;
    file.sync_all().map_err(|why| why.to_string())?;
    drop(file);
    fs::rename(&staged, path).map_err(|why| why.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static SERIAL: AtomicU64 = AtomicU64::new(0);

    fn path() -> PathBuf {
        std::env::temp_dir().join(format!(
            "dossier-witness-delivery-{}-{}.json",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn a_play_survives_restart_and_is_removed_only_after_confirmation() {
        let path = path();
        let play = Delivery {
            owner: "first".into(),
            request: Request::Play(Told {
                replay: "hash".into(),
                ..Told::default()
            }),
        };
        let mut queue = Queue::load(path.clone());
        queue.enqueue(play.clone()).unwrap();
        assert_eq!(queue.pending(), 1);
        let mut queue = Queue::load(path.clone());
        let now = Instant::now();
        assert!(queue.ready("second", now).is_none());
        assert_eq!(queue.ready("first", now), Some(play.clone()));
        queue.finish(&play, false, now).unwrap();
        assert_eq!(Queue::load(path.clone()).pending(), 1);
        assert!(queue.ready("first", now + Duration::from_secs(1)).is_none());
        assert_eq!(
            queue.ready("first", now + Duration::from_secs(60)),
            Some(play.clone())
        );
        assert!(queue
            .finish(&play, true, now + Duration::from_secs(60))
            .unwrap());
        assert_eq!(Queue::load(path.clone()).pending(), 0);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn a_newer_session_snapshot_is_not_lost_when_an_older_snapshot_completes() {
        let path = path();
        let old = Delivery {
            owner: "first".into(),
            request: Request::Session(Sitting {
                started_at: 1,
                ended_at: 2,
                ..Sitting::default()
            }),
        };
        let newer = Delivery {
            owner: "first".into(),
            request: Request::Session(Sitting {
                started_at: 1,
                ended_at: 9,
                plays: 1,
                ..Sitting::default()
            }),
        };
        let mut queue = Queue::load(path.clone());
        let now = Instant::now();
        queue.enqueue(old.clone()).unwrap();
        assert_eq!(queue.ready("first", now), Some(old.clone()));
        queue.enqueue(newer.clone()).unwrap();
        assert!(!queue.finish(&old, true, now).unwrap());
        assert_eq!(Queue::load(path.clone()).pending(), 1);
        assert_eq!(queue.ready("first", now + BETWEEN), Some(newer));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn invalid_queue_data_is_not_overwritten() {
        let path = path();
        fs::write(&path, b"not json").unwrap();
        let mut queue = Queue::load(path.clone());
        assert_eq!(queue.pending(), 1);
        assert!(queue
            .enqueue(Delivery {
                owner: "first".into(),
                request: Request::Session(Sitting::default())
            })
            .is_err());
        assert_eq!(fs::read(&path).unwrap(), b"not json");
        let _ = fs::remove_file(path);
    }

    #[test]
    fn a_failed_delivery_does_not_block_the_next_play() {
        let path = path();
        let mut queue = Queue::load(path.clone());
        let first = Delivery {
            owner: "first".into(),
            request: Request::Play(Told {
                replay: "one".into(),
                ..Told::default()
            }),
        };
        let second = Delivery {
            owner: "first".into(),
            request: Request::Play(Told {
                replay: "two".into(),
                ..Told::default()
            }),
        };
        queue.enqueue(first.clone()).unwrap();
        queue.enqueue(second.clone()).unwrap();
        let now = Instant::now();
        assert_eq!(queue.ready("first", now), Some(first.clone()));
        queue.finish(&first, false, now).unwrap();
        assert_eq!(queue.ready("first", now + BETWEEN), Some(second));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn an_unidentified_play_waits_on_disk_until_the_players_name_is_known() {
        let path = path();
        let bytes = include_bytes!("../tests/fixtures/witness.osr");
        let player = dossier_replay::Replay::heading(bytes).unwrap().player;
        let kept = Kept {
            osr: bytes.iter().map(|byte| format!("{byte:02x}")).collect(),
            passed: true,
            watched: Some(false),
            ..Kept::default()
        };
        let mut queue = Queue::load(path.clone());
        queue
            .enqueue(Delivery {
                owner: "first".into(),
                request: Request::Deferred { kept, ended: 123 },
            })
            .unwrap();
        let mut queue = Queue::load(path.clone());
        assert!(queue.has_deferred("first"));
        assert!(queue.ready("first", Instant::now()).is_none());
        queue.resolve("first", &player).unwrap();
        assert!(!queue.has_deferred("first"));
        assert!(matches!(
            queue.ready("first", Instant::now()).unwrap().request,
            Request::Play(_)
        ));
        let _ = fs::remove_file(path);
    }
}
