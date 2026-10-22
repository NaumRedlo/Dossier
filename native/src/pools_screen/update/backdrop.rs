use super::*;

impl State {
    pub(super) fn handle_backdrop(&mut self, message: Message, now: i64) -> Vec<Effect> {
        let mut effects = Vec::new();
        match message {
            Message::BackdropPanel(open) => {
                if let Some(editor) = self.editor_mut() {
                    editor.backdrop = open;
                    editor.palette = false;
                    editor.menu = None;
                }
                self.backdrop_refused = false;
            }
            Message::BackdropFrom(hash) => {
                if let Some(at) = self.editing_at() {
                    let held = self.list[at].backdrop.clone();
                    self.list[at].backdrop = hash.map(|hash| match held {
                        Some(held) => pools::Backdrop { from: pools::BackdropFrom::Map(hash), ..held },
                        None => pools::Backdrop::of(pools::BackdropFrom::Map(hash)),
                    });
                    self.backdrop_refused = false;
                    self.save(at, now);
                    effects.extend(self.backdrop_effect());
                }
            }
            Message::BackdropPick => effects.push(Effect::PickBackdrop),
            Message::BackdropPicked(picked) => {
                if let (Some(at), Some(path)) = (self.editing_at(), picked) {
                    let fits = path.metadata().is_ok_and(|about| about.is_file() && about.len() <= pools::Backdrop::MOST_BYTES)
                        && path.extension().is_some_and(|ext| ["jpg", "jpeg", "png"].iter().any(|known| ext.eq_ignore_ascii_case(known)));
                    let kept = fits.then(|| {
                        let folder = self.dir.join("backdrops");
                        let target = folder.join(format!("{}.{}", self.list[at].id, path.extension().map(|ext| ext.to_string_lossy().to_lowercase()).unwrap_or_default()));
                        std::fs::create_dir_all(&folder).and_then(|_| std::fs::copy(&path, &target)).ok().map(|_| target)
                    }).flatten();
                    self.backdrop_refused = kept.is_none();
                    if let Some(target) = kept {
                        let held = self.list[at].backdrop.clone();
                        self.list[at].backdrop = Some(match held {
                            Some(held) => pools::Backdrop { from: pools::BackdropFrom::File(target), ..held },
                            None => pools::Backdrop::of(pools::BackdropFrom::File(target)),
                        });
                        self.backdrop = None;
                        self.save(at, now);
                        effects.extend(self.backdrop_effect());
                    }
                }
            }
            Message::BackdropDim(dim) => {
                if let Some(at) = self.editing_at() {
                    if let Some(backdrop) = self.list[at].backdrop.as_mut() {
                        backdrop.dim = dim.clamp(pools::Backdrop::LEAST_DIM, pools::Backdrop::MOST_DIM);
                        self.defer_save(at, now);
                    }
                }
            }
            Message::BackdropBlur(on) => {
                if let Some(at) = self.editing_at() {
                    if let Some(backdrop) = self.list[at].backdrop.as_mut() {
                        backdrop.blur = on;
                        self.save(at, now);
                        effects.extend(self.backdrop_effect());
                    }
                }
            }
            Message::BackdropCard(on) => {
                if let Some(at) = self.editing_at() {
                    if let Some(backdrop) = self.list[at].backdrop.as_mut() {
                        backdrop.on_card = on;
                        self.save(at, now);
                    }
                }
            }
            Message::BackdropLoaded(key, picture) => {
                if self.backdrop_key().as_ref().is_some_and(|(wanted, _, _)| *wanted == key) {
                    self.backdrop = picture.map(|picture| (key, picture));
                }
            }
            _ => unreachable!("message routed to the wrong pool handler"),
        }
        effects.extend(self.import_next());
        effects
    }
}
