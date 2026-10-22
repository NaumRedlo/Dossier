use super::*;

impl State {
    pub(super) fn handle_sources(&mut self, message: Message, now: i64) -> Vec<Effect> {
        let mut effects = Vec::new();
        match message {
            Message::Lean(skill) => {
                effects.extend(self.update(Message::AddPanel(true), now));
                if let Some(editor) = self.editor_mut() {
                    editor.lean = Some(skill);
                }
                effects.extend(self.update(Message::Source(SourceTab::Suggest), now));
            }
            Message::AddPanel(open) => {
                if open {
                    if let Some(at) = self.editing_at() {
                        let empty = match &self.screen {
                            Screen::Editor(editor) => editor.selected.filter(|slot| self.list[at].slots.get(*slot).is_some_and(Slot::is_empty)),
                            _ => None,
                        }.or_else(|| self.list[at].first_empty());
                        let slot = match empty {
                            Some(slot) => slot,
                            None => {
                                self.remember(at);
                                let slot = self.list[at].slots.len();
                                let mods = match &self.screen { Screen::Editor(editor) => editor.last_mod, _ => Mod::Nm };
                                self.list[at].slots.push(Slot::empty(mods));
                                self.save(at, now);
                                slot
                            }
                        };
                        let Some(at) = self.editing_at() else { return effects };
                        let automatic = matches!(&self.screen, Screen::Editor(editor) if editor.selected != Some(slot));
                        if automatic && self.list[at].frame == Frame::Free {
                            let mods = match &self.screen { Screen::Editor(editor) => editor.last_mod, _ => Mod::Nm };
                            let (category, colour) = match &self.screen { Screen::Editor(editor) => (editor.last_category.clone(), editor.last_colour), _ => (String::new(), None) };
                            if self.list[at].slots[slot].mods != mods || self.list[at].slots[slot].category != category || self.list[at].slots[slot].colour != colour {
                                self.remember(at);
                                self.list[at].slots[slot].mods = mods;
                                self.list[at].slots[slot].category = category;
                                self.list[at].slots[slot].colour = colour;
                                self.list[at].slots[slot].measure = None;
                                self.save(at, now);
                            }
                        }
                        if let Some(editor) = self.editor_mut() { editor.selected = Some(slot); }
                    }
                }
                if !open {
                    self.cancel_suggestion();
                }
                self.notice = None;
                if let Some(editor) = self.editor_mut() {
                    editor.replace = false;
                    editor.panel = if open { Panel::Add } else if editor.selected.is_some() { Panel::Slot } else { Panel::Closed };
                }
                if open && self.songs.is_none() && !self.reading {
                    self.reading = true;
                    effects.push(Effect::ReadSongs);
                }
                let place = self.default_place();
                if let Some(Finding::Found(candidate)) = &mut self.finding { candidate.place = place; }
            }
            Message::Source(source) => {
                if source == SourceTab::Suggest {
                    if let Some(slot) = self.editing().and_then(|pool| match &self.screen {
                        Screen::Editor(editor) => editor.selected.filter(|slot| pool.slots.get(*slot).is_some_and(Slot::is_empty)).or_else(|| pool.first_empty()),
                        _ => None,
                    }) {
                        effects.extend(self.suggest_slot(slot));
                    }
                } else {
                    self.cancel_suggestion();
                }
                if let Some(editor) = self.editor_mut() {
                    editor.panel = Panel::Add;
                    editor.source = source;
                }
                if let Some(Finding::Fetching(fetching)) = &self.finding {
                    fetching.stop.store(true, std::sync::atomic::Ordering::SeqCst);
                }
                self.finding = None;
                self.notice = None;
                if source == SourceTab::Collections && !self.collecting {
                    self.collecting = true;
                    effects.push(Effect::ReadCollections);
                }
                if source == SourceTab::Best && self.best.is_none() {
                    effects.push(Effect::ReadBest(false));
                }
            }
            Message::RefreshBest => effects.push(Effect::ReadBest(true)),
            Message::Best(at) => {
                if matches!(self.finding, Some(Finding::Fetching(_))) {
                    return effects;
                }
                let Some(score) = self.best.as_ref().and_then(|scores| scores.get(at)).cloned() else { return effects };
                let Some(target) = score_target(&score) else { return effects };
                let hash = score.hash.to_ascii_lowercase();
                if self.songs.as_ref().is_some_and(|songs| songs.contains_key(&hash)) {
                    self.finding = None;
                    effects.extend(self.put(&hash, now));
                } else {
                    let query = match &target {
                        Target::Hash(hash) => hash.clone(),
                        Target::Beatmap { id, .. } => format!("https://osu.ppy.sh/beatmaps/{id}"),
                        Target::Set { .. } => unreachable!(),
                    };
                    if let Some(editor) = self.editor_mut() {
                        editor.query = query;
                    }
                    self.notice = None;
                    self.finding = Some(Finding::Asking);
                    effects.push(self.resolve(target));
                }
            }
            Message::Collection(collection) => {
                if let Some(editor) = self.editor_mut() {
                    editor.collection = collection;
                    editor.collection_page = 0;
                }
                if let Some(Finding::Fetching(fetching)) = &self.finding {
                    fetching.stop.store(true, std::sync::atomic::Ordering::SeqCst);
                }
                self.finding = None;
                let whole = matches!(&self.screen, Screen::Editor(editor) if editor.wholesale);
                if let (true, Some(at)) = (whole, collection) {
                    if let Some(editor) = self.editor_mut() { editor.wholesale = false; editor.last_mod = Mod::Nm; }
                    let held: HashSet<String> = self.editing().map(|pool| pool.slots.iter().filter_map(|slot| slot.hash.clone()).collect()).unwrap_or_default();
                    let mut seen = HashSet::new();
                    let queue: Vec<String> = self.collections.as_ref().and_then(|all| all.get(at)).map(|chosen| chosen.hashes.iter().filter(|hash| !held.contains(*hash) && seen.insert((*hash).clone())).take(WHOLESALE_MOST).cloned().collect()).unwrap_or_default();
                    if !queue.is_empty() {
                        let total = queue.len();
                        self.finding = Some(Finding::Fetching(Fetching { queue, total, step: None, place: None, stop: Arc::new(std::sync::atomic::AtomicBool::new(false)) }));
                        effects.extend(self.advance(now));
                    }
                }
            }
            Message::CollectionPage(page) => {
                if let Some(editor) = self.editor_mut() {
                    editor.collection_page = page;
                }
            }
            Message::CollectionHash(hash) => {
                if self.songs.as_ref().is_some_and(|songs| songs.contains_key(&hash)) {
                    effects.extend(self.put(&hash, now));
                } else if pool_links::parse(&hash).is_ok() {
                    self.finding = Some(Finding::Asking);
                    effects.push(self.resolve(Target::Hash(hash)));
                }
            }
            Message::Collections(collections) => {
                self.collections = Some(collections);
                self.collecting = false;
            }
            Message::Suggest(slot) => effects.extend(self.suggest_slot(slot)),
            Message::Offered(request, found) => {
                if let Some(suggestion) = self.suggestions.as_mut().filter(|suggestion| suggestion.request == request) {
                    suggestion.offered = Some(found);
                }
            }
            Message::TakeOffered(id) => {
                effects.extend(self.look_at(&format!("https://osu.ppy.sh/b/{id}")));
            }
            Message::Suggested(request, maps) => {
                let current = self.suggestions.as_ref().filter(|suggestion| suggestion.request == request).map(|suggestion| (suggestion.pool.clone(), suggestion.fingerprint.clone(), suggestion.slot, suggestion.mods));
                if let Some((id, fingerprint, slot, mods)) = current {
                    let valid = self.editing().is_some_and(|pool| pool.id == id && pool.fingerprint() == fingerprint && pool.slots.get(slot).is_some_and(|entry| entry.is_empty() && entry.mods == mods));
                    if valid {
                        for (hash, measure) in &maps {
                            self.measures.put(hash, mods, Ok(*measure));
                        }
                        let lean = match &self.screen { Screen::Editor(editor) => editor.lean, _ => None };
                        if let Some(suggestion) = &mut self.suggestions {
                            suggestion.maps = Some(pools::leaning(maps, lean));
                        }
                    }
                }
            }
            Message::Query(query) => {
                self.notice = None;
                let text = query.trim().to_owned();
                if let Some(editor) = self.editor_mut() {
                    editor.query = query;
                }
                effects.extend(self.look_at(&text));
            }
            _ => unreachable!("message routed to the wrong pool handler"),
        }
        effects.extend(self.import_next());
        effects
    }
}
