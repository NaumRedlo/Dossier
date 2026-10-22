use super::*;

impl State {
    pub(super) fn drop_pool(&mut self, at: usize) {
        let id = self.list[at].id.clone();
        if let Err(why) = pools::remove(&self.dir, &id) {
            self.delete_error = Some(why);
            return;
        }
        self.cancel_network();
        self.cancel_import();
        self.cancel_suggestion();
        self.list.remove(at);
        self.delete_error = None;
        self.unsaved.remove(&id);
        self.save_errors.remove(&id);
        self.undo.retain(|(pool, _)| *pool != id);
        self.uncollect_refused = false;
        if matches!(&self.screen, Screen::Editor(editor) if editor.id == id) {
            self.screen = Screen::Shelf;
        }
        self.notice = None;
    }

    pub(super) fn save(&mut self, at: usize, now: i64) {
        if self.suggestions.as_ref().is_some_and(|suggestion| self.list.get(at).is_some_and(|pool| pool.id == suggestion.pool && pool.fingerprint() != suggestion.fingerprint)) {
            self.cancel_suggestion();
        }
        if let Some(pool) = self.list.get_mut(at) {
            pool.changed_at = now;
        }
        self.save_quiet(at);
        self.list.sort_by(|a, b| b.changed_at.cmp(&a.changed_at).then_with(|| a.id.cmp(&b.id)));
    }

    pub(super) fn save_quiet(&mut self, at: usize) {
        if let Some(pool) = self.list.get(at) {
            match pools::save(&self.dir, pool) {
                Ok(()) => {
                    self.unsaved.remove(&pool.id);
                    self.save_errors.remove(&pool.id);
                }
                Err(why) => {
                    self.unsaved.insert(pool.id.clone(), Instant::now() + std::time::Duration::from_secs(5));
                    self.save_errors.insert(pool.id.clone(), why);
                }
            }
        }
    }

    pub(super) fn defer_save(&mut self, at: usize, now: i64) {
        if let Some(pool) = self.list.get_mut(at) {
            pool.changed_at = now;
            self.unsaved.insert(pool.id.clone(), Instant::now() + std::time::Duration::from_millis(400));
        }
    }

    pub fn has_unsaved(&self) -> bool { !self.unsaved.is_empty() }

    pub fn save_failed(&self) -> bool { !self.save_errors.is_empty() }

    pub fn flush_saves(&mut self, force: bool) -> bool {
        let now = Instant::now();
        let due: Vec<_> = self.unsaved.iter().filter(|(_, at)| force || now >= **at).map(|(id, _)| id.clone()).collect();
        for id in due {
            if let Some(at) = self.list.iter().position(|pool| pool.id == id) {
                self.save_quiet(at);
            }
        }
        self.list.sort_by(|a, b| b.changed_at.cmp(&a.changed_at).then_with(|| a.id.cmp(&b.id)));
        !self.has_unsaved()
    }
}
