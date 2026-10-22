use super::*;

impl State {
    pub(super) fn resolve(&mut self, target: Target) -> Effect {
        self.resolve_request = self.resolve_request.wrapping_add(1);
        Effect::Resolve(self.resolve_request, target)
    }

    pub(super) fn fetch(&mut self, hash: String, stop: Arc<AtomicBool>) -> Effect {
        self.fetch_request = self.fetch_request.wrapping_add(1);
        self.fetching = Some((self.fetch_request, hash.clone()));
        Effect::Fetch(self.fetch_request, hash, stop)
    }

    pub(super) fn cancel_network(&mut self) {
        if let Some(Finding::Fetching(fetching)) = &self.finding {
            fetching.stop.store(true, Ordering::SeqCst);
        }
        if let Screen::Open(opening) = &self.screen {
            opening.stop.store(true, Ordering::SeqCst);
        }
        self.finding = None;
        self.fetching = None;
    }

    pub(super) fn cancel_import(&mut self) {
        self.importing = None;
        self.importing_pool = None;
        self.dropped.clear();
    }

    pub(super) fn link_next(&mut self) -> Vec<Effect> {
        if self.finding.is_some() || self.importing.is_some() || self.editing().is_none() {
            return Vec::new();
        }
        match self.links.pop_front() {
            Some(link) => self.update(Message::Pasted(link), self.clock),
            None => {
                self.linking = false;
                Vec::new()
            }
        }
    }

