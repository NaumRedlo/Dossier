use super::*;

impl State {
    pub(super) fn handle_editing(&mut self, message: Message, now: i64) -> Vec<Effect> {
        let mut effects = Vec::new();
        match message {
            Message::ShowSlot(at) => {
                effects.extend(self.update(Message::PublishSheet(false), now));
                effects.extend(self.update(Message::Select(Some(at)), now));
            }
            Message::Select(Some(at)) | Message::Mark(at) if self.shift && matches!(&self.screen, Screen::Editor(editor) if !editor.starting) => {
                if let Some(editor) = self.editor_mut() {
                    let from = editor.marked.last().copied().or(editor.selected).unwrap_or(at);
                    editor.choosing = true;
                    editor.panel = Panel::Closed;
                    editor.selected = None;
                    editor.bulk = None;
                    for slot in from.min(at)..=from.max(at) {
                        if !editor.marked.contains(&slot) {
                            editor.marked.push(slot);
                        }
                    }
                }
            }
            Message::Select(slot) => {
                self.cancel_suggestion();
                self.notice = None;
                let empty = slot.is_some_and(|at| self.editing().and_then(|pool| pool.slots.get(at)).is_some_and(Slot::is_empty));
                let mods = slot.and_then(|at| self.editing()?.slots.get(at).map(|slot| slot.mods));
                let category = slot.and_then(|at| self.editing()?.slots.get(at).map(|slot| (slot.category.clone(), slot.colour, colour_hex(slot_colour(slot)))));
                if let Some(editor) = self.editor_mut() {
                    if let Some(mods) = mods { editor.last_mod = mods; }
                    if let Some((name, colour, hex)) = category { editor.last_category = name; editor.last_colour = colour; editor.colour_draft = hex; }
                    editor.selected = slot;
                    editor.replace = false;
                    editor.panel = if empty { Panel::Add } else if slot.is_some() { Panel::Slot } else { Panel::Closed };
                }
                if empty && self.songs.is_none() && !self.reading {
                    self.reading = true;
                    effects.push(Effect::ReadSongs);
                }
                let place = self.default_place();
                if let Some(Finding::Found(candidate)) = &mut self.finding { candidate.place = place; }
            }
            Message::Replace => {
                self.notice = None;
                if let Some(editor) = self.editor_mut() {
                    editor.replace = editor.selected.is_some();
                    editor.panel = Panel::Add;
                }
                if self.songs.is_none() && !self.reading {
                    self.reading = true;
                    effects.push(Effect::ReadSongs);
                }
                let place = self.default_place();
                if let Some(Finding::Found(candidate)) = &mut self.finding { candidate.place = place; }
            }
            Message::ReplaceAt(slot) => {
                if self.editing().is_some_and(|pool| slot < pool.slots.len()) {
                    effects.extend(self.update(Message::Select(Some(slot)), now));
                    effects.extend(self.update(Message::Replace, now));
                }
            }
            Message::Grouped(on) => {
                if let Some(editor) = self.editor_mut() {
                    editor.grouped = on;
                }
            }
            Message::SetMod(slot, mods) => {
                if self.editing().is_some_and(|pool| slot < pool.slots.len()) {
                    if let Some(editor) = self.editor_mut() { editor.last_mod = mods; editor.last_category.clear(); editor.last_colour = None; }
                }
                if let Some(at) = self.editing_at() {
                    if self.list[at].slots.get(slot).is_some_and(|entry| entry.mods != mods || !entry.category.is_empty()) {
                        self.remember(at);
                        self.list[at].slots[slot].mods = mods;
                        self.list[at].slots[slot].category.clear();
                        self.list[at].slots[slot].colour = None;
                        self.list[at].slots[slot].measure = None;
                        self.save(at, now);
                        effects.extend(self.measure_effects());
                    }
                }
            }
            Message::AddMod(mods) => {
                if let Some(editor) = self.editor_mut() { editor.last_mod = mods; editor.last_category.clear(); editor.last_colour = None; }
                if let Some(slot) = self.editing().and_then(|pool| match &self.screen {
                    Screen::Editor(editor) => target(pool, editor),
                    _ => None,
                }) {
                    effects.extend(self.update(Message::SetMod(slot, mods), now));
                }
            }
            Message::Clear(slot) => {
                if let Some(at) = self.editing_at() {
                    self.remember(at);
                    let pool = &mut self.list[at];
                    if slot < pool.slots.len() {
                        if pool.frame == Frame::Free || pool.slots[slot].is_empty() {
                            pool.slots.remove(slot);
                        } else {
                            pool.slots[slot].clear();
                        }
                        if let Some(editor) = self.editor_mut() {
                            editor.selected = None;
                            editor.panel = Panel::Closed;
                        }
                        self.save(at, now);
                    }
                }
            }
            Message::Choosing(on) => {
                if let Some(editor) = self.editor_mut() {
                    editor.choosing = on;
                    editor.marked.clear();
                    editor.bulk = None;
                    if on {
                        editor.panel = Panel::Closed;
                        editor.selected = None;
                    }
                }
            }
            Message::Mark(slot) => {
                if let Some(editor) = self.editor_mut() {
                    match editor.marked.iter().position(|marked| *marked == slot) {
                        Some(at) => {
                            editor.marked.remove(at);
                        }
                        None => editor.marked.push(slot),
                    }
                    editor.bulk = None;
                }
            }
            Message::Bulk(kind) => {
                if let Some(editor) = self.editor_mut() {
                    editor.bulk = if editor.bulk == kind { None } else { kind };
                }
            }
            Message::BulkMod(mods) => {
                let marked = self.marked_slots();
                if let Some(at) = self.editing_at() {
                    if !marked.is_empty() {
                        self.remember(at);
                        for slot in marked {
                            if let Some(entry) = self.list[at].slots.get_mut(slot) {
                                if entry.mods != mods {
                                    entry.mods = mods;
                                    entry.measure = None;
                                }
                            }
                        }
                        self.save(at, now);
                        effects.extend(self.measure_effects());
                    }
                }
                if let Some(editor) = self.editor_mut() {
                    editor.bulk = None;
                }
            }
            Message::Move(id, keys, key, before) => {
                if let Some(at) = self.editing_at() {
                    if self.list[at].id == id && slot_keys(&self.list[at]) == keys {
                        if let Some(order) = moved_order(&keys, key, before) {
                            self.remember(at);
                            self.reorder_slots(at, &order, now);
                        }
                    }
                }
            }
            Message::Shift(down) => {
                let marked = self.marked_slots();
                if let Some(at) = self.editing_at() {
                    let length = self.list[at].slots.len();
                    if shifted(&marked, length, down) != marked {
                        self.remember(at);
                        let mut order: Vec<usize> = (0..length).collect();
                        shift_in(&mut order, &marked, down);
                        self.reorder_slots(at, &order, now);
                    }
                }
            }
            Message::RemoveMarked => {
                let marked = self.marked_slots();
                if let Some(at) = self.editing_at() {
                    if !marked.is_empty() {
                        self.remember(at);
                        let free = self.list[at].frame == Frame::Free;
                        for slot in marked.iter().rev() {
                            if *slot < self.list[at].slots.len() {
                                if free {
                                    self.list[at].slots.remove(*slot);
                                } else {
                                    self.list[at].slots[*slot].clear();
                                }
                            }
                        }
                        self.save(at, now);
                    }
                }
                if let Some(editor) = self.editor_mut() {
                    editor.marked.clear();
                    editor.bulk = None;
                    editor.selected = None;
                }
            }
            Message::Undo => effects.extend(self.step_history(true, now)),
            Message::Redo => effects.extend(self.step_history(false, now)),
            Message::Find(said) => {
                if let Some(editor) = self.editor_mut() {
                    editor.find = said;
                }
            }
            Message::Duplicate(slot) => {
                if let Some(at) = self.editing_at().filter(|at| self.list[*at].slots.get(slot).is_some_and(|slot| !slot.is_empty())) {
                    self.remember(at);
                    let copy = self.list[at].slots[slot].clone();
                    self.list[at].slots.insert(slot + 1, copy);
                    self.save(at, now);
                    if let Some(editor) = self.editor_mut() {
                        editor.selected = editor.selected.map(|held| if held > slot { held + 1 } else { held });
                        editor.marked.clear();
                    }
                }
            }
            Message::SlotMenu(slot) => {
                if let Some(editor) = self.editor_mut() {
                    editor.menu = slot;
                    editor.palette = false;
                }
            }
            Message::Palette(open) => {
                if let Some(editor) = self.editor_mut() {
                    editor.palette = open;
                    editor.menu = None;
                    editor.command.clear();
                }
            }
            Message::Command(said) => {
                if let Some(editor) = self.editor_mut() {
                    editor.command = said;
                }
            }
            Message::Chosen(inner) => {
                let stays = matches!(*inner, Message::History(true));
                if let Some(editor) = self.editor_mut() {
                    editor.menu = None;
                    editor.palette = stays;
                    editor.command.clear();
                }
                effects.extend(self.update(*inner, now));
            }
            Message::CopySlot(slot) => {
                if let Some(hash) = self.editing().and_then(|pool| pool.slots.get(slot)).and_then(|slot| slot.hash.clone()) {
                    let link = self.songs.as_ref().and_then(|songs| songs.get(&hash)).and_then(|map| map_link(&map.file));
                    match link {
                        Some(link) => effects.push(Effect::Copy(link, "pool-copied-link")),
                        None => effects.push(Effect::Copy(hash, "pool-copied-hash")),
                    }
                }
            }
            Message::CopySlotId(slot) => {
                if let Some(hash) = self.editing().and_then(|pool| pool.slots.get(slot)).and_then(|slot| slot.hash.clone()) {
                    match self.songs.as_ref().and_then(|songs| songs.get(&hash)).and_then(|map| map_id(&map.file)) {
                        Some(id) => effects.push(Effect::Copy(id.to_string(), "pool-copied-id")),
                        None => effects.push(Effect::Copy(hash, "pool-copied-hash")),
                    }
                }
            }
            Message::OnlyEmpty => {
                if let Some(editor) = self.editor_mut() {
                    editor.only_empty = !editor.only_empty;
                }
            }
            Message::SortStars => {
                if let Some(at) = self.editing_at() {
                    let pool = &self.list[at];
                    let key = |slot: &Slot| if slot.category.is_empty() { slot.mods.code().to_owned() } else { slot.category.clone() };
                    let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
                    for (spot, slot) in pool.slots.iter().enumerate() {
                        let name = key(slot);
                        match groups.iter_mut().find(|(held, _)| *held == name) {
                            Some((_, spots)) => spots.push(spot),
                            None => groups.push((name, vec![spot])),
                        }
                    }
                    let mut sorted = pool.slots.clone();
                    for (_, spots) in &groups {
                        let mut held: Vec<Slot> = spots.iter().map(|spot| pool.slots[*spot].clone()).collect();
                        held.sort_by(|a, b| match (a.measure, b.measure) {
                            (Some(a), Some(b)) => a.stars.total_cmp(&b.stars),
                            (Some(_), None) => std::cmp::Ordering::Less,
                            (None, Some(_)) => std::cmp::Ordering::Greater,
                            (None, None) => std::cmp::Ordering::Equal,
                        });
                        for (spot, slot) in spots.iter().zip(held) {
                            sorted[*spot] = slot;
                        }
                    }
                    if sorted != pool.slots {
                        self.remember(at);
                        self.list[at].slots = sorted;
                        self.save(at, now);
                        if let Some(editor) = self.editor_mut() {
                            editor.selected = None;
                            editor.marked.clear();
                            if editor.panel == Panel::Slot { editor.panel = Panel::Closed; }
                        }
                    }
                }
            }
            Message::Put(hash) => effects.extend(self.put(&hash, now)),
            Message::Note(slot, note) => {
                if let Some(at) = self.editing_at() {
                    if let Some(entry) = self.list[at].slots.get_mut(slot) {
                        entry.note = note;
                        self.defer_save(at, now);
                    }
                }
            }
            _ => unreachable!("message routed to the wrong pool handler"),
        }
        effects.extend(self.import_next());
        effects
    }
}
