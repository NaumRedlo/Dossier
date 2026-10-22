use super::*;

impl State {
    pub(super) fn handle_identity(&mut self, message: Message, now: i64) -> Vec<Effect> {
        let mut effects = Vec::new();
        match message {
            Message::AuthorsDraft(said) => {
                let apply = said.ends_with(char::is_whitespace) && said.chars().filter(|ch| *ch == '"').count() % 2 == 0;
                if let Some(editor) = self.editor_mut() { editor.authors = said; }
                if apply { effects.extend(self.update(Message::ApplyAuthors, now)); }
            }
            Message::ApplyAuthors => { if let Screen::Editor(editor) = &self.screen { effects.extend(self.update(Message::Authors(editor.authors.clone()), now)); } if let Some(editor) = self.editor_mut() { editor.editing_authors = false; } }
            Message::AuthorDraft(said) => {
                let done = said.ends_with(',');
                if let Some(editor) = self.editor_mut() { editor.author_draft = said.trim_end_matches(',').to_owned(); }
                if done { effects.extend(self.update(Message::AddAuthor, now)); }
            }
            Message::AddAuthor => {
                let draft = match &self.screen { Screen::Editor(editor) => editor.author_draft.clone(), _ => String::new() };
                if let Some(at) = self.editing_at() {
                    let mut added = false;
                    for name in draft.split([',', ';']).map(|name| name.trim().trim_matches('"').to_owned()).filter(|name| !name.is_empty()) {
                        if !self.list[at].authors.iter().any(|known| known.eq_ignore_ascii_case(&name)) {
                            if !added { self.remember(at); }
                            self.list[at].authors.push(name);
                            added = true;
                        }
                    }
                    if added {
                        effects.push(Effect::Faces(self.list[at].authors.clone()));
                        self.save(at, now);
                    }
                }
                if let Some(editor) = self.editor_mut() { editor.author_draft.clear(); editor.untouched = false; }
            }
            Message::DropAuthor(name) => {
                if let Some(at) = self.editing_at().filter(|at| self.list[*at].authors.contains(&name)) {
                    self.remember(at);
                    self.list[at].authors.retain(|known| *known != name);
                    self.save(at, now);
                }
            }
            Message::Categories(open) => {
                if let Some(editor) = self.editor_mut() { editor.categories = open; editor.category_draft.clear(); }
            }
            Message::Tint(None, colour) => {
                if let Some(editor) = self.editor_mut() { editor.tint = colour; }
            }
            Message::Tint(Some(name), colour) => {
                if let Some(at) = self.editing_at().filter(|_| !Mod::ALL.iter().any(|mods| mods.code() == name)) {
                    self.remember(at);
                    self.list[at].categories.insert(name.clone(), colour);
                    for slot in &mut self.list[at].slots { if slot.category == name { slot.colour = Some(colour); } }
                    self.save(at, now);
                }
            }
            Message::DropCategory(name) => {
                if let Some(at) = self.editing_at() {
                    self.remember(at);
                    for slot in &mut self.list[at].slots { if slot.category == name { slot.category.clear(); slot.colour = None; } }
                    self.list[at].categories.remove(&name);
                    self.list[at].category_order.retain(|known| *known != name);
                    self.save(at, now);
                }
            }
            Message::NewCategory => {
                let draft = match &self.screen { Screen::Editor(editor) => Some((editor.category_draft.clone(), editor.tint)), _ => None };
                if let (Some(at), Some((name, colour))) = (self.editing_at(), draft) {
                    let taken = Mod::ALL.iter().any(|mods| mods.code() == name) || self.list[at].categories.contains_key(&name) || self.list[at].slots.iter().any(|slot| slot.category == name);
                    if !name.is_empty() && !taken {
                        self.remember(at);
                        self.list[at].categories.insert(name.clone(), colour);
                        let mut slot = Slot::empty(Mod::Nm);
                        slot.category = name;
                        slot.colour = Some(colour);
                        self.list[at].slots.push(slot);
                        self.save(at, now);
                        if let Some(editor) = self.editor_mut() { editor.category_draft.clear(); editor.untouched = false; }
                    }
                }
            }
            Message::EditAuthors => { if let Some(editor) = self.editor_mut() { editor.editing_authors = true; } }
            Message::OwnCompiler(id, name) => {
                if let Some(at) = self.list.iter().position(|pool| pool.id == id && !name.is_empty() && pool.compiler != name) {
                    self.list[at].compiler = name.clone(); self.save(at, now);
                    effects.push(Effect::Faces(vec![name]));
                }
            }
            Message::Face(name, face) => { if let Some(face) = face { self.faces.insert(name.to_lowercase(), face); } }
            Message::MoveCategory(name, down) => {
                if let Some(at) = self.editing_at() {
                    let mut order = category_order(&self.list[at]);
                    if let Some(from) = order.iter().position(|key| key == &name) {
                        let to = if down { from.saturating_add(1) } else { from.saturating_sub(1) };
                        if to < order.len() && to != from { self.remember(at); order.swap(from, to); self.list[at].category_order = order; self.save(at, now); }
                    }
                }
            }
            Message::Rename(name) => {
                if let Some(at) = self.editing_at() {
                    self.list[at].name = name;
                    if let Some(editor) = self.editor_mut() {
                        editor.untouched = false;
                    }
                    self.defer_save(at, now);
                }
            }
            Message::Authors(said) => {
                if let Some(at) = self.editing_at() {
                    let mut authors = Vec::new();
                    for author in author_names(&said) {
                        if !authors.iter().any(|known: &String| known.eq_ignore_ascii_case(&author)) { authors.push(author); }
                    }
                    self.list[at].authors = authors;
                    effects.push(Effect::Faces(self.list[at].authors.clone()));
                    if let Some(editor) = self.editor_mut() { editor.authors = said; editor.untouched = false; }
                    self.save(at, now);
                }
            }
            Message::Category(slot, name) => {
                if let Some(at) = self.editing_at() {
                    let colour = self.list[at].categories.get(&name).copied().or_else(|| self.list[at].slots.iter().find(|slot| slot.category == name).and_then(|slot| slot.colour));
                    if let Some(editor) = self.editor_mut() { editor.last_category = name.clone(); editor.last_colour = colour; }
                }
                if let Some(at) = self.editing_at().filter(|at| slot < self.list[*at].slots.len()) {
                    self.remember(at);
                    self.list[at].slots[slot].category = name.clone();
                    if let Some(colour) = self.list[at].categories.get(&name).copied().or_else(|| self.list[at].slots.iter().find(|item| item.category == name && item.colour.is_some()).and_then(|item| item.colour)) { self.list[at].slots[slot].colour = Some(colour); }
                    if let Some(editor) = self.editor_mut() { editor.last_category = name; }
                    self.save(at, now);
                }
            }
            Message::Author(name) => { self.makers = None; effects.push(Effect::Author(name)); }
            Message::CreateCategory(open) => {
                if let Some(editor) = self.editor_mut() { editor.category_creator = open; editor.category_draft.clear(); editor.colour_draft = "#5ec2d0".into(); }
            }
            Message::CategoryDraft(name) => {
                if let Some(editor) = self.editor_mut() { editor.category_draft = name.chars().filter(|c| c.is_alphabetic()).flat_map(char::to_uppercase).take(2).collect(); }
            }
            Message::SaveCategory(slot) => {
                let draft = match &self.screen { Screen::Editor(editor) => Some((editor.category_draft.clone(), editor.colour_draft.clone())), _ => None };
                if let Some((name, colour)) = draft.filter(|(name, colour)| !name.is_empty() && !Mod::ALL.iter().any(|m| m.code() == name) && parse_colour(colour).is_some()) {
                    if let Some(at) = self.editing_at() { self.remember(at); self.list[at].categories.insert(name.clone(), parse_colour(&colour).unwrap()); self.save(at, now); }
                    if let Some(editor) = self.editor_mut() { editor.category_creator = false; }
                    effects.extend(self.update(Message::Category(slot, name), now));
                    effects.extend(self.update(Message::Colour(slot, colour), now));
                    if let Some(editor) = self.editor_mut() { editor.category_creator = false; }
                }
            }
            Message::Colour(slot, value) => {
                if let Some(editor) = self.editor_mut() { editor.colour_draft = value.clone(); }
                if matches!(&self.screen, Screen::Editor(editor) if editor.category_creator) { return effects; }
                if let (Some(at), Some(colour)) = (self.editing_at(), parse_colour(&value)) {
                    if let Some(category) = self.list[at].slots.get(slot).map(|slot| slot.category.clone()).filter(|category| !category.is_empty()) {
                        self.remember(at);
                        self.list[at].categories.insert(category.clone(), colour);
                        for slot in &mut self.list[at].slots { if slot.category == category { slot.colour = Some(colour); } }
                        if let Some(editor) = self.editor_mut() { editor.last_colour = Some(colour); }
                        self.save(at, now);
                    }
                }
            }
            _ => unreachable!("message routed to the wrong pool handler"),
        }
        effects.extend(self.import_next());
        effects
    }
}