    pub(super) fn import_next(&mut self) -> Vec<Effect> {
        if self.importing.is_some() || self.editing().is_none() || matches!(self.finding, Some(Finding::Found(_) | Finding::Fetching(_))) {
            return Vec::new();
        }
        let Some(path) = self.dropped.pop_front() else { return Vec::new() };
        self.file_request = self.file_request.wrapping_add(1);
        let pool = self.editing().unwrap();
        self.importing = Some(FileImport { request: self.file_request, pool: pool.id.clone(), fingerprint: pool.fingerprint(), place: self.default_place() });
        if path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("osz")) {
            vec![Effect::ImportArchive(self.file_request, path)]
        } else {
            vec![Effect::ImportMap(self.file_request, path)]
        }
    }

    pub(super) fn look_at(&mut self, text: &str) -> Vec<Effect> {
        if !matches!(self.finding, Some(Finding::Fetching(_))) {
            self.finding = None;
        } else {
            return Vec::new();
        }
        match pool_links::parse(text) {
            Ok(target) => {
                self.finding = Some(Finding::Asking);
                vec![self.resolve(target)]
            }
            Err(pool_links::Refusal::Mode(mode)) => {
                self.finding = Some(Finding::Refused(mode));
                Vec::new()
            }
            Err(pool_links::Refusal::NotALink) => {
                if text.starts_with("http") || text.contains("ppy.sh") {
                    self.notice = Some(Notice::NotALink);
                }
                Vec::new()
            }
        }
    }

    pub(super) fn confirm(&mut self, all: bool, now: i64) -> Vec<Effect> {
        let Some(Finding::Found(candidate)) = self.finding.take() else { return Vec::new() };
        let queue: Vec<String> = if all {
            candidate.found.difficulties.iter().map(|difficulty| difficulty.hash.clone()).collect()
        } else {
            match candidate.choice.and_then(|at| candidate.found.difficulties.get(at)) {
                Some(difficulty) => vec![difficulty.hash.clone()],
                None => {
                    self.finding = Some(Finding::Found(candidate));
                    return Vec::new();
                }
            }
        };
        let total = queue.len();
        self.finding = Some(Finding::Fetching(Fetching { queue, total, step: None, place: Some(candidate.place), stop: Arc::new(std::sync::atomic::AtomicBool::new(false)) }));
        self.advance(now)
    }

    pub(super) fn advance(&mut self, now: i64) -> Vec<Effect> {
        let mut effects = Vec::new();
        loop {
            let Some(Finding::Fetching(fetching)) = &mut self.finding else { return effects };
            if fetching.queue.is_empty() {
                self.finding = None;
                if let Some(editor) = self.editor_mut() {
                    editor.query.clear();
                }
                effects.extend(self.import_next());
                return effects;
            }
            let hash = fetching.queue.remove(0);
            let place = fetching.place;
            let stop = fetching.stop.clone();
            fetching.step = None;
            let known = self.songs.as_ref().and_then(|songs| songs.get(&hash)).or_else(|| self.fetched.get(&hash)).cloned();
            match known {
                Some(map) => {
                    if let Some(Finding::Fetching(fetching)) = &mut self.finding {
                        fetching.place = None;
                    }
                    effects.extend(self.place_map(&hash, map, place, now));
                }
                None => {
                    effects.push(self.fetch(hash, stop));
                    return effects;
                }
            }
        }
    }

    pub(super) fn begin_open(&mut self, pool: Pool) -> Vec<Effect> {
        self.cancel_network();
        self.cancel_import();
        self.cancel_suggestion();
        if let Some(Finding::Fetching(fetching)) = &self.finding {
            fetching.stop.store(true, Ordering::SeqCst);
        }
        self.refused = None;
        self.finding = None;
        self.screen = Screen::Open(Opening { pool, queue: Vec::new(), total: 0, step: None, stop: Arc::new(std::sync::atomic::AtomicBool::new(false)), running: false, lost: 0 });
        let mut effects = Vec::new();
        if self.songs.is_none() && !self.reading {
            self.reading = true;
            effects.push(Effect::ReadSongs);
        }
        effects.extend(self.measure_effects());
        effects
    }

    pub fn missing(&self, pool: &Pool) -> Vec<String> {
        let Some(songs) = &self.songs else { return Vec::new() };
        let mut seen = HashSet::new();
        pool.slots
            .iter()
            .filter_map(|slot| slot.hash.clone())
            .filter(|hash| !songs.contains_key(hash) && !self.fetched.contains_key(hash))
            .filter(|hash| seen.insert(hash.clone()))
            .collect()
    }

    pub(super) fn get_missing(&mut self, now: i64) -> Vec<Effect> {
        let queue = match &self.screen {
            Screen::Open(opening) if !opening.running => self.missing(&opening.pool),
            _ => return Vec::new(),
        };
        if queue.is_empty() {
            return Vec::new();
        }
        if let Screen::Open(opening) = &mut self.screen {
            opening.total = queue.len();
            opening.queue = queue;
            opening.running = true;
            opening.lost = 0;
            opening.stop.store(false, std::sync::atomic::Ordering::SeqCst);
        }
        self.advance_open(now)
    }

    pub(super) fn advance_open(&mut self, _now: i64) -> Vec<Effect> {
        loop {
            let Screen::Open(opening) = &mut self.screen else { return Vec::new() };
            if opening.queue.is_empty() {
                opening.running = false;
                opening.step = None;
                return self.measure_effects();
            }
            let hash = opening.queue.remove(0);
            opening.step = None;
            let stop = opening.stop.clone();
            let known = self.songs.as_ref().is_some_and(|songs| songs.contains_key(&hash)) || self.fetched.contains_key(&hash);
            if !known {
                return vec![self.fetch(hash, stop)];
            }
        }
    }

    pub(super) fn keep_opened(&mut self, now: i64) -> Vec<Effect> {
        self.cancel_network();
        let Screen::Open(opening) = std::mem::replace(&mut self.screen, Screen::Shelf) else { return Vec::new() };
        opening.stop.store(true, std::sync::atomic::Ordering::SeqCst);
        let mut pool = opening.pool;
        pool.made_at = now;
        pool.changed_at = now;
        let id = pool.id.clone();
        self.list.insert(0, pool);
        self.save(0, now);
        self.screen = Screen::Editor(Editor::at(id));
        self.measure_effects()
    }

    pub(super) fn fetch_next(&mut self, now: i64) -> Vec<Effect> {
        let Screen::Editor(editor) = &self.screen else { return Vec::new() };
        if !editor.fetch_all || self.fetching.is_some() || matches!(self.finding, Some(Finding::Fetching(_))) {
            return Vec::new();
        }
        let tried = editor.fetch_tried.clone();
        let next = self.editing().and_then(|pool| pool.slots.iter().enumerate().find(|(_, slot)| self.ail(slot) == Ail::Missing && slot.hash.as_ref().is_some_and(|hash| !tried.contains(hash))).map(|(at, slot)| (at, slot.hash.clone().unwrap_or_default())));
        match next {
            Some((at, hash)) => {
                if let Some(editor) = self.editor_mut() {
                    editor.fetch_tried.push(hash);
                }
                self.finding = None;
                self.update(Message::FetchSlot(at), now)
            }
            None => {
                if let Some(editor) = self.editor_mut() {
                    editor.fetch_all = false;
                }
                Vec::new()
            }
        }
    }

    pub(super) fn pull_next(&mut self) -> Vec<Effect> {
        loop {
            let Some(pull) = self.pull.as_mut() else { return Vec::new() };
            pull.current = None;
            pull.step = None;
            if pull.queue.is_empty() {
                let id = pull.pool.clone();
                let Some(pool) = self.list.iter().find(|pool| pool.id == id) else { self.pull = None; return Vec::new() };
                let held: Vec<String> = pool.slots.iter().filter_map(|slot| slot.hash.clone()).filter(|hash| self.songs.as_ref().is_some_and(|songs| songs.contains_key(hash)) || self.fetched.contains_key(hash)).collect();
                if held.is_empty() {
                    if let Some(pull) = self.pull.as_mut() { pull.over = Some(Err("empty".to_owned())); }
                    return Vec::new();
                }
                let name = pool.name.clone();
                let mut effects = self.measure_effects();
                effects.push(Effect::Collect(id, name, held));
                return effects;
            }
            let hash = pull.queue.remove(0);
            let stop = pull.stop.clone();
            let known = self.songs.as_ref().is_some_and(|songs| songs.contains_key(&hash)) || self.fetched.contains_key(&hash);
            if !known {
                if let Some(pull) = self.pull.as_mut() { pull.current = Some(hash.clone()); }
                return vec![self.fetch(hash, stop)];
            }
        }
    }

    pub(super) fn pull_stepped(&mut self, hash: &str, step: crate::maps::Step) -> Vec<Effect> {
        use crate::maps::Step;
        let Some(pull) = self.pull.as_mut() else { return Vec::new() };
        pull.step = Some(step.clone());
        match step {
            Step::Done(map) => {
                self.fetched.insert(hash.to_owned(), map.clone());
                if let Some(songs) = self.songs.as_mut() {
                    Arc::make_mut(songs).insert(hash.to_owned(), map);
                }
                self.pull_next()
            }
            Step::Nowhere | Step::Failed(_) => {
                pull.lost += 1;
                self.pull_next()
            }
            Step::Stopped => {
                self.pull = None;
                Vec::new()
            }
            Step::Looking | Step::Found(_) | Step::Downloading { .. } | Step::Unpacking | Step::Checking => Vec::new(),
        }
    }

    pub(super) fn stepped(&mut self, hash: &str, step: crate::maps::Step, now: i64) -> Vec<Effect> {
        use crate::maps::Step;
        if self.pull.as_ref().is_some_and(|pull| pull.current.as_deref() == Some(hash)) {
            return self.pull_stepped(hash, step);
        }
        if let Screen::Open(opening) = &mut self.screen {
            opening.step = Some(step.clone());
            return match step {
                Step::Done(map) => {
                    self.fetched.insert(hash.to_owned(), map.clone());
                    if let Some(songs) = self.songs.as_mut() {
                        Arc::make_mut(songs).insert(hash.to_owned(), map);
                    }
                    let mut effects = self.measure_effects();
                    effects.extend(self.advance_open(now));
                    effects
                }
                Step::Nowhere | Step::Failed(_) => {
                    opening.lost += 1;
                    self.advance_open(now)
                }
                Step::Stopped => {
                    opening.queue.clear();
                    opening.running = false;
                    Vec::new()
                }
                Step::Looking | Step::Found(_) | Step::Downloading { .. } | Step::Unpacking | Step::Checking => Vec::new(),
            };
        }
        let Some(Finding::Fetching(fetching)) = &mut self.finding else { return Vec::new() };
        fetching.step = Some(step.clone());
        match step {
            Step::Done(map) => {
                let place = fetching.place.take();
                self.fetched.insert(hash.to_owned(), map.clone());
                if let Some(songs) = self.songs.as_mut() {
                    Arc::make_mut(songs).insert(hash.to_owned(), map.clone());
                }
                let mut effects = self.place_map(hash, map, place, now);
                effects.extend(self.advance(now));
                effects
            }
            Step::Nowhere => {
                self.finding = Some(Finding::Missing);
                Vec::new()
            }
            Step::Failed(why) => {
                self.finding = Some(Finding::Silent(why));
                Vec::new()
            }
            Step::Stopped => {
                self.finding = None;
                Vec::new()
            }
            Step::Looking | Step::Found(_) | Step::Downloading { .. } | Step::Unpacking | Step::Checking => Vec::new(),
        }
    }
}
