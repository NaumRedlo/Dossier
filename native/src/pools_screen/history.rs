use super::*;

impl State {
    pub(super) fn step_history(&mut self, back: bool, now: i64) -> Vec<Effect> {
        let mut effects = Vec::new();
                let current = match &self.screen {
                    Screen::Editor(editor) => Some(editor.id.clone()),
                    _ => None,
                };
                if let Some(id) = current {
                    let stack = if back { &mut self.undo } else { &mut self.redo };
                    if let Some(from) = stack.iter().rposition(|(pool, _)| *pool == id) {
                        let (_, before) = stack.remove(from);
                        if let Some(now_at) = self.list.iter().position(|pool| pool.id == id) {
                            let held = (id.clone(), self.list[now_at].clone());
                            if back { self.redo.push(held); } else { self.undo.push(held); }
                        }
                        if let Some(at) = self.list.iter().position(|pool| pool.id == id) {
                            let current_keys = slot_keys(&self.list[at]);
                            let restored_keys = slot_keys(&before);
                            if current_keys.len() == restored_keys.len() && restored_keys.iter().all(|key| current_keys.contains(key)) {
                                let order: Vec<usize> = restored_keys.iter().map(|key| current_keys.iter().position(|old| old == key).unwrap()).collect();
                                self.follow_order(&order);
                                if let Some(importing) = self.importing.as_mut().filter(|importing| importing.pool == id && importing.fingerprint == self.list[at].fingerprint()) {
                                    importing.fingerprint = before.fingerprint();
                                }
                            }
                            self.list[at] = before;
                            let length = self.list[at].slots.len();
                            self.save(at, now);
                            if let Some(editor) = self.editor_mut() {
                                editor.marked.retain(|slot| *slot < length);
                                editor.selected = editor.selected.filter(|slot| *slot < length);
                                if editor.selected.is_none() && editor.panel == Panel::Slot {
                                    editor.panel = Panel::Closed;
                                }
                            }
                            effects.extend(self.measure_effects());
                        }
                    }
                }
            
        effects
    }

    pub(super) fn remember(&mut self, at: usize) {
        if let Some(pool) = self.list.get(at) {
            let id = pool.id.clone();
            self.redo.retain(|(held, _)| *held != id);
            self.undo.push((pool.id.clone(), pool.clone()));
            if self.undo.len() > 40 {
                self.undo.remove(0);
            }
        }
    }

    pub(super) fn marked_slots(&self) -> Vec<usize> {
        match &self.screen {
            Screen::Editor(editor) => {
                let mut marked = editor.marked.clone();
                marked.sort_unstable();
                marked.dedup();
                marked
            }
            _ => Vec::new(),
        }
    }

    pub(super) fn reorder_slots(&mut self, at: usize, order: &[usize], now: i64) {
        let fingerprint = self.list[at].fingerprint();
        let slots = self.list[at].slots.clone();
        self.list[at].slots = order.iter().map(|slot| slots[*slot].clone()).collect();
        self.follow_order(order);
        self.cancel_suggestion();
        if let Some(importing) = self.importing.as_mut().filter(|importing| importing.pool == self.list[at].id && importing.fingerprint == fingerprint) {
            importing.fingerprint = self.list[at].fingerprint();
        }
        self.save(at, now);
    }

    pub(super) fn follow_order(&mut self, order: &[usize]) {
        let position = |old: usize| order.iter().position(|slot| *slot == old);
        let place = |place: &mut Place| {
            match *place {
                Place::Slot(old) => { if let Some(at) = position(old) { *place = Place::Slot(at); } }
                Place::Replace(old) => { if let Some(at) = position(old) { *place = Place::Replace(at); } }
                Place::End => {}
            }
        };
        if let Some(editor) = self.editor_mut() {
            editor.selected = editor.selected.and_then(position);
            editor.marked = editor.marked.iter().filter_map(|at| position(*at)).collect();
            editor.marked.sort_unstable();
        }
        match &mut self.finding {
            Some(Finding::Found(candidate)) => place(&mut candidate.place),
            Some(Finding::Fetching(fetching)) => { if let Some(target) = &mut fetching.place { place(target); } }
            _ => {}
        }
        if let Some(importing) = &mut self.importing {
            place(&mut importing.place);
        }
        if let Some(Notice::Already(old)) = &mut self.notice {
            if let Some(at) = position(*old) { *old = at; }
        }
    }
}
