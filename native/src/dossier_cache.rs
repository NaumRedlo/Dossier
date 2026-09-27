use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use crate::community::wire::Me;

const FRESH: i64 = 60;
const RETRY: Duration = Duration::from_secs(15);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub scope: String,
    pub id: i64,
    serial: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Entry {
    pub saved_at: i64,
    pub dossier: Me,
}

#[derive(Debug, Clone, Default)]
pub struct Cache {
    scope: String,
    serial: u64,
    entries: HashMap<i64, Entry>,
    pending: HashMap<i64, Request>,
    attempted: HashMap<i64, Instant>,
}

pub fn scope(server: &str, token: &str, device: &str, chat: i64) -> String {
    use sha2::{Digest, Sha256};
    let identity = serde_json::to_vec(&(server.trim_end_matches('/'), token, device, chat)).unwrap_or_default();
    format!("{:x}", Sha256::digest(identity))
}

impl Cache {
    pub fn configure(&mut self, scope: String) -> bool {
        if self.scope == scope { return false; }
        self.clear();
        self.scope = scope;
        true
    }

    pub fn clear(&mut self) {
        self.serial += 1;
        self.entries.clear();
        self.pending.clear();
        self.attempted.clear();
    }

    pub fn get(&self, id: i64) -> Option<&Entry> { self.entries.get(&id) }
    pub fn loading(&self, id: i64) -> bool { self.pending.contains_key(&id) }

    pub fn request(&mut self, id: i64, now: Instant, unix: i64) -> Option<Request> {
        if self.scope.is_empty() || self.loading(id)
            || self.entries.get(&id).is_some_and(|entry| (0..FRESH).contains(&(unix - entry.saved_at)))
            || self.attempted.get(&id).is_some_and(|at| now.saturating_duration_since(*at) < RETRY) {
            return None;
        }
        self.serial += 1;
        let request = Request { scope: self.scope.clone(), id, serial: self.serial };
        self.pending.insert(id, request.clone());
        self.attempted.insert(id, now);
        Some(request)
    }

    fn matches(&self, request: &Request) -> bool {
        self.scope == request.scope && self.pending.get(&request.id) == Some(request)
    }

    pub fn cached(&mut self, request: &Request, entry: Entry, unix: i64) -> bool {
        if !self.matches(request) || entry.dossier.person.id != request.id
            || entry.saved_at <= 0 || entry.saved_at > unix
            || self.entries.get(&request.id).is_some_and(|old| old.saved_at >= entry.saved_at) {
            return false;
        }
        self.entries.insert(request.id, entry);
        true
    }

    pub fn finish(&mut self, request: &Request, said: Result<Me, String>, unix: i64) -> Option<Entry> {
        if !self.matches(request) { return None; }
        self.pending.remove(&request.id);
        let dossier = said.ok().filter(|me| me.person.id == request.id)?;
        let entry = Entry { saved_at: unix, dossier };
        self.entries.insert(request.id, entry.clone());
        Some(entry)
    }
}

fn path(root: &Path, request: &Request) -> PathBuf {
    root.join("cache").join("dossiers").join(&request.scope).join(format!("{}.json", request.id))
}

pub fn load(root: &Path, request: &Request) -> Option<Entry> {
    std::fs::read(path(root, request)).ok().and_then(|bytes| serde_json::from_slice(&bytes).ok())
}

pub fn save(root: &Path, request: &Request, entry: &Entry) {
    let Ok(bytes) = serde_json::to_vec(entry) else { return; };
    let path = path(root, request);
    let Some(parent) = path.parent() else { return; };
    if std::fs::create_dir_all(parent).is_err() { return; }
    let temporary = path.with_extension(format!("{}.tmp", request.serial));
    if std::fs::write(&temporary, bytes).is_ok() {
        let _ = std::fs::rename(&temporary, path);
    }
    let _ = std::fs::remove_file(temporary);
}

#[cfg(test)]
mod tests {
    use super::*;
    fn me(id: i64, points: u32) -> Me {
        Me { person: crate::community::wire::Person { id, name: "Player".into(), ..Default::default() }, points, ..Default::default() }
    }

