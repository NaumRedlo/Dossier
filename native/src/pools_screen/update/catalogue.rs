use super::*;

impl State {
    pub(super) fn handle_catalogue(&mut self, message: Message, now: i64) -> Vec<Effect> {
        let mut effects = Vec::new();
        match message {
            Message::Makers(asked) => { self.makers = if asked == self.makers { None } else { asked }; }
            Message::Catalogue(collections) => {
                self.makers = None;
                self.collection_shelf = collections;
                self.screen = Screen::Shelf;
                self.publications.clear();
                self.catalogue_error = None;
                self.catalogue_loading = true;
                effects.push(Effect::Catalogue(collections, 0));
                if collections && self.collections.is_none() && !self.collecting { self.collecting = true; effects.push(Effect::ReadCollections); }
                if collections && self.songs.is_none() && !self.reading { self.reading = true; effects.push(Effect::ReadSongs); }
            }
            Message::Shelf(tab) => {
                self.makers = None;
                self.shelf = tab;
                if self.collection_shelf || self.screen != Screen::Shelf {
                    effects.extend(self.update(Message::Catalogue(false), now));
                }
            }
            Message::SavePublication(id) => {
                let held = self.list.iter().any(|pool| pool.saved.as_ref().is_some_and(|saved| saved.id == id));
                if let Some(publication) = self.publications.iter().find(|row| row.id == id && !row.mine).filter(|_| !held).cloned() {
                    if let Some(mut pool) = published_pool(&publication, self.songs.as_deref(), now) {
                        pool.published_revision = 0;
                        pool.backdrop = pools::held_backdrop(&self.dir, &pool.id, &publication.content);
                        pool.saved = Some(pools::Saved { id: publication.id.clone(), code: publication.code.clone(), revision: publication.revision, publisher: pool.compiler.clone(), collected: false });
                        self.list.push(pool);
                        self.save(self.list.len() - 1, now);
                        effects.extend(self.measure_effects());
                        if self.songs.is_none() && !self.reading { self.reading = true; effects.push(Effect::ReadSongs); }
                    }
                }
            }
            Message::Pull(id) => {
                let busy = self.fetching.is_some() || self.pull.as_ref().is_some_and(Pull::running);
                if let Some(pool) = self.list.iter().find(|pool| pool.id == id && pool.saved.is_some()).filter(|_| !busy && self.songs.is_some()) {
                    let queue = self.missing(pool);
                    self.pull = Some(Pull { pool: id, total: queue.len(), queue, lost: 0, current: None, step: None, stop: Arc::new(AtomicBool::new(false)), over: None });
                    effects.extend(self.pull_next());
                }
            }
            Message::Renew(id) => {
                let busy = self.pull.as_ref().is_some_and(|pull| pull.pool == id && pull.running());
                if let Some(at) = self.list.iter().position(|pool| pool.id == id).filter(|_| !busy) {
                    let fresh = self.newer_of(&self.list[at]).cloned();
                    if let Some((publication, mut pool)) = fresh.and_then(|publication| published_pool(&publication, self.songs.as_deref(), now).map(|pool| (publication, pool))) {
                        pool.id = id.clone();
                        pool.published_revision = 0;
                        pool.made_at = self.list[at].made_at;
                        pool.backdrop = pools::held_backdrop(&self.dir, &pool.id, &publication.content);
                        pool.saved = Some(pools::Saved { id: publication.id.clone(), code: publication.code.clone(), revision: publication.revision, publisher: pool.compiler.clone(), collected: false });
                        self.list[at] = pool;
                        if self.pull.as_ref().is_some_and(|pull| pull.pool == id) { self.pull = None; }
                        self.save(at, now);
                        effects.extend(self.measure_effects());
                    }
                }
            }
            Message::StopPull => {
                if let Some(pull) = self.pull.as_mut().filter(|pull| pull.running()) {
                    pull.stop.store(true, Ordering::SeqCst);
                    pull.queue.clear();
                    if pull.current.is_none() {
                        self.pull = None;
                    }
                }
            }
            Message::Collected(id, result) => {
                if let Some(at) = self.list.iter().position(|pool| pool.id == id) {
                    if result.is_ok() {
                        if let Some(saved) = self.list[at].saved.as_mut() {
                            saved.collected = true;
                        }
                        self.save_quiet(at);
                    }
                }
                if let Some(pull) = self.pull.as_mut().filter(|pull| pull.pool == id) {
                    pull.over = Some(result);
                }
            }
            Message::CatalogueMore => {
                if !self.catalogue_loading { self.catalogue_loading = true; effects.push(Effect::Catalogue(self.collection_shelf, self.publications.len())); }
            }
            Message::CatalogueLoaded(collections, append, result) => {
                if collections == self.collection_shelf {
                    self.catalogue_loading = false;
                    match result {
                        Ok(rows) => {
                            let names: Vec<_> = rows.iter().flat_map(publication_authors).filter(|name| !self.faces.contains_key(&name.to_lowercase())).collect();
                            if !names.is_empty() { effects.push(Effect::Faces(names)); }
                            self.catalogue_more = rows.len() == 100; if !append { self.publications.clear(); } self.publications.extend(rows); self.catalogue_error = None;
                            if let Some((id, keep)) = self.resolving.take() {
                                effects.extend(self.settle_conflict(&id, keep, now));
                            }
                        }
                        Err(why) => {
                            self.catalogue_error = Some(why);
                            self.resolving = None;
                        }
                    }
                }
            }
            Message::OpenPublication(id) => {
                self.makers = None;
                if self.publications.iter().any(|row| row.id == id && !row.mine) {
                    let held = |list: &[Pool]| list.iter().find(|pool| pool.saved.as_ref().is_some_and(|saved| saved.id == id)).map(|pool| pool.id.clone());
                    if held(&self.list).is_none() {
                        effects.extend(self.update(Message::SavePublication(id.clone()), now));
                    }
                    if let Some(local) = held(&self.list) {
                        effects.extend(self.update(Message::Open(local), now));
                    }
                    return effects;
                }
                if let Some(publication) = self.publications.iter().find(|row| row.id == id).cloned() {
                    let parsed = if publication.kind == "collection" {
                        let collection = Collection { name: publication.name.clone(), hashes: publication.content["hashes"].as_array().into_iter().flatten().filter_map(|h| h.as_str().map(str::to_owned)).collect() };
                        Some(collection_pool(&collection, self.songs.as_deref(), now))
                    } else { serde_json::to_vec(&publication.content).ok().and_then(|bytes| pool_share::from_file(&bytes, now).ok()) };
                    if let Some(mut pool) = parsed {
                        if publication.mine { pool.id = publication.local_id; pool.published_revision = publication.revision; }
                        pool.backdrop = self.list.iter().find(|item| item.id == pool.id).and_then(|item| item.backdrop.clone()).or_else(|| pools::held_backdrop(&self.dir, &pool.id, &publication.content));
                        if let Some(at) = self.list.iter().position(|item| item.id == pool.id && item.published_revision != pool.published_revision) {
                            let mut draft = self.list[at].clone();
                            draft.id = Pool::new(Frame::Free, "", now).id;
                            draft.published_revision = 0;
                            self.list[at] = pool.clone();
                            self.save(at, now);
                            self.list.push(draft);
                            self.save(self.list.len() - 1, now);
                        }
                        if let Some(existing) = self.list.iter().find(|item| item.id == pool.id) { effects.extend(self.update(Message::Open(existing.id.clone()), now)); }
                        else { let id = pool.id.clone(); self.list.push(pool); self.save(self.list.len() - 1, now); effects.extend(self.update(Message::Open(id), now)); }
                    }
                }
            }
            Message::RetryCatalogue => {
                self.catalogue_error = None;
                if !self.catalogue_loading {
                    self.catalogue_loading = true;
                    effects.push(Effect::Catalogue(self.collection_shelf, 0));
                }
            }
            Message::History(open) => {
                if let Some(editor) = self.editor_mut() {
                    editor.history = open;
                    editor.palette = open;
                }
            }
            Message::OpenVersion(revision) => {
                let id = self.editing().map(|pool| pool.id.clone());
                if let Some(mut copy) = id.and_then(|id| self.versions.iter().find(|held| held.id == id && held.published_revision == revision).cloned()) {
                    copy.id = Pool::new(Frame::Free, "", now).id;
                    copy.name = format!("{} v{revision}", copy.name);
                    copy.published_revision = 0;
                    copy.saved = None;
                    let fresh = copy.id.clone();
                    self.list.push(copy);
                    self.save(self.list.len() - 1, now);
                    effects.extend(self.update(Message::Open(fresh), now));
                }
            }
            _ => unreachable!("message routed to the wrong pool handler"),
        }
        effects.extend(self.import_next());
        effects
    }
}
