mod catalogue;
mod publication;
mod identity;
mod sources;
mod importing;
mod editing;
mod backdrop;
mod navigation;

use super::*;

impl State {
    pub fn update(&mut self, message: Message, now: i64) -> Vec<Effect> {
        let shown = match &self.screen {
            Screen::Editor(editor) if matches!(self.panel_key(), "panel" | "publish") => Some(editor.clone()),
            _ => None,
        };
        self.clock = now;
        let before = self.guide;
        if self.guide.on {
            match &message {
                Message::SetMod(..) | Message::BulkMod(..) | Message::Category(..) => self.guide.modded = true,
                Message::Move(..) => self.guide.moved = true,
                Message::Share(true) | Message::Published(_, Ok(_)) => self.guide.shared = true,
                _ => {}
            }
        }
        self.depth += 1;
        let mut effects = std::mem::take(&mut self.pending);
        effects.extend(self.handle(message, now));
        self.depth -= 1;
        if self.guide.on && self.editing().is_some_and(|pool| !pool.collection && self.guide.current(pool).is_none()) {
            self.guide.on = false;
        }
        if self.depth == 0 && self.guide != before {
            effects.push(Effect::Guide(self.guide));
        }
        if let (Some(was), "none") = (shown, self.panel_key()) {
            if let Some(editor) = self.editor_mut().filter(|editor| editor.id == was.id) {
                editor.gone = Some(Box::new(Editor { gone: None, ..was }));
            }
        }
        let filled = self.editing().is_some_and(|pool| pool.filled() > 0);
        let open = self.panel_key() != "none";
        if let Some(editor) = self.editor_mut().filter(|editor| editor.starting && (filled || open)) {
            editor.starting = false;
        }
        if self.depth == 0 {
            if !matches!(&self.screen, Screen::Editor(editor) if editor.panel == Panel::Add) {
                self.links.clear();
                self.linking = false;
                if let Some(editor) = self.editor_mut() {
                    editor.lean = None;
                }
            }
            effects.extend(self.link_next());
        }
        effects
    }

    pub(super) fn handle(&mut self, message: Message, now: i64) -> Vec<Effect> {
        if message.edits() && matches!(self.screen, Screen::Editor(_)) && self.editing().is_some_and(|pool| pool.saved.is_some()) {
            return Vec::new();
        }
        match message {
            Message::Makers(..)
            | Message::Catalogue(..)
            | Message::Shelf(..)
            | Message::SavePublication(..)
            | Message::Pull(..)
            | Message::Renew(..)
            | Message::StopPull
            | Message::Collected(..)
            | Message::CatalogueMore
            | Message::CatalogueLoaded(..)
            | Message::OpenPublication(..)
            | Message::RetryCatalogue
            | Message::History(..)
            | Message::OpenVersion(..) => self.handle_catalogue(message, now),
            Message::PublishSheet(..)
            | Message::AskWithdraw(..)
            | Message::Withdraw
            | Message::Withdrawn(..)
            | Message::Publish
            | Message::Published(..)
            | Message::Conflict(..)
            | Message::Share(..)
            | Message::CopyText
            | Message::CopyHash
            | Message::SaveImage
            | Message::ImageSaved(..)
            | Message::SaveFile
            | Message::OpenFile
            | Message::Saved(..) => self.handle_publication(message, now),
            Message::AuthorsDraft(..)
            | Message::ApplyAuthors
            | Message::AuthorDraft(..)
            | Message::AddAuthor
            | Message::DropAuthor(..)
            | Message::EditAuthors
            | Message::OwnCompiler(..)
            | Message::Face(..)
            | Message::Authors(..)
            | Message::Author(..)
            | Message::Rename(..)
            | Message::Categories(..)
            | Message::Tint(..)
            | Message::DropCategory(..)
            | Message::NewCategory
            | Message::MoveCategory(..)
            | Message::Category(..)
            | Message::CreateCategory(..)
            | Message::CategoryDraft(..)
            | Message::SaveCategory(..)
            | Message::Colour(..) => self.handle_identity(message, now),
            Message::Lean(..)
            | Message::AddPanel(..)
            | Message::Source(..)
            | Message::RefreshBest
            | Message::Best(..)
            | Message::Collection(..)
            | Message::CollectionPage(..)
            | Message::CollectionHash(..)
            | Message::Collections(..)
            | Message::Suggest(..)
            | Message::Offered(..)
            | Message::TakeOffered(..)
            | Message::Suggested(..)
            | Message::Query(..) => self.handle_sources(message, now),
            Message::PasteInto(..)
            | Message::PastedInto(..)
            | Message::Pasted(..)
            | Message::Resolved(..)
            | Message::Cover(..)
            | Message::Choose(..)
            | Message::Aim(..)
            | Message::Confirm
            | Message::ConfirmAll
            | Message::Dismiss
            | Message::FetchSlot(..)
            | Message::Retry
            | Message::Step(..)
            | Message::Imported(..)
            | Message::Dropped(..)
            | Message::MapFile(..)
            | Message::ArchiveFile(..)
            | Message::PoolFile(..)
            | Message::GetMissing
            | Message::Keep
            | Message::FetchMissing
            | Message::Songs(..)
            | Message::Measured(..) => self.handle_importing(message, now),
            Message::ShowSlot(..)
            | Message::Select(..)
            | Message::Mark(..)
            | Message::Replace
            | Message::ReplaceAt(..)
            | Message::Grouped(..)
            | Message::SetMod(..)
            | Message::AddMod(..)
            | Message::Clear(..)
            | Message::Choosing(..)
            | Message::Bulk(..)
            | Message::BulkMod(..)
            | Message::Move(..)
            | Message::Shift(..)
            | Message::RemoveMarked
            | Message::Undo
            | Message::Redo
            | Message::Find(..)
            | Message::Duplicate(..)
            | Message::SlotMenu(..)
            | Message::Palette(..)
            | Message::Command(..)
            | Message::Chosen(..)
            | Message::CopySlot(..)
            | Message::CopySlotId(..)
            | Message::OnlyEmpty
            | Message::SortStars
            | Message::Put(..)
            | Message::Note(..) => self.handle_editing(message, now),
            Message::BackdropPanel(..)
            | Message::BackdropFrom(..)
            | Message::BackdropPick
            | Message::BackdropPicked(..)
            | Message::BackdropDim(..)
            | Message::BackdropBlur(..)
            | Message::BackdropCard(..)
            | Message::BackdropLoaded(..) => self.handle_backdrop(message, now),
            Message::FlushSaves
            | Message::RetrySaves
            | Message::Begin(..)
            | Message::HideGuide
            | Message::GuideNext
            | Message::GuideGot
            | Message::ImportCollection(..)
            | Message::New
            | Message::Open(..)
            | Message::Back
            | Message::UseFrame(..)
            | Message::DropStray
            | Message::Hot(..)
            | Message::AskDelete(..)
            | Message::DeletePool
            | Message::Uncollected(..) => self.handle_navigation(message, now),
        }
    }
}