    #[test]
    fn cached_data_survives_failure_and_refreshes_without_duplicate_requests() {
        let now = Instant::now();
        let mut cache = Cache::default();
        cache.configure(scope("server", "token", "device", -1));
        let request = cache.request(7, now, 1000).unwrap();
        assert!(cache.cached(&request, Entry { saved_at: 900, dossier: me(7, 10) }, 1000));
        assert!(cache.loading(7));
        assert!(cache.request(7, now, 1000).is_none());
        assert!(cache.finish(&request, Err("offline".into()), 1000).is_none());
        assert_eq!(cache.get(7).unwrap().dossier.points, 10);
        assert!(!cache.loading(7));
        assert!(cache.request(7, now + Duration::from_secs(5), 1005).is_none());
        let retry = cache.request(7, now + RETRY, 1015).unwrap();
        assert!(cache.finish(&request, Ok(me(7, 99)), 1015).is_none());
        cache.finish(&retry, Ok(me(7, 20)), 1015).unwrap();
        assert_eq!(cache.get(7).unwrap().dossier.points, 20);
        assert!(cache.request(7, now + RETRY * 2, 1040).is_none());
        assert!(cache.request(7, now + Duration::from_secs(80), 1080).is_some());
    }

    #[test]
    fn identities_clear_pending_data_and_reject_late_or_wrong_player_replies() {
        let mut cache = Cache::default();
        let one = scope("server", "token", "device", -1);
        for other in [scope("other", "token", "device", -1), scope("server", "other", "device", -1), scope("server", "token", "device", -2)] {
            assert_ne!(one, other);
            cache.configure(one.clone());
            let old = cache.request(7, Instant::now(), 1000).unwrap();
            cache.configure(other);
            let new = cache.request(7, Instant::now(), 1000).unwrap();
            assert!(cache.finish(&old, Ok(me(7, 1)), 1000).is_none());
            assert!(cache.loading(7));
            assert!(cache.finish(&new, Ok(me(8, 1)), 1000).is_none());
            assert!(cache.get(7).is_none());
        }
        cache.configure(one);
        let old = cache.request(7, Instant::now(), 1000).unwrap();
        cache.clear();
        assert!(cache.finish(&old, Ok(me(7, 1)), 1000).is_none());
    }

    #[test]
    fn a_restart_restores_the_full_snapshot_and_corruption_is_a_cache_miss() {
        let root = std::env::temp_dir().join(format!("dossier-cache-{}", std::process::id()));
        let mut cache = Cache::default();
        cache.configure(scope("server", "private-token", "device", -1));
        let request = cache.request(7, Instant::now(), 1000).unwrap();
        let mut dossier = me(7, 30);
        dossier.title_dates.insert("title".into(), Some(900));
        dossier.activity.push(crate::community::wire::Day { day: "2026-09-27".into(), n: 12 });
        dossier.recent.push(crate::community::wire::Recent::default());
        dossier.duels = vec![4, 2];
        dossier.history.push(crate::community::wire::Week { at: Some(900), pp: 234.0, ..Default::default() });
        dossier.card = Some(crate::community::wire::Card { username: "Player".into(), pp: 1234.0, ..Default::default() });
        let entry = cache.finish(&request, Ok(dossier.clone()), 1000).unwrap();
        save(&root, &request, &entry);
        let bytes = std::fs::read(path(&root, &request)).unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains("private-token"));
        assert_eq!(load(&root, &request).unwrap().dossier, dossier);
        let mut restart = Cache::default();
        restart.configure(request.scope.clone());
        let reopened = restart.request(7, Instant::now(), 1100).unwrap();
        assert!(restart.cached(&reopened, load(&root, &request).unwrap(), 1100));
        assert_eq!(restart.get(7).unwrap().dossier, dossier);
        assert!(!restart.cached(&reopened, Entry { saved_at: 1200, dossier: me(7, 40) }, 1100));
        std::fs::write(path(&root, &request), "broken").unwrap();
        assert!(load(&root, &request).is_none());
        std::fs::remove_dir_all(root).unwrap();
    }
}
