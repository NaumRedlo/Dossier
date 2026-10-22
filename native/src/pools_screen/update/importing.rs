use super::*;

impl State {
    pub(super) fn handle_importing(&mut self, message: Message, now: i64) -> Vec<Effect> {
        let mut effects = Vec::new();
        match message {
            Message::PasteInto(input, contents) => {
                if let Some(pool) = self.editing() {
                    effects.push(Effect::ReadPaste(pool.id.clone(), input, contents));
                }
            }
            Message::PastedInto(id, input, contents, pasted) => {
                if self.editing().is_some_and(|pool| pool.id == id) {
                    let message = if is_paste(&pasted) || (matches!(input, Input::Query) && !links_in(&pasted).is_empty()) {
                        Message::Pasted(pasted)
                    } else {
                        match input {
                            Input::Name => Message::Rename(contents),
                            Input::Query => Message::Query(contents),
                            Input::Note(slot) => Message::Note(slot, contents),
                        }
                    };
                    effects.extend(self.update(message, now));
                }
            }
            Message::Pasted(text) if pool_share::is_text(&text) => {
                match pool_share::from_text(&text, now) {
                    Ok(pool) => effects.extend(self.begin_open(pool)),
                    Err(refused) => self.refused = Some(refused),
                }
            }
            Message::Pasted(text) => {
                self.stray = profile_link(&text).filter(|_| matches!(self.screen, Screen::Editor(_)));
                if self.stray.is_some() {
                    return effects;
                }
                let mut rest = links_in(&text);
                let text = if rest.is_empty() { text.trim().to_owned() } else { rest.remove(0) };
                let linkish = match pool_links::parse(&text) {
                    Ok(_) | Err(pool_links::Refusal::Mode(_)) => true,
                    Err(pool_links::Refusal::NotALink) => false,
                };
                if self.editing_at().is_none() {
                    if !linkish {
                        return effects;
                    }
                    self.update(Message::New, now);
                }
                if text.is_empty() {
                    return effects;
                }
                self.cancel_suggestion();
                self.notice = None;
                if let Some(editor) = self.editor_mut() {
                    editor.panel = Panel::Add;
                    editor.source = SourceTab::Search;
                    editor.replace = false;
                    editor.query = text.clone();
                }
                effects.extend(self.update(Message::AddPanel(true), now));
                if !rest.is_empty() {
                    self.links = rest.into();
                    self.linking = true;
                }
                if self.songs.is_none() && !self.reading {
                    self.reading = true;
                    effects.push(Effect::ReadSongs);
                }
                effects.extend(self.look_at(&text));
            }
            Message::Resolved(request, said) => {
                if request == self.resolve_request && matches!(self.finding, Some(Finding::Asking)) {
                    match said {
                        Ok(found) => {
                            for score in self.best.iter_mut().flatten() {
                                if let Some(difficulty) = found.difficulties.iter().find(|difficulty| score_id(score) == Some(difficulty.id)) {
                                    score.hash = difficulty.hash.clone();
                                }
                            }
                            let choice = found.picked.or_else(|| (found.difficulties.len() == 1).then_some(0));
                            let place = self.default_place();
                            effects.push(Effect::Cover(found.cover(), found.set));
                            self.finding = Some(Finding::Found(Candidate { found, choice, place, cover: None }));
                            if self.linking && choice.is_some() {
                                effects.extend(self.confirm(false, now));
                            }
                        }
                        Err(Why::Nowhere) => self.finding = Some(Finding::Missing),
                        Err(Why::Mode(mode)) => self.finding = Some(Finding::Refused(mode)),
                        Err(Why::Silent(why)) => self.finding = Some(Finding::Silent(why)),
                    }
                }
            }
            Message::Cover(set, handle) => {
                if let Some(Finding::Found(candidate)) = &mut self.finding {
                    if candidate.found.set == set {
                        candidate.cover = handle;
                    }
                }
            }
            Message::Choose(at) => {
                if let Some(Finding::Found(candidate)) = &mut self.finding {
                    if at < candidate.found.difficulties.len() {
                        candidate.choice = Some(at);
                    }
                }
            }
            Message::Aim(place) => {
                if let Some(Finding::Found(candidate)) = &mut self.finding {
                    candidate.place = place;
                }
            }
            Message::Confirm => effects.extend(self.confirm(false, now)),
            Message::ConfirmAll => effects.extend(self.confirm(true, now)),
            Message::Dismiss => {
                self.cancel_network();
                self.notice = None;
                effects.extend(self.import_next());
            }
            Message::FetchSlot(at) => {
                let wanted = self.editing().and_then(|pool| pool.slots.get(at)).and_then(|slot| slot.hash.clone());
                if let (Some(hash), false) = (wanted, matches!(self.finding, Some(Finding::Fetching(_)))) {
                    self.fetched.remove(&hash);
                    self.finding = Some(Finding::Fetching(Fetching { queue: vec![hash], total: 1, step: None, place: Some(Place::Slot(at)), stop: Arc::new(std::sync::atomic::AtomicBool::new(false)) }));
                    effects.extend(self.advance(now));
                }
            }
            Message::Retry => {
                let text = match &self.screen {
                    Screen::Editor(editor) => editor.query.trim().to_owned(),
                    Screen::Shelf | Screen::Open(_) => String::new(),
                };
                self.finding = None;
                effects.extend(self.look_at(&text));
            }
            Message::Step(request, hash, step) => {
                if self.fetching.as_ref() == Some(&(request, hash.clone())) {
                    if matches!(step, crate::maps::Step::Done(_) | crate::maps::Step::Nowhere | crate::maps::Step::Failed(_) | crate::maps::Step::Stopped) {
                        self.fetching = None;
                    }
                    effects.extend(self.stepped(&hash, step, now));
                    if self.fetching.is_none() {
                        effects.extend(self.fetch_next(now));
                    }
                }
            }
            Message::Imported(said) => match said {
                Ok(pool) => effects.extend(self.begin_open(pool)),
                Err(refused) => self.refused = Some(refused),
            },
            Message::Dropped(path) => {
                if path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("pool")) {
                    if let Some(Finding::Fetching(fetching)) = &self.finding {
                        fetching.stop.store(true, Ordering::SeqCst);
                    }
                    self.finding = None;
                    self.update(Message::Back, now);
                    self.file_request = self.file_request.wrapping_add(1);
                    self.importing_pool = Some(self.file_request);
                    effects.push(Effect::ReadPool(self.file_request, path));
                } else if path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("osu") || ext.eq_ignore_ascii_case("osz")) {
                    if matches!(self.screen, Screen::Open(_)) {
                        effects.push(Effect::Say("pool-file-open-first"));
                        return effects;
                    }
                    if self.editing().is_none() {
                        self.update(Message::New, now);
                    }
                    self.dropped.push_back(path);
                    effects.extend(self.import_next());
                }
            }
            Message::MapFile(request, said) => {
                let Some(importing) = self.importing.as_ref().filter(|importing| importing.request == request).cloned() else { return effects };
                self.importing = None;
                if !self.editing().is_some_and(|pool| pool.id == importing.pool && pool.fingerprint() == importing.fingerprint) {
                    self.cancel_import();
                    return effects;
                }
                match said {
                    Ok((hash, map)) => {
                        self.fetched.insert(hash.clone(), map.clone());
                        Arc::make_mut(self.songs.get_or_insert_with(|| Arc::new(HashMap::new()))).insert(hash.clone(), map.clone());
                        if let Some(Finding::Fetching(fetching)) = &self.finding {
                            fetching.stop.store(true, Ordering::SeqCst);
                        }
                        self.finding = None;
                        self.notice = None;
                        effects.extend(self.place_map(&hash, map, Some(importing.place), now));
                    }
                    Err(why) => effects.push(Effect::Say(match why {
                        crate::pool_files::Refused::Mode => "pool-file-mode",
                        crate::pool_files::Refused::Save => "pool-file-save-failed",
                        crate::pool_files::Refused::Read | crate::pool_files::Refused::Map => "pool-file-invalid",
                    })),
                }
                effects.extend(self.import_next());
            }
            Message::ArchiveFile(request, said) => {
                let Some(importing) = self.importing.as_ref().filter(|importing| importing.request == request).cloned() else { return effects };
                self.importing = None;
                if !self.editing().is_some_and(|pool| pool.id == importing.pool && pool.fingerprint() == importing.fingerprint) {
                    self.cancel_import();
                    return effects;
                }
                match said {
                    Ok(maps) if !maps.is_empty() => {
                        self.cancel_network();
                        let first = &maps[0].1;
                        let found = pool_links::Found {
                            set: 0,
                            artist: first.artist.clone(),
                            title: first.title.clone(),
                            picked: (maps.len() == 1).then_some(0),
                            difficulties: maps.iter().map(|(hash, map)| pool_links::Difficulty { id: 0, hash: hash.clone(), version: map.version.clone(), stars: 0.0 }).collect(),
                        };
                        for (hash, map) in maps {
                            self.fetched.insert(hash.clone(), map.clone());
                            Arc::make_mut(self.songs.get_or_insert_with(|| Arc::new(HashMap::new()))).insert(hash, map);
                        }
                        let choice = found.picked;
                        self.finding = Some(Finding::Found(Candidate { found, choice, place: importing.place, cover: None }));
                        if let Some(editor) = self.editor_mut() { editor.panel = Panel::Add; editor.source = SourceTab::Search; }
                        effects.extend(self.measure_effects());
                        if let Some(Finding::Found(candidate)) = &self.finding {
                            for difficulty in &candidate.found.difficulties {
                                if self.asked.insert((difficulty.hash.clone(), Mod::Nm)) {
                                    let map = self.songs.as_ref().unwrap()[&difficulty.hash].clone();
                                    effects.push(Effect::Measure(difficulty.hash.clone(), map, Mod::Nm));
                                }
                            }
                        }
                    }
                    Err(why) => {
                        effects.push(Effect::Say(match why {
                            crate::pool_files::Refused::Mode => "pool-file-mode",
                            crate::pool_files::Refused::Save => "pool-file-save-failed",
                            crate::pool_files::Refused::Read | crate::pool_files::Refused::Map => "pool-file-invalid",
                        }));
                        effects.extend(self.import_next());
                    }
                    Ok(_) => {
                        effects.push(Effect::Say("pool-file-invalid"));
                        effects.extend(self.import_next());
                    }
                }
            }
            Message::PoolFile(request, said) => {
                if self.importing_pool == Some(request) {
                    self.importing_pool = None;
                    effects.extend(self.update(Message::Imported(said), now));
                }
            }
            Message::GetMissing => effects.extend(self.get_missing(now)),
            Message::Keep => effects.extend(self.keep_opened(now)),
            Message::FetchMissing => {
                if let Some(editor) = self.editor_mut() {
                    editor.fetch_all = true;
                    editor.fetch_tried.clear();
                }
                effects.extend(self.fetch_next(now));
            }
            Message::Songs(songs) => {
                let mut songs = songs;
                if !self.fetched.is_empty() {
                    let merged = Arc::make_mut(&mut songs);
                    for (hash, map) in &self.fetched {
                        merged.entry(hash.clone()).or_insert_with(|| map.clone());
                    }
                }
                self.songs = Some(songs);
                effects.extend(self.backdrop_effect());
                for pool in &mut self.list {
                    if pool.collection {
                        for slot in &mut pool.slots {
                            if let Some(hash) = slot.hash.clone() {
                                if let Some(map) = self.songs.as_ref().and_then(|songs| songs.get(&hash)) { slot.fill(&hash, map); }
                            }
                        }
                    }
                }
                self.reading = false;
                effects.extend(self.measure_effects());
                if let Some(slot) = self.pending_suggestion.take() {
                    effects.extend(self.suggest_slot(slot));
                }
            }
            Message::Measured(hash, mods, said) => {
                self.measures.put(&hash, mods, said);
                self.apply_measures();
                if let Some(slot) = self.suggestions.as_ref().filter(|suggestion| suggestion.target.is_none()).map(|suggestion| suggestion.slot) {
                    effects.extend(self.suggest_slot(slot));
                }
            }
            _ => unreachable!("message routed to the wrong pool handler"),
        }
        effects.extend(self.import_next());
        effects
    }
}
