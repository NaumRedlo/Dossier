use super::*;

impl State {
    pub(super) fn handle_navigation(&mut self, message: Message, now: i64) -> Vec<Effect> {
        let mut effects = Vec::new();
        match message {
            Message::FlushSaves => { self.flush_saves(false); }
            Message::RetrySaves => { self.flush_saves(true); }
            Message::Begin(way) => {
                if let Some(editor) = self.editor_mut() { editor.starting = false; }
                if let Some(at) = self.editing_at() {
                    if self.list[at].frame == Frame::Free && self.list[at].slots.is_empty() {
                        self.list[at].slots = vec![Slot::empty(Mod::Nm); 4];
                        self.save(at, now);
                    }
                }
                match way {
                    Start::Empty => {}
                    Start::Links => {
                        effects.extend(self.update(Message::AddPanel(true), now));
                        if let Some(pool) = self.editing() { effects.push(Effect::ReadPaste(pool.id.clone(), Input::Query, String::new())); }
                    }
                    Start::Collections => {
                        effects.extend(self.update(Message::AddPanel(true), now));
                        effects.extend(self.update(Message::Source(SourceTab::Collections), now));
                        if let Some(editor) = self.editor_mut() { editor.wholesale = true; }
                    }
                    Start::Best => {
                        if let Some(editor) = self.editor_mut() { editor.spread = true; }
                        if self.best.is_none() {
                            effects.push(Effect::ReadBest(false));
                        }
                        effects.extend(self.spread_best(now));
                    }
                }
            }
            Message::HideGuide => self.guide.on = false,
            Message::GuideNext => {
                match self.editing().and_then(|pool| self.guide.current(pool)) {
                    Some(step) => self.guide.at = step as u8 + 1,
                    None => self.guide.on = false,
                }
            }
            Message::ImportCollection(at) => {
                if let Some(collection) = self.collections.as_ref().and_then(|collections| collections.get(at)) {
                    let name = collection.name.clone();
                    let pool = collection_pool(collection, self.songs.as_deref(), now);
                    let id = pool.id.clone(); self.list.push(pool); self.save(self.list.len() - 1, now);
                    effects.extend(self.update(Message::Open(id), now));
                    if let Some(editor) = self.editor_mut() {
                        editor.game_collection = Some(name);
                    }
                }
            }
            Message::New => {
                self.makers = None;
                self.cancel_network();
                self.cancel_suggestion();
                self.cancel_import();
                let mut pool = Pool::new(Frame::Free, "", now);
                pool.collection = self.collection_shelf;
                pool.slots = vec![Slot::empty(Mod::Nm); 4];
                let id = pool.id.clone();
                self.list.insert(0, pool);
                self.save(0, now);
                effects.push(Effect::OwnCompiler(id.clone()));
                let starting = !self.collection_shelf;
                self.screen = Screen::Editor(Editor { untouched: true, starting, ..Editor::at(id) });
            }
            Message::Open(id) => {
                self.makers = None;
                if self.list.iter().any(|pool| pool.id == id) {
                    self.cancel_network();
                    self.cancel_suggestion();
                    self.cancel_import();
                    self.screen = Screen::Editor(Editor::at(id.clone()));
                    self.versions = pools::versions(&self.dir, &id);
                    let authors = self.editing().map(|pool| pool.authors.iter().map(|name| if name.contains(' ') { format!("\"{name}\"") } else { name.clone() }).collect::<Vec<_>>().join(" ")).unwrap_or_default();
                    if let Some(editor) = self.editor_mut() { editor.editing_authors = false; editor.authors = authors; }
                    effects.push(Effect::OwnCompiler(id.clone()));
                    if let Some(pool) = self.editing() { effects.push(Effect::Faces(pool.authors.iter().cloned().chain(std::iter::once(pool.compiler.clone())).filter(|name| !name.is_empty()).collect())); }
                    if self.songs.is_none() && !self.reading {
                        self.reading = true;
                        effects.push(Effect::ReadSongs);
                    }
                }
            }
            Message::Back => {
                if !self.flush_saves(true) { return effects; }
                self.cancel_network();
                self.cancel_import();
                self.cancel_suggestion();
                if let (Some(at), Screen::Editor(editor)) = (self.editing_at(), self.screen.clone()) {
                    if editor.untouched && self.list[at].filled() == 0 && self.list[at].name.is_empty() {
                        let id = self.list.remove(at).id;
                        let _ = pools::remove(&self.dir, &id);
                    }
                }
                if let Screen::Open(opening) = &self.screen {
                    opening.stop.store(true, std::sync::atomic::Ordering::SeqCst);
                }
                self.screen = Screen::Shelf;
                self.notice = None;
                self.refused = None;
            }
            Message::UseFrame(frame) => {
                if let Some(at) = self.editing_at() {
                    if self.list[at].filled() == 0 {
                        self.list[at].frame = frame;
                        self.list[at].slots = frame.mods().into_iter().map(Slot::empty).collect();
                        if let Some(editor) = self.editor_mut() {
                            editor.selected = None;
                        }
                        self.save(at, now);
                    }
                }
            }
            Message::GuideGot => {
                if self.guide.on {
                    self.guide.modded = true;
                }
            }
            Message::DropStray => self.stray = None,
            Message::Hot(Hot::Cancel) => {
                if let Some(next) = self.cancel_next() {
                    effects.extend(self.update(next, now));
                }
            }
            Message::Hot(Hot::Confirm) => {
                if let Some(next) = self.confirm_next() {
                    effects.extend(self.update(next, now));
                }
            }
            Message::Hot(hot) => {
                let target = match &self.screen {
                    Screen::Editor(editor) if !editor.starting && !editor.choosing => editor.menu.or(editor.selected),
                    _ => None,
                };
                let filled = target.filter(|at| self.editing().and_then(|pool| pool.slots.get(*at)).is_some_and(|slot| !slot.is_empty()));
                let next = match (hot, filled) {
                    (Hot::Text, _) => matches!(self.screen, Screen::Editor(_)).then_some(Message::CopyText),
                    (Hot::Add, _) => matches!(&self.screen, Screen::Editor(editor) if !editor.starting).then_some(Message::AddPanel(true)),
                    (Hot::Note, Some(at)) => Some(Message::Select(Some(at))),
                    (Hot::Replace, Some(at)) => Some(Message::ReplaceAt(at)),
                    (Hot::Double, Some(at)) => Some(Message::Duplicate(at)),
                    (Hot::Copy, Some(at)) => Some(Message::CopySlot(at)),
                    (Hot::Remove, Some(at)) => Some(Message::Clear(at)),
                    (Hot::Remove, None) => target.map(Message::Clear),
                    _ => None,
                };
                if let Some(next) = next {
                    effects.extend(self.update(Message::Chosen(Box::new(next)), now));
                }
            }
            Message::AskDelete(ask) => {
                self.uncollect_refused = false;
                self.delete_error = None;
                if let Some(editor) = self.editor_mut() {
                    editor.asking_delete = ask;
                }
            }
            Message::DeletePool => {
                self.delete_error = None;
                self.uncollect_refused = false;
                if let Some(at) = self.editing_at() {
                    let pool = &self.list[at];
                    let game_name = match &self.screen {
                        Screen::Editor(editor) => editor.game_collection.clone(),
                        _ => None,
                    };
                    if let Some(name) = game_name.or_else(|| pool.saved.as_ref().filter(|saved| saved.collected).map(|_| pool.name.clone())) {
                        effects.push(Effect::Uncollect(pool.id.clone(), name));
                    } else {
                        self.drop_pool(at);
                    }
                }
            }
            Message::Uncollected(id, result) => {
                if let Some(at) = self.list.iter().position(|pool| pool.id == id) {
                    match result {
                        Err(code) if code == "running" => self.uncollect_refused = true,
                        Err(why) => self.delete_error = Some(why),
                        Ok(_) => {
                            self.drop_pool(at);
                            effects.push(Effect::ReadCollections);
                        }
                    }
                }
            }
            _ => unreachable!("message routed to the wrong pool handler"),
        }
        effects.extend(self.import_next());
        effects
    }
}
