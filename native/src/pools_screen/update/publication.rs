use super::*;

impl State {
    pub(super) fn handle_publication(&mut self, message: Message, now: i64) -> Vec<Effect> {
        let mut effects = Vec::new();
        match message {
            Message::PublishSheet(open) => {
                if open {
                    effects.extend(self.update(Message::Choosing(false), now));
                    self.catalogue_error = None;
                }
                if let Some(editor) = self.editor_mut().filter(|editor| !editor.starting) {
                    editor.publish = open;
                    editor.asking_withdraw = false;
                    if open {
                        editor.panel = Panel::Closed;
                        editor.share = false;
                    }
                }
            }
            Message::AskWithdraw(asking) => {
                if let Some(editor) = self.editor_mut() {
                    editor.asking_withdraw = asking;
                }
            }
            Message::Withdraw => {
                if !self.publishing {
                    if let Some(pool) = self.editing().filter(|pool| pool.published_revision > 0).cloned() {
                        self.publishing = true;
                        self.catalogue_error = None;
                        effects.push(Effect::Withdraw(pool));
                    }
                }
            }
            Message::Withdrawn(id, result) => {
                self.publishing = false;
                match result {
                    Ok(()) => {
                        if let Some(at) = self.list.iter().position(|pool| pool.id == id) { self.list[at].published_revision = 0; self.save(at, now); }
                        self.publications.retain(|row| !(row.mine && row.local_id == id));
                        self.catalogue_error = None;
                        if let Some(editor) = self.editor_mut().filter(|editor| editor.id == id) { editor.publish = false; editor.asking_withdraw = false; }
                        effects.push(Effect::Say("catalogue-withdrawn"));
                    }
                    Err(why) => {
                        self.catalogue_error = Some(why);
                        if let Some(editor) = self.editor_mut() { editor.asking_withdraw = false; }
                    }
                }
            }
            Message::Publish => {
                if !self.publishing {
                    if let Some(pool) = self.editing().filter(|pool| publish_ready(pool)).cloned() {
                        self.publishing = true;
                        self.catalogue_error = None;
                        effects.push(Effect::Publish(pool));
                    }
                }
            }
            Message::Published(id, result) => {
                self.publishing = false;
                match result {
                    Ok(publication) => {
                        if let Some(at) = self.list.iter().position(|pool| pool.id == id) {
                            self.list[at].published_revision = publication.revision;
                            self.save(at, now);
                            if let Some(pool) = self.list.iter().find(|pool| pool.id == id) {
                                let _ = pools::keep_version(&self.dir, pool);
                            }
                            self.versions = pools::versions(&self.dir, &id);
                    effects.extend(self.backdrop_effect());
                        }
                        if self.conflict.as_deref() == Some(id.as_str()) { self.conflict = None; }
                        self.publications.retain(|p| p.id != publication.id);
                        self.publications.insert(0, publication);
                        self.catalogue_error = None;
                        if let Some(editor) = self.editor_mut().filter(|editor| editor.id == id && editor.publish) {
                            editor.publish = false;
                            effects.push(Effect::Say("catalogue-published-now"));
                        }
                    }
                    Err(why) if why == "409" => {
                        self.catalogue_error = None;
                        self.conflict = Some(id.clone());
                        let collection = self.list.iter().find(|pool| pool.id == id).is_some_and(|pool| pool.collection);
                        if let Some(editor) = self.editor_mut().filter(|editor| editor.id == id) { editor.publish = false; }
                        effects.extend(self.refresh_for(collection));
                    }
                    Err(why) => self.catalogue_error = Some(why),
                }
            }
            Message::Conflict(keep) => {
                if let Some(pool) = self.editing().filter(|pool| self.conflict.as_deref() == Some(pool.id.as_str()) && self.resolving.is_none()) {
                    let (id, collection) = (pool.id.clone(), pool.collection);
                    self.resolving = Some((id, keep));
                    effects.extend(self.refresh_for(collection));
                }
            }
            Message::Share(open) => {
                if let Some(editor) = self.editor_mut() {
                    editor.share = open;
                }
            }
            Message::CopyText => {
                if let Some(pool) = self.editing() {
                    effects.push(Effect::Copy(pool_share::to_text(pool), "pool-copied"));
                }
            }
            Message::CopyHash => {
                if let Some(pool) = self.editing() {
                    effects.push(Effect::Copy(pool.fingerprint(), "pool-copied-hash"));
                }
            }
            Message::SaveImage => {
                if self.exporting.is_none() {
                    if let Some(pool) = self.editing().filter(|pool| pool.filled() > 0).cloned() {
                        self.export_request = self.export_request.wrapping_add(1);
                        self.exporting = Some(self.export_request);
                        effects.push(Effect::SaveImage(self.export_request, pool));
                    }
                }
            }
            Message::ImageSaved(request, said) => {
                if self.exporting == Some(request) {
                    self.exporting = None;
                    match said {
                        Ok(Some(_)) => effects.push(Effect::Say("pool-image-saved")),
                        Err(_) => effects.push(Effect::Say("pool-save-failed")),
                        Ok(None) => {}
                    }
                }
            }
            Message::SaveFile => {
                if let Some(pool) = self.editing() {
                    effects.push(Effect::SaveFile(file_name_of(pool), pool_share::to_file(pool)));
                }
            }
            Message::OpenFile => effects.push(Effect::PickFile),
            Message::Saved(_) => {}
            _ => unreachable!("message routed to the wrong pool handler"),
        }
        effects.extend(self.import_next());
        effects
    }
}
