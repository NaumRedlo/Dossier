use super::*;
use iced::widget::column;

pub(super) fn share_row<'a>(icon: Icon, title: String, hint: String, action: String, press: Option<Message>) -> Element<'a, Message> {
    let k = ui::fade();
    let tile = container(glyph(icon, 16.0, MUTED))
        .width(38.0)
        .height(38.0)
        .center(38.0)
        .style(move |_| container::Style { background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.012 * k))), border: Border { color: crate::theme::outline(Color::from_rgba(1.0, 1.0, 1.0, 0.04 * k)), width: 1.0, radius: 10.0.into() }, ..container::Style::default() });
    let words = column![semi(title, 14.0, INK), container(mono(hint, 11.5, FAINT)).clip(true)].spacing(3).width(Length::Fill);
    row![tile, words, match press {
        Some(press) => quiet_button(action, press),
        None => ui::quiet(action, None),
    }]
    .spacing(14)
    .align_y(iced::Center)
    .into()
}

pub(super) fn share_modal<'a>(pool: &'a Pool, words: &'a Words, exporting: bool) -> Element<'a, Message> {
    let line = pool_share::to_text(pool);
    let shown = if line.chars().count() > 46 { format!("{}…", line.chars().take(46).collect::<String>()) } else { line };
    let hash = row![
        ui::label(words.t("pool-hash"), FAINT),
        text(pool.fingerprint()).font(theme::MONO_BOLD).size(14.0).color(ui::faded(INK)),
        icon_button(Icon::Copy, 14.0, MUTED, Some(Message::CopyHash)),
    ]
    .spacing(10)
    .align_y(iced::Center);
    let inside = column![
        row![ui::told(semi(words.t("pool-share-title"), 18.0, INK), words.t("pool-share-note"), 16.0), ui::grow(), hash].align_y(iced::Center),
        share_row(Icon::File, words.t("pool-share-file"), words.t("pool-share-file-hint"), words.t("pool-save"), Some(Message::SaveFile)),
        share_row(Icon::Copy, words.t("pool-share-string"), shown, words.t("pool-copy"), Some(Message::CopyText)),
        share_row(Icon::Pool, words.t("pool-share-image"), words.t("pool-share-image-hint"), words.t(if exporting { "pool-image-saving" } else { "pool-save" }), (!exporting).then_some(Message::SaveImage)),
        row![ui::grow(), quiet_button(words.t("pool-close"), Message::Share(false))],
    ]
    .spacing(16);
    let card = sheet_box(inside.into());
    let veil = iced::widget::mouse_area(ui::veil(theme::SCRIM)).on_press(Message::Share(false));
    iced::widget::stack![veil, container(container(card).max_width(620.0).padding(24)).center(Length::Fill)].into()
}

pub(super) const WARNED: Color = Color::from_rgb(1.0, 0.61, 0.61);

pub(super) fn check_line<'a>(check: &Check, words: &Words) -> Element<'a, Message> {
    let k = ui::fade();
    let mark: Element<'a, Message> = match check.verdict {
        Verdict::Fine => glyph(Icon::Check, 16.0, ACCENT),
        Verdict::Stop => glyph(Icon::Close, 16.0, WARNED),
        Verdict::Note => container(container(Space::new().width(6).height(6)).style(move |_| container::Style { background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.28 * k))), border: Border { radius: 3.0.into(), ..Border::default() }, ..container::Style::default() })).center(16.0).into(),
    };
    let said: Element<'a, Message> = match check.verdict {
        Verdict::Stop => text(check.words(words)).font(theme::MONO_BOLD).size(14.0).color(ui::faded(WARNED)).into(),
        Verdict::Fine | Verdict::Note => text(check.words(words)).font(theme::SANS).size(14.0).color(ui::faded(if check.verdict == Verdict::Fine { MUTED } else { FAINT })).into(),
    };
    let mut line = row![mark, container(said).width(Length::Fill)].spacing(12).align_y(iced::Center);
    if let Some(slot) = check.slot {
        line = line.push(outlined(words.t("publish-show-slot"), Some(Message::ShowSlot(slot))));
    }
    container(line).center_y(34.0).into()
}

pub(super) fn publish_panel<'a>(state: &'a State, editor: &'a Editor, pool: &'a Pool, words: &'a Words, thumbs: &'a HashMap<String, image::Handle>) -> Element<'a, Message> {
    let k = ui::fade();
    let caption = |key: &str| ui::section(words.t(key));
    let named = !pool.name.trim().is_empty();
    let name = if named { pool.name.clone() } else { words.t(if pool.collection { "collection-name-hint" } else { "pool-name-hint" }) };
    let head = row![
        column![caption("publish-title"), semi(name.clone(), 21.0, if named { INK } else { MUTED })].spacing(5).width(Length::Fill),
        ui::close_square(Some(Message::PublishSheet(false)), ui::CLOSE_SIDE),
    ]
    .spacing(12)
    .align_y(iced::Center);
    let checks = publish_checks(pool);
    let ready = checks.iter().all(|check| check.verdict != Verdict::Stop);
    let mut list = column![caption("publish-before")].spacing(4);
    for check in &checks {
        list = list.push(check_line(check, words));
    }
    let hashes: Vec<Option<&str>> = pool.slots.iter().filter(|slot| !slot.is_empty()).take(3).map(|slot| slot.hash.as_deref()).collect();
    let head = row![cover_stack(thumbs, &hashes), head].spacing(16).align_y(iced::Center);
    let issue = container(text(words.n("publish-issue", pool.published_revision + 1)).font(theme::MONO_BOLD).size(12.0).color(ui::faded(INK)))
        .padding([4, 10])
        .style(move |_| container::Style { background: Some(Background::Color(tinted(ACCENT, 0.16, k))), border: Border { color: crate::theme::outline(tinted(ACCENT, 0.4, k)), width: 1.0, radius: 12.0.into() }, ..container::Style::default() });
    let mut who = row![].spacing(12).align_y(iced::Center);
    if !pool.compiler.is_empty() {
        who = who.push(named_face(state, &pool.compiler, 220.0));
    }
    who = who.push(issue);
    let mut facts = column![who].spacing(8);
    if pool.published_revision > 0 {
        facts = facts.push(faded_text(words.n("publish-replaces", pool.published_revision), 13.0, MUTED));
    }
    let list = container(list).padding([12, 16]).width(Length::Fill).style(move |_| container::Style { background: Some(Background::Color(Color { a: k, ..ui::mix(theme::GROUND, Color::BLACK, 0.3) })), border: Border { color: crate::theme::outline(soft(0.08, k)), width: 1.0, radius: 12.0.into() }, ..container::Style::default() });
    let go = ui::primary(
        words.t(if state.publishing { "catalogue-saving" } else if pool.published_revision > 0 { "catalogue-update" } else { "catalogue-publish" }),
        (ready && !state.publishing).then_some(Message::Publish),
    );
    let mut foot = row![go, outlined(words.t("publish-cancel"), Some(Message::PublishSheet(false))), ui::grow()].spacing(8).align_y(iced::Center);
    if pool.published_revision > 0 {
        foot = if editor.asking_withdraw {
            foot.push(primary_button(words.t("catalogue-withdraw-yes"), Message::Withdraw)).push(quiet_button(words.t("pool-delete-no"), Message::AskWithdraw(false)))
        } else {
            foot.push(button(container(text(words.t("catalogue-withdraw")).font(theme::SANS_SEMI).size(14.0)).center_y(CONTROL)).padding([0, 12]).style(ui::button_faded(theme::danger_words)).on_press_maybe((!state.publishing).then_some(Message::AskWithdraw(true))))
        };
    }
    let mut inside = column![head, facts, list].spacing(20);
    if let Some(error) = &state.catalogue_error {
        inside = inside.push(catalogue_error(error, words));
    }
    container(inside.push(foot))
        .padding(24)
        .width(Length::Fill)
        .style(move |_| container::Style {
            background: Some(Background::Color(Color { a: k, ..Color::from_rgb(0.071, 0.027, 0.039) })),
            border: Border { color: crate::theme::outline(soft(0.16, k)), width: 1.0, radius: 16.0.into() },
            ..container::Style::default()
        })
        .into()
}

pub(super) fn bulk_bar<'a>(editor: &'a Editor, words: &'a Words) -> Element<'a, Message> {
    let count = editor.marked.len();
    let mut buttons = row![
        semi(words.n("pool-marked", count as u64), 14.0, INK),
        quiet_button(words.t("pool-bulk-mod"), Message::Bulk(Some(Bulk::Mod))),
        quiet_button(words.t("pool-bulk-shift"), Message::Bulk(Some(Bulk::Shift))),
        quiet_button(words.t("pool-bulk-remove"), Message::RemoveMarked),
        plain_button(words.t("pool-bulk-clear"), Message::Choosing(false)),
    ]
    .spacing(12)
    .align_y(iced::Center);
    let mut inside = column![].spacing(12).align_x(iced::Center);
    match editor.bulk {
        Some(Bulk::Mod) => {
            let mut pills = row![].spacing(8);
            for mods in Mod::ALL {
                pills = pills.push(button(container(text(mods.code()).font(theme::SANS_SEMI).size(13.5)).center_y(CONTROL)).padding([0, 14]).style(ui::button_faded(theme::filter_chip(false))).on_press(Message::BulkMod(mods)));
            }
            inside = inside.push(pills);
        }
        Some(Bulk::Shift) => {
            let pills = row![
                button(container(text(words.t("pool-shift-up")).font(theme::SANS_SEMI).size(13.5)).center_y(CONTROL)).padding([0, 14]).style(ui::button_faded(theme::filter_chip(false))).on_press(Message::Shift(false)),
                button(container(text(words.t("pool-shift-down")).font(theme::SANS_SEMI).size(13.5)).center_y(CONTROL)).padding([0, 14]).style(ui::button_faded(theme::filter_chip(false))).on_press(Message::Shift(true)),
            ]
            .spacing(8);
            inside = inside.push(pills);
        }
        None => {}
    }
    buttons = buttons.align_y(iced::Center);
    inside = inside.push(buttons);
    let bar = sheet_box(inside.into());
    container(container(bar).max_width(640.0)).width(Length::Fill).height(Length::Fill).align_x(iced::alignment::Horizontal::Center).align_y(iced::alignment::Vertical::Bottom).padding(24).into()
}

