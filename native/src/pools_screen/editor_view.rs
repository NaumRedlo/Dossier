use super::*;
use iced::widget::column;

pub(super) fn back_button<'a>(words: &Words, collection: bool) -> Element<'a, Message> {
    button(container(text(words.t(if collection { "collection-back" } else { "pool-back" })).font(theme::SANS_SEMI).size(theme::BODY)).center_y(theme::CONTROL_HEIGHT - 2.0))
        .padding([0, 12])
        .style(ui::button_faded(|_, status| {
            let hot = matches!(status, button::Status::Hovered | button::Status::Pressed);
            button::Style {
                background: Some(Background::Color(soft(if hot { 0.09 } else { 0.045 }, 1.0))),
                text_color: INK,
                border: Border { color: crate::theme::outline(soft(if hot { 0.2 } else { 0.08 }, 1.0)), width: 1.0, radius: 8.0.into() },
                ..button::Style::default()
            }
        }))
        .on_press(Message::Back)
        .into()
}

pub(super) fn menu_line<'a>(label: String, hint: String, ink: Color, press: Option<Message>, lit: bool, high: f32) -> Element<'a, Message> {
    let free = press.is_some();
    let ink = if free { ink } else { FAINT };
    button(container(row![container(text(label).font(theme::SANS).size(14.0).color(ui::faded(ink))).width(Length::Fill), text(ui::keys(&hint)).font(theme::MONO).size(12.0).color(ui::faded(if free { MUTED } else { FAINT }))].spacing(16).align_y(iced::Center)).center_y(high))
        .padding([0, 12])
        .width(Length::Fill)
        .style(ui::button_faded(move |_: &iced::Theme, status: button::Status| button::Style {
            background: if lit && free { Some(Background::Color(Color { a: 0.16, ..ACCENT })) } else { matches!(status, button::Status::Hovered | button::Status::Pressed).then_some(Background::Color(soft(0.08, 1.0))) },
            text_color: INK,
            border: Border { radius: 8.0.into(), ..Border::default() },
            shadow: iced::Shadow::default(),
            snap: true,
        }))
        .on_press_maybe(press.map(|press| Message::Chosen(Box::new(press))))
        .into()
}

pub(super) fn menu_box<'a>(inside: Element<'a, Message>, wide: f32, pad: f32, round: f32, fill: Color) -> Element<'a, Message> {
    let k = ui::fade();
    iced::widget::opaque(container(inside).padding(pad).width(wide).style(move |_| container::Style {
        background: Some(Background::Color(Color { a: k, ..fill })),
        border: Border { color: crate::theme::outline(soft(0.16, k)), width: 1.0, radius: round.into() },
        ..container::Style::default()
    }))
}

pub(super) fn slot_menu<'a>(at: usize, words: &'a Words) -> Element<'a, Message> {
    let line = |key: &str, hint: &str, ink: Color, press: Message| menu_line(words.t(key), hint.to_owned(), ink, Some(press), false, 44.0);
    let lines = column![
        line("pool-menu-note", "N", INK, Message::Select(Some(at))),
        line("pool-menu-replace", "R", INK, Message::ReplaceAt(at)),
        line("pool-menu-double", "Ctrl D", INK, Message::Duplicate(at)),
        line("pool-menu-category", "", INK, Message::Select(Some(at))),
        line("pool-menu-copy", "Ctrl C", INK, Message::CopySlot(at)),
        line("pool-menu-copy-id", "", INK, Message::CopySlotId(at)),
        line("pool-menu-remove", "Delete", WARNED, Message::Clear(at)),
    ]
    .spacing(2);
    menu_box(lines.into(), 300.0, 8.0, 12.0, Color::from_rgb(0.114, 0.043, 0.059))
}

pub fn commands(state: &State, editor: &Editor, pool: &Pool) -> Vec<(&'static str, String, Option<Message>)> {
    let filled = pool.filled() > 0;
    let missing = state.missing(pool).len();
    let versions = state.versions.iter().filter(|held| held.id == pool.id).count();
    let mut list = vec![
        ("pool-command-add", "Ctrl N".to_owned(), Some(Message::AddPanel(true))),
        ("pool-command-paste", "Ctrl V".to_owned(), Some(Message::Begin(Start::Links))),
        ("pool-command-backdrop", String::new(), (filled && !pool.collection).then_some(Message::BackdropPanel(!editor.backdrop))),
        ("pool-command-share", String::new(), filled.then_some(Message::Share(true))),
        (if editor.choosing { "pool-command-choose-off" } else { "pool-command-choose" }, "pool-command-shift".to_owned(), filled.then_some(Message::Choosing(!editor.choosing))),
    ];
    if missing > 0 {
        list.push(("pool-command-missing", missing.to_string(), Some(Message::FetchMissing)));
    }
    if versions > 0 {
        list.push(("pool-command-history", versions.to_string(), Some(Message::History(true))));
    }
    if !pool.collection {
        list.push(("pool-command-authors", String::new(), Some(Message::EditAuthors)));
    }
    list
}

pub(super) fn palette<'a>(state: &'a State, editor: &'a Editor, pool: &'a Pool, words: &'a Words) -> Element<'a, Message> {
    let k = ui::fade();
    let ground = Color::from_rgb(0.071, 0.027, 0.039);
    if editor.history {
        let mut lines = column![menu_line(words.t("pool-history-back"), String::new(), MUTED, Some(Message::History(false)), false, 40.0)].spacing(2);
        for held in state.versions.iter().filter(|held| held.id == pool.id) {
            let label = words.with("pool-history-line", &[("n", held.published_revision.to_string()), ("date", words.day(held.changed_at, state.clock))]);
            lines = lines.push(menu_line(label, words.n("shelf-maps", held.filled() as u64), INK, Some(Message::OpenVersion(held.published_revision)), false, 40.0));
        }
        return menu_box(column![container(ui::mono_small(words.t("pool-command-history").to_uppercase(), MUTED)).padding([6, 12]), lines].spacing(8).into(), 440.0, 16.0, 14.0, ground);
    }
    let wanted = editor.command.trim().to_lowercase();
    let field = text_input(&words.t("pool-command-hint"), &editor.command).on_input(Message::Command).font(theme::SANS).size(16.0).padding(0).style(ui::bare_input(ui::fade())).width(Length::Fill);
    let field = container(row![field, text(ui::keys("Ctrl K")).font(theme::MONO).size(12.0).color(ui::faded(MUTED))].spacing(8).align_y(iced::Center)).padding([0, 12]).center_y(44.0).style(move |_| container::Style {
        background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.26 * k))),
        border: Border { color: crate::theme::outline(soft(0.08, k)), width: 1.0, radius: 8.0.into() },
        ..container::Style::default()
    });
    let mut lines = column![].spacing(2);
    let mut shown = 0;
    for (key, hint, press) in commands(state, editor, pool) {
        let label = words.t(key);
        if wanted.is_empty() || label.to_lowercase().contains(&wanted) {
            let ink = INK;
            let hint = if hint == "pool-command-shift" { words.t("pool-command-shift") } else { hint };
            lines = lines.push(menu_line(label, hint, ink, press, shown == 0, 40.0));
            shown += 1;
        }
    }
    if shown == 0 {
        lines = lines.push(container(faded_text(words.t("pool-command-none"), 14.0, MUTED)).padding([11, 12]));
    }
    menu_box(column![field, lines].spacing(12).into(), 440.0, 16.0, 14.0, ground)
}

pub fn slot_found(slot: &Slot, find: &str) -> bool {
    let wanted = find.trim().to_lowercase();
    if wanted.is_empty() {
        return true;
    }
    !slot.is_empty() && [slot.title.as_str(), slot.artist.as_str(), slot.version.as_str(), slot.note.as_str(), slot.category.as_str(), slot.mods.code()].iter().any(|held| held.to_lowercase().contains(&wanted))
}

pub fn saved_words(state: &State, pool: &Pool, words: &Words) -> String {
    if state.save_errors.contains_key(&pool.id) {
        words.t("pool-foot-unsaved")
    } else if state.unsaved.contains_key(&pool.id) {
        words.t("pool-saving")
    } else {
        let passed = (state.clock - pool.changed_at).max(0);
        match passed {
            0..=59 => words.t("pool-foot-saved-now"),
            60..=3599 => words.n("pool-foot-saved-minutes", (passed / 60) as u64),
            3600..=86_399 => words.n("pool-foot-saved-hours", (passed / 3600) as u64),
            _ => words.with("pool-foot-saved-on", &[("date", words.day(pool.changed_at, state.clock))]),
        }
    }
}

pub fn pool_foot<'a>(state: &State, pool: &Pool, words: &'a Words) -> Element<'a, Message> {
    let facts = pool.facts();
    let mut line = row![fact(facts.cards.to_string(), words.t("shelf-cards"))].spacing(26).align_y(iced::alignment::Vertical::Bottom);
    if facts.low.is_some() {
        line = line.push(column![stars_span(words, facts.low, facts.high), faded_text(words.t("shelf-stars"), 12.0, MUTED)].spacing(3));
    }
    let saved = saved_words(state, pool, words);
    line.push(ui::grow()).push(mono(saved, 12.0, MUTED)).into()
}

pub(super) fn editor_toolbar<'a>(state: &'a State, editor: &'a Editor, pool: &'a Pool, words: &'a Words, width: f32) -> Element<'a, Message> {
    let mut left = row![back_button(words, pool.collection)].spacing(8).align_y(iced::Center);
    let step = |icon: Icon, press: Option<Message>| -> Element<'a, Message> {
        let ink = if press.is_some() { INK } else { FAINT };
        icon_button(icon, 18.0, ink, press)
    };
    if !editor.starting {
        left = left.push(step(Icon::Undo, state.undo.iter().any(|(id, _)| *id == pool.id).then_some(Message::Undo)));
        left = left.push(step(Icon::Redo, state.redo.iter().any(|(id, _)| *id == pool.id).then_some(Message::Redo)));
        if pool.filled() > 1 {
            let k = ui::fade();
            let field = text_input(&words.t("pool-find"), &editor.find).on_input(Message::Find).font(theme::SANS).size(14.0).padding(0).style(ui::bare_input(ui::fade())).width(Length::Fill);
            left = left.push(container(row![glyph(Icon::Search, 14.0, MUTED), field].spacing(8).align_y(iced::Center)).padding([0, 12]).width((width - 730.0).clamp(140.0, 260.0)).center_y(CONTROL).style(move |_| container::Style {
                background: Some(Background::Color(Color { a: k, ..ui::mix(theme::GROUND, Color::BLACK, 0.26) })),
                border: Border { color: crate::theme::outline(soft(0.08, k)), width: 1.0, radius: 8.0.into() },
                ..container::Style::default()
            }));
            if !editor.find.trim().is_empty() {
                let found = pool.slots.iter().filter(|slot| slot_found(slot, &editor.find)).count();
                left = left.push(mono(words.of(found as u64, pool.filled() as u64), 12.0, MUTED));
            }
        }
    }
    if state.unsaved.contains_key(&pool.id) && !state.save_errors.contains_key(&pool.id) {
        left = left.push(text(words.t("pool-saving")).size(12).color(MUTED));
    }
    let publish = ui::primary(
        words.t(if state.publishing { "catalogue-saving" } else if pool.published_revision > 0 { "catalogue-update" } else { "catalogue-publish" }),
        (!state.publishing && !editor.starting && pool.saved.is_none()).then_some(Message::PublishSheet(true)),
    );
    let danger: Element<'a, Message> = if editor.asking_delete {
        row![outlined(words.t("pool-delete-yes"), Some(Message::DeletePool)), quiet_button(words.t("pool-delete-no"), Message::AskDelete(false))].spacing(8).align_y(iced::Center).into()
    } else {
        button(container(text(words.t(if pool.collection { "collection-delete" } else { "pool-delete" })).font(theme::SANS_SEMI).size(14.0)).center_y(CONTROL)).padding([0, 12]).style(ui::button_faded(theme::danger_words)).on_press(Message::AskDelete(true)).into()
    };
    let buttons = row![publish, danger].spacing(8).align_y(iced::Center);
    row![left.width(Length::Fill), buttons].spacing(12).align_y(iced::Center).into()
}

pub(super) fn swatches<'a>(chosen: Option<[u8; 3]>, press: impl Fn([u8; 3]) -> Message + 'a) -> Element<'a, Message> {
    let k = ui::fade();
    row(SWATCHES.iter().map(|colour| {
        let lit = chosen == Some(*colour);
        let fill = Color::from_rgb8(colour[0], colour[1], colour[2]);
        button(container(text("")).width(22.0).height(22.0))
            .padding(0)
            .style(move |_: &iced::Theme, status: button::Status| {
                let hot = lit || matches!(status, button::Status::Hovered | button::Status::Pressed);
                button::Style { background: Some(Background::Color(Color { a: k, ..fill })), text_color: INK, border: Border { color: crate::theme::outline(if hot { Color { a: k, ..INK } } else { Color::TRANSPARENT }), width: 2.0, radius: 11.0.into() }, shadow: iced::Shadow::default(), snap: true }
            })
            .on_press(press(*colour))
            .into()
    }))
    .spacing(6)
    .align_y(iced::Center)
    .into()
}

pub(super) fn categories_panel<'a>(pool: &'a Pool, editor: &'a Editor, words: &'a Words) -> Element<'a, Message> {
    let mut names = category_order(pool);
    for name in pool.categories.keys() {
        if !names.contains(name) { names.push(name.clone()); }
    }
    let present = category_order(pool);
    let mut lines = column![].spacing(8);
    for name in names {
        let place = present.iter().position(|known| *known == name);
        let standard = Mod::ALL.iter().find(|mods| mods.code() == name).copied();
        if let Some(mods) = standard {
            let held = pool.slots.iter().filter(|slot| slot.category.is_empty() && slot.mods == mods).count();
            lines = lines.push(
                row![
                    container(text(name.clone()).font(theme::MONO_BOLD).size(14.0).color(ui::faded(mod_tint(mods)))).width(44.0),
                    mono(held.to_string(), 12.0, MUTED),
                    ui::grow(),
                    icon_button(Icon::Up, 13.0, MUTED, place.filter(|at| *at > 0).map(|_| Message::MoveCategory(name.clone(), false))),
                    icon_button(Icon::Down, 13.0, MUTED, place.filter(|at| at + 1 < present.len()).map(|_| Message::MoveCategory(name.clone(), true))),
                ]
                .spacing(12)
                .align_y(iced::Center),
            );
            continue;
        }
        let held = pool.slots.iter().filter(|slot| slot.category == name).count();
        let chosen = pool.categories.get(&name).copied();
        let tint = chosen.map_or(MUTED, |colour| Color::from_rgb8(colour[0], colour[1], colour[2]));
        let picked = name.clone();
        lines = lines.push(
            row![
                container(text(name.clone()).font(theme::MONO_BOLD).size(14.0).color(ui::faded(ui::mix(tint, Color::WHITE, 0.45)))).width(44.0),
                swatches(chosen, move |colour| Message::Tint(Some(picked.clone()), colour)),
                mono(held.to_string(), 12.0, MUTED),
                ui::grow(),
                icon_button(Icon::Up, 13.0, MUTED, place.filter(|at| *at > 0).map(|_| Message::MoveCategory(name.clone(), false))),
                icon_button(Icon::Down, 13.0, MUTED, place.filter(|at| at + 1 < present.len()).map(|_| Message::MoveCategory(name.clone(), true))),
                icon_button(Icon::Close, 13.0, WARNED, Some(Message::DropCategory(name.clone()))),
            ]
            .spacing(12)
            .align_y(iced::Center),
        );
    }
    let taken = Mod::ALL.iter().any(|mods| mods.code() == editor.category_draft) || pool.categories.contains_key(&editor.category_draft);
    let name = text_input(&words.t("pool-category-new"), &editor.category_draft).on_input(Message::CategoryDraft).on_submit(Message::NewCategory).font(theme::MONO).size(14.0).padding([7, 10]).style(theme::field_faded(ui::fade())).width(150.0);
    lines = lines.push(row![name, swatches(Some(editor.tint), |colour| Message::Tint(None, colour)), ui::grow(), outlined(words.t("pool-category-add"), (!editor.category_draft.is_empty() && !taken).then_some(Message::NewCategory))].spacing(12).align_y(iced::Center));
    column![ui::mono_small(words.t("pool-tool-categories").to_uppercase(), MUTED), lines].spacing(12).into()
}

pub(super) fn editor_tools<'a>(state: &'a State, editor: &'a Editor, pool: &'a Pool, words: &'a Words, shut: &[(&'static str, f32)]) -> Element<'a, Message> {
    let filled = pool.filled() > 0;
    let missing = state.missing(pool).len();
    let versions = state.versions.iter().filter(|held| held.id == pool.id).count();
    let mut tools: Vec<Element<'a, Message>> = Vec::new();
    if !pool.collection {
        tools.push(outlined(words.t("pool-tool-categories"), Some(Message::Categories(!editor.categories))));
        tools.push(outlined(words.t("pool-tool-backdrop"), filled.then_some(Message::BackdropPanel(!editor.backdrop))));
    }
    tools.push(outlined(words.t("pool-command-share"), filled.then_some(Message::Share(true))));
    if missing > 0 {
        tools.push(outlined(words.with("pool-tool-missing", &[("n", missing.to_string())]), Some(Message::FetchMissing)));
    }
    if versions > 0 {
        tools.push(outlined(words.t("pool-tool-history"), Some(Message::History(true))));
    }
    let line: Element<'a, Message> = row(tools).spacing(8).align_y(iced::Center).wrap().into();
    if editor.history {
        return ui::under_leaning(line, Some(palette(state, editor, pool, words)), Message::History(false), ui::Lean::Left(0.0), 8.0);
    }
    let made = || menu_box(categories_panel(pool, editor, words), 560.0, 16.0, 14.0, Color::from_rgb(0.071, 0.027, 0.039));
    let sheet = if pool.collection {
        None
    } else if editor.categories {
        Some(made())
    } else {
        leaving(shut, "categories").map(|k| ui::appearing(k * k, -10.0, made))
    };
    ui::under_leaning(line, sheet, Message::Categories(false), ui::Lean::Left(0.0), 8.0)
}

pub(super) fn editor_title<'a>(pool: &'a Pool, words: &'a Words) -> Element<'a, Message> {
    text_input(&words.t(if pool.collection { "collection-name-hint" } else { "pool-name-hint" }), &pool.name)
        .on_input(Message::Rename)
        .on_paste(|contents| Message::PasteInto(Input::Name, contents))
        .font(theme::SANS_SEMI)
        .size(22.0)
        .padding([2, 0])
        .style(ui::bare_input(ui::fade()))
        .width(Length::Fill)
        .into()
}

pub(super) fn credit_group<'a>(label: String, body: Element<'a, Message>) -> Element<'a, Message> {
    row![ui::mono_small(label.to_uppercase(), MUTED), body].spacing(10).align_y(iced::Center).into()
}

pub(super) fn editor_credits<'a>(state: &'a State, editor: &'a Editor, pool: &'a Pool, words: &'a Words) -> Option<Element<'a, Message>> {
    if pool.collection {
        return (!pool.compiler.is_empty()).then(|| credit_group(words.t("pool-publisher"), author_chip(state, &pool.compiler)));
    }
    let mut makers: Vec<Element<'a, Message>> = pool.authors.iter().map(|author| row![author_chip(state, author), icon_button(Icon::Close, 11.0, FAINT, Some(Message::DropAuthor(author.clone())))].spacing(0).align_y(iced::Center).into()).collect();
    if pool.saved.is_some() {
        makers = pool.authors.iter().map(|author| author_chip(state, author)).collect();
        if makers.is_empty() { return None; }
    } else {
        let field = text_input(&words.t("pool-author-add"), &editor.author_draft).on_input(Message::AuthorDraft).on_submit(Message::AddAuthor).font(theme::SANS).size(14.0).padding([5, 0]).style(ui::bare_input(ui::fade())).width(190.0);
        makers.push(field.into());
    }
    Some(credit_group(words.t("pool-mappoolers"), row(makers).spacing(6).align_y(iced::Center).wrap().into()))
}

pub fn profile_link(text: &str) -> Option<String> {
    let said = text.trim();
    if said.is_empty() || said.split_whitespace().count() != 1 {
        return None;
    }
    let bare = said.trim_start_matches("https://").trim_start_matches("http://");
    let path = bare.strip_prefix("osu.ppy.sh/")?;
    (path.starts_with("users/") || path.starts_with("u/")).then(|| bare.trim_end_matches('/').to_owned())
}

pub(super) fn backdrop_panel<'a>(state: &'a State, pool: &'a Pool, words: &'a Words, thumbs: &'a HashMap<String, image::Handle>) -> Element<'a, Message> {
    let k = ui::fade();
    let held = pool.backdrop.as_ref();
    let chosen = held.and_then(|backdrop| match &backdrop.from { pools::BackdropFrom::Map(hash) => Some(hash.as_str()), pools::BackdropFrom::File(_) => None });
    let own = held.is_some_and(|backdrop| matches!(backdrop.from, pools::BackdropFrom::File(_)));
    let pick = |inside: Element<'a, Message>, on: bool, press: Message| -> Element<'a, Message> {
        button(container(inside).center(Length::Fill))
            .padding(0)
            .width(Length::FillPortion(1))
            .height(44.0)
            .style(ui::button_faded(move |_: &iced::Theme, status: button::Status| {
                let hot = matches!(status, button::Status::Hovered | button::Status::Pressed);
                button::Style { background: Some(Background::Color(Color::from_rgb(0.051, 0.02, 0.031))), text_color: INK, border: Border { color: crate::theme::outline(if on { ACCENT } else { soft(if hot { 0.3 } else { 0.12 }, 1.0) }), width: if on { 2.0 } else { 1.0 }, radius: 8.0.into() }, shadow: iced::Shadow::default(), snap: true }
            }))
            .on_press(press)
            .into()
    };
    let mut seen = HashSet::new();
    let covers: Vec<&str> = pool.slots.iter().filter_map(|slot| slot.hash.as_deref()).filter(|hash| seen.insert(*hash)).take(4).collect();
    let mut tiles = row![pick(glyph(Icon::Close, 14.0, MUTED), held.is_none(), Message::BackdropFrom(None))].spacing(8);
    for hash in covers {
        tiles = tiles.push(pick(cover(thumbs, Some(hash), 62.0, 40.0, 6.0), chosen == Some(hash), Message::BackdropFrom(Some(hash.to_owned()))));
    }
    let figure = |value: &str, label: String| -> Element<'a, Message> { column![text(value.to_owned()).font(theme::MONO_BOLD).size(14.0).color(ui::faded(INK)), faded_text(label, 12.0, MUTED)].into() };
    let file = button(container(text(words.t("pool-backdrop-file")).font(theme::SANS_SEMI).size(theme::BODY).color(ui::faded(INK))).center_y(theme::CONTROL_HEIGHT - 2.0))
        .padding([0, 12])
        .style(ui::button_faded(move |_: &iced::Theme, status: button::Status| {
            let hot = matches!(status, button::Status::Hovered | button::Status::Pressed);
            button::Style { background: hot.then_some(Background::Color(soft(0.08, 1.0))), text_color: INK, border: Border { color: crate::theme::outline(if own { ACCENT } else { soft(0.12, 1.0) }), width: if own { 2.0 } else { 1.0 }, radius: 8.0.into() }, shadow: iced::Shadow::default(), snap: true }
        }))
        .on_press(Message::BackdropPick);
    let mut from = column![
        ui::mono_small(words.t("pool-backdrop-from").to_uppercase(), MUTED),
        tiles,
        row![file],
        row![figure("1920 × 1080", words.t("pool-backdrop-best")), figure("16:9", words.t("pool-backdrop-shape")), figure(&words.t("pool-backdrop-weight"), words.t("pool-backdrop-kinds"))].spacing(24),
        para(words.t("pool-backdrop-small"), 12.0, FAINT),
    ]
    .spacing(12);
    if state.backdrop_refused {
        from = from.push(para(words.t("pool-backdrop-refused"), 12.0, WARNED));
    }
    let dim = held.map_or(60, |backdrop| backdrop.dim);
    let live = held.is_some();
    let scale = iced::widget::slider(pools::Backdrop::LEAST_DIM..=pools::Backdrop::MOST_DIM, dim, Message::BackdropDim).step(5u8).height(24.0).style(move |_: &iced::Theme, _| iced::widget::slider::Style {
        rail: iced::widget::slider::Rail { backgrounds: (Background::Color(Color { a: k, ..ACCENT }), Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.82 * k))), width: 4.0, border: Border { radius: 2.0.into(), ..Border::default() } },
        handle: iced::widget::slider::Handle { shape: iced::widget::slider::HandleShape::Circle { radius: 8.0 }, background: Background::Color(Color { a: k, ..ACCENT }), border_width: 0.0, border_color: Color::TRANSPARENT },
    });
    let tick = |on: bool, label: String, press: Message| -> Element<'a, Message> {
        button(row![check_box(on), text(label).font(theme::SANS).size(14.0).color(ui::faded(INK))].spacing(12).align_y(iced::Center)).padding(0).style(ui::button_faded(theme::bare)).on_press_maybe(live.then_some(press)).into()
    };
    let inside = column![
        row![semi(words.t("pool-backdrop-title"), 16.0, INK), ui::grow(), mono(words.t("pool-backdrop-optional"), 12.0, MUTED)].align_y(iced::Center),
        from,
        column![row![text(words.t("pool-backdrop-dim")).font(theme::SANS).size(14.0).color(ui::faded(INK)), ui::grow(), text(format!("{dim}%")).font(theme::MONO_BOLD).size(14.0).color(ui::faded(INK))].align_y(iced::Center), scale].spacing(8),
        tick(held.is_some_and(|backdrop| backdrop.blur), words.t("pool-backdrop-blur"), Message::BackdropBlur(!held.is_some_and(|backdrop| backdrop.blur))),
        tick(held.is_some_and(|backdrop| backdrop.on_card), words.t("pool-backdrop-card"), Message::BackdropCard(!held.is_some_and(|backdrop| backdrop.on_card))),
        para(words.t("pool-backdrop-note"), 12.0, FAINT),
        row![primary_button(words.t("pool-backdrop-done"), Message::BackdropPanel(false)), outlined(words.t("pool-backdrop-drop"), live.then_some(Message::BackdropFrom(None)))].spacing(8),
    ]
    .spacing(20);
    container(inside).padding(24).width(400.0).style(move |_| container::Style {
        background: Some(Background::Color(Color { a: k, ..Color::from_rgb(0.071, 0.027, 0.039) })),
        border: Border { color: crate::theme::outline(soft(0.08, k)), width: 1.0, radius: 14.0.into() },
        ..container::Style::default()
    }).into()
}

pub(super) fn stray_card<'a>(link: &'a str, words: &'a Words) -> Element<'a, Message> {
    let k = ui::fade();
    let inside = row![
        icon_button(Icon::Close, 14.0, WARNED, Some(Message::DropStray)),
        column![text(words.t("pool-stray-profile")).font(theme::SANS).size(15.0).color(ui::faded(WARNED)), text(link).font(theme::MONO).size(13.0).color(ui::faded(INK))].spacing(4).width(Length::Fill),
        text(words.t("pool-stray-fits")).font(theme::SANS).size(15.0).color(ui::faded(MUTED)),
    ]
    .spacing(14)
    .align_y(iced::Center);
    container(inside).padding([18, 24]).width(Length::Fill).style(move |_| container::Style {
        background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.22 * k))),
        border: Border { color: crate::theme::outline(soft(0.08, k)), width: 1.0, radius: 14.0.into() },
        ..container::Style::default()
    }).into()
}

pub(super) fn catalogue_toast<'a>(error: &str, words: &'a Words) -> Element<'a, Message> {
    let k = ui::fade();
    let said = words.t(match error { "401" => "catalogue-sign-in", "403" => "catalogue-profile-required", "400" => "catalogue-invalid", _ => "catalogue-unavailable-now" });
    let inside = row![
        glyph(Icon::Close, 14.0, WARNED),
        column![text(said).font(theme::SANS).size(15.0).color(ui::faded(WARNED)), text(words.t("catalogue-draft-kept")).font(theme::SANS).size(13.0).color(ui::faded(MUTED))].spacing(4).width(Length::Fill),
        outlined(words.t("catalogue-retry"), Some(Message::RetryCatalogue)),
    ]
    .spacing(14)
    .align_y(iced::Center);
    iced::widget::opaque(container(inside).padding([18, 18]).width(400.0).style(move |_| container::Style {
        background: Some(Background::Color(Color { a: k, ..Color::from_rgb(0.114, 0.043, 0.059) })),
        border: Border { color: crate::theme::outline(Color::from_rgba(0.886, 0.282, 0.282, 0.4 * k)), width: 1.0, radius: 14.0.into() },
        ..container::Style::default()
    }))
}

pub fn map_link(file: &std::path::Path) -> Option<String> {
    map_id(file).map(|id| format!("https://osu.ppy.sh/b/{id}"))
}

pub fn map_id(file: &std::path::Path) -> Option<u64> {
    use std::io::{BufRead, BufReader};
    let lines = BufReader::new(std::fs::File::open(file).ok()?).lines().take(120);
    for line in lines.map_while(Result::ok) {
        if let Some(said) = line.trim().strip_prefix("BeatmapID:") {
            return said.trim().parse::<u64>().ok().filter(|id| *id > 0);
        }
    }
    None
}

pub(super) fn mod_tint(mods: Mod) -> Color {
    let base = slot_colour(&Slot::empty(mods));
    Color { r: base.r + (1.0 - base.r) * 0.22, g: base.g + (1.0 - base.g) * 0.22, b: base.b + (1.0 - base.b) * 0.22, a: 1.0 }
}

pub(super) fn frame_tags<'a>(frame: Frame, words: &Words) -> Element<'a, Message> {
    let k = ui::fade();
    let tag = move |label: String, ink: Color, fill: Color| -> Element<'a, Message> {
        container(text(label).font(theme::MONO_BOLD).size(11.5).color(Color { a: ink.a * k, ..ink }))
            .padding([4, 8])
            .style(move |_| container::Style { background: Some(Background::Color(Color { a: fill.a * k, ..fill })), border: Border { radius: 6.0.into(), ..Border::default() }, ..container::Style::default() })
            .into()
    };
    let mut counted: Vec<(Mod, usize)> = Vec::new();
    for mods in frame.mods() {
        match counted.last_mut() {
            Some((last, count)) if *last == mods => *count += 1,
            _ => counted.push((mods, 1)),
        }
    }
    if counted.is_empty() {
        return ui::dashed(container(text(words.t("pool-frame-own")).font(theme::MONO_BOLD).size(11.5).color(ui::faded(MUTED))).padding([4, 8]), 6.0, soft(0.2, 1.0));
    }
    row(counted.into_iter().map(|(mods, count)| {
        let base = slot_colour(&Slot::empty(mods));
        let label = if count > 1 { format!("{} {count}", mods.code()) } else { mods.code().to_owned() };
        tag(label, mod_tint(mods), Color { a: 0.22, ..base })
    }).collect::<Vec<_>>())
    .spacing(6)
    .wrap()
    .into()
}

pub(super) fn start_tile(lit: bool) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_, status| {
        let hot = matches!(status, button::Status::Hovered | button::Status::Pressed);
        let (fill, edge) = match (lit, hot) {
            (true, _) => (tinted(ACCENT, 0.16, 1.0), tinted(ACCENT, 0.4, 1.0)),
            (false, true) => (soft(0.07, 1.0), soft(0.16, 1.0)),
            (false, false) => (soft(0.045, 1.0), soft(0.08, 1.0)),
        };
        button::Style { background: Some(Background::Color(fill)), text_color: INK, border: Border { color: crate::theme::outline(edge), width: 1.0, radius: 12.0.into() }, shadow: iced::Shadow::default(), snap: true }
    }
}

pub(super) const START_WIDE: f32 = 1290.0;
pub(super) const START_TALL: f32 = 640.0;
pub(super) const START_GROWTH: f32 = 1.4;

pub(super) const START_SMALLER: f32 = if cfg!(test) { 1.0 } else { 0.86 };

pub(super) fn start_scale(width: f32) -> f32 {
    (width / START_WIDE * START_SMALLER).clamp(START_SMALLER, START_GROWTH * START_SMALLER)
}

pub(super) fn start_view<'a>(state: &'a State, editor: &'a Editor, pool: &'a Pool, words: &'a Words, width: f32, high: f32, t: f32) -> Element<'a, Message> {
    let grown = start_scale(width);
    let width = width / grown;
    let caption = |key: &str| ui::mono_small(words.t(key).to_uppercase(), MUTED);
    let frames = [(Frame::Duel, Icon::Swords, "pool-start-duel", false), (Frame::Stage, Icon::Trophy, "pool-start-stage", true), (Frame::Free, Icon::Shapes, "pool-frame-free", true)];
    let narrow = width < 900.0;
    let frame_high = if narrow { 112.0 } else { 168.0 };
    let frame_tile = |frame: Frame, icon: Icon, key: &str, offered: bool| -> Element<'a, Message> {
        let build = || -> Element<'a, Message> {
            let mut name = row![semi(words.t(key), 16.0, if offered { INK } else { MUTED })].spacing(8).align_y(iced::Center);
            if !offered {
                let k = ui::fade();
                name = name.push(container(mono(words.t("pool-frame-soon"), 11.0, MUTED)).padding([1, 7]).style(move |_| container::Style { border: Border { color: crate::theme::outline(soft(0.16, k)), width: 1.0, radius: 9.0.into() }, ..container::Style::default() }));
            }
            let inside = row![glyph(icon, 32.0, if offered { INK } else { FAINT }), column![name, frame_tags(frame, words)].spacing(10).width(Length::Fill)].spacing(16);
            let tile = button(container(inside).padding(16).width(Length::Fill).height(Length::Fill)).padding(0).width(Length::Fill).height(frame_high).style(ui::button_faded(start_tile(offered && pool.frame == frame)));
            if offered { tile.on_press(Message::UseFrame(frame)).into() } else { tile.into() }
        };
        if offered { build() } else { ui::fading(ui::fade() * 0.6, build) }
    };
    let lined = |tiles: Vec<Element<'a, Message>>, across: usize| -> Element<'a, Message> {
        let mut rows = column![].spacing(16);
        let mut tiles = tiles.into_iter().peekable();
        while tiles.peek().is_some() {
            let mut line = row![].spacing(16);
            for at in 0..across {
                line = match tiles.next() {
                    Some(tile) => line.push(container(tile).width(Length::FillPortion(1))),
                    None if at > 0 => line.push(Space::new().width(Length::FillPortion(1))),
                    None => line,
                };
            }
            rows = rows.push(line);
        }
        rows.into()
    };
    let frame_tiles: Vec<Element<'a, Message>> = frames.iter().map(|(frame, icon, key, offered)| frame_tile(*frame, *icon, key, *offered)).collect();
    let foot = row![faded_text(words.t("pool-start-note"), 14.0, MUTED), ui::grow(), primary_button(words.t("pool-start-go"), Message::Begin(Start::Empty))].spacing(12).align_y(iced::Center);
    let page = column![
        ui::appearing(ui::appear(t, 0), 0.0, || editor_toolbar(state, editor, pool, words, width)),
        ui::appearing(ui::appear(t, 1), 0.0, || {
            column![caption("pool-start-name"), editor_title(pool, words)].spacing(8).into()
        }),
        ui::appearing(ui::appear(t, 2), 0.0, || column![caption("pool-start-frame"), lined(frame_tiles, if narrow { 1 } else { 3 })].spacing(12).into()),
    ]
    .spacing(24);
    let tall = (high - 40.0) / grown;
    let foot = ui::appearing(ui::appear(t, 4), 0.0, || foot.into());
    let page: Element<'a, Message> = if !narrow && tall >= START_TALL {
        container(column![page, Space::new().height(Length::Fill), foot]).height(tall).into()
    } else {
        column![page, foot].spacing(24).into()
    };
    ui::scaled(page, grown).into()
}

pub(super) fn editor_view<'a>(state: &'a State, editor: &'a Editor, pool: &'a Pool, words: &'a Words, thumbs: &'a HashMap<String, image::Handle>, width: f32, high: f32, t: f32, shut: &[(&'static str, f32)], backdrop_age: f32, gone_age: f32) -> Element<'a, Message> {
    if editor.starting {
        return start_view(state, editor, pool, words, width, high, t);
    }
    let mut head = column![
        ui::appearing(ui::appear(t, 0), 0.0, || editor_toolbar(state, editor, pool, words, width)),
        ui::appearing(ui::appear(t, 1), 0.0, || if pool.saved.is_some() { Space::new().into() } else { editor_tools(state, editor, pool, words, shut) }),
        ui::appearing(ui::appear(t, 1), 0.0, || editor_title(pool, words)),
    ]
    .spacing(14);

    if let Some(credits) = ui::fading(ui::fade() * ui::appear(t, 2), || editor_credits(state, editor, pool, words)) {
        head = head.push(ui::lifted(credits, ui::appear(t, 2), 0.0));
    }
    if state.conflict.as_deref() == Some(pool.id.as_str()) {
        head = head.push(ui::appearing(ui::appear(t, 3), 0.0, || conflict_banner(state, pool, words, width)));
    }

    if state.uncollect_refused {
        head = head.push(text(words.t("collection-delete-running")).size(13).color(WARNED));
    }
    if let Some(why) = &state.delete_error {
        head = head.push(column![text(words.t("pool-delete-failed")).size(13).color(WARNED), text(why).size(12).color(MUTED)].spacing(4));
    }
    let songs = state.songs.as_ref();
    let beside = width >= PANEL_BESIDE;
    let backdrop_out = leaving(shut, "backdrop").filter(|_| !editor.backdrop);
    let panelled = (editor.backdrop || backdrop_out.is_some()) && !pool.collection;
    let board_wide = if beside && panelled { width - BACKDROP_WIDE - PANEL_GAP } else { width };
    let columns = ((board_wide + TABLE_GAP) / (TABLE_LEAST + TABLE_GAP)).floor().clamp(1.0, 3.0) as usize;
    let step = if pool.collection || editor.choosing { None } else { state.guide.current(pool) };
    let pointed = match step {
        Some(1) => pool.first_empty(),
        Some(2..=4) => pool.slots.iter().position(|slot| !slot.is_empty()),
        _ => None,
    };
    let loose = step.filter(|step| matches!(step, 1..=4) && pointed.is_none());
    if let Some(step) = step.filter(|step| *step == 0).or(loose) {
        head = head.push(ui::appearing(ui::appear(t, 3), 0.0, || coach(step, words)));
    }
    if let Some(step) = step.filter(|step| *step >= 5) {
        head = head.push(ui::appearing(ui::appear(t, 3), 0.0, || container(coach(step, words)).align_right(Length::Fill).into()));
    }
    let downloaded = match &state.finding {
        Some(Finding::Fetching(Fetching { step: Some(crate::maps::Step::Downloading { done, total, .. }), .. })) => Some((*done, *total)),
        _ => None,
    };
    let keys = slot_keys(pool);
    let board = |slots: Vec<usize>, group: Option<Mod>, across: usize| -> Element<'a, Message> {
        let pieces = slots.iter().map(|at| {
            let lit = if editor.choosing { editor.marked.contains(at) } else { editor.selected == Some(*at) };
            let shown = slot_found(&pool.slots[*at], &editor.find) && (!editor.only_empty || pool.slots[*at].is_empty());
            let dim = if shown { 1.0 } else { 0.28 };
            let adding = editor.panel == Panel::Add && editor.selected == Some(*at) && !editor.choosing;
            let left = editor.gone.as_deref().filter(|was| !adding && editor.panel != Panel::Add && was.panel == Panel::Add && was.selected == Some(*at) && gone_age < SHEET_OUT);
            let made: Element<'a, Message> = if adding {
                ui::under_staying(add_row(pool, *at, editor, words), Some(add_drop(state, editor, pool, words, thumbs)), ui::Lean::Left(0.0), 6.0)
            } else {
                ui::appearing(ui::appear(t, *at + 4) * dim, 0.0, || slot_row(pool, *at, lit, editor.choosing, words, thumbs, songs, state.ail(&pool.slots[*at]), pointed == Some(*at) && step == Some(2), downloaded))
            };
            let made: Element<'a, Message> = match left {
                Some(was) => {
                    let k = 1.0 - gone_age / SHEET_OUT;
                    ui::under_staying(made, Some(ui::appearing(k * k, -10.0, || add_drop(state, was, pool, words, thumbs))), ui::Lean::Left(0.0), 6.0)
                }
                None => made,
            };
            let made = ui::under_leaning(made, (editor.menu == Some(*at)).then(|| slot_menu(*at, words)), Message::SlotMenu(None), ui::Lean::Left(260.0), 4.0);
            let made: Element<'a, Message> = match step.filter(|_| pointed == Some(*at) && !adding) {
                Some(step) => column![made, coach(step, words)].spacing(8).into(),
                None => made,
            };
            (keys[*at], made)
        }).collect();
        let next = group.and_then(|_| slots.last()).and_then(|last| keys.get(last + 1)).copied();
        let snapshot = keys.clone();
        crate::board::board(pieces, 8.0, move |key, before| Message::Move(pool.id.clone(), snapshot.clone(), key, before.or(next)))
            .identity(format!("{}:{}:{:?}", pool.id, group.map_or("order", Mod::code), slots.first()))
            .across(across)
            .centred()
            .anywhere()
            .grab_from(0.0)
            .grab_high(ROW_HIGH + 16.0)
            .radius(TILE_ROUND)
            .solid(theme::GROUND)
            .into()
    };
    let list: Element<'a, Message> = if pool.slots.is_empty() {
        ui::appearing(ui::appear(t, 4), 0.0, || container(faded_text(words.t("pool-free-empty"), 14.0, MUTED)).padding([24, 10]).into())
    } else if !pool.collection {
        let mut custom = std::collections::BTreeMap::<&str, Vec<usize>>::new();
        for (at, slot) in pool.slots.iter().enumerate() { custom.entry(if slot.category.is_empty() { slot.mods.code() } else { &slot.category }).or_default().push(at); }
        let order = category_order(pool);
        let mut blocks: Vec<Element<'a, Message>> = Vec::new();
        for name in order.iter() {
            let Some(slots) = custom.remove(name.as_str()) else { continue };
            let mods = pool.slots[slots[0]].mods;
            blocks.push(board(slots, Some(mods), columns));
        }
        if pool.saved.is_none() && !editor.choosing {
            blocks.push(ui::appearing(ui::appear(t, pool.slots.len() + 4), 0.0, || {
                let k = ui::fade();
                let more = button(container(glyph(Icon::Plus, 16.0, MUTED)).center_x(Length::Fill).center_y(ROW_HIGH))
                    .padding(0)
                    .width(180.0)
                    .style(ui::button_faded(move |_: &iced::Theme, status: button::Status| {
                        let hot = matches!(status, button::Status::Hovered | button::Status::Pressed);
                        button::Style { background: Some(Background::Color(soft(if hot { 0.08 } else { 0.03 }, k))), text_color: INK, border: Border { color: crate::theme::outline(soft(0.14, k)), width: 1.0, radius: TILE_ROUND.into() }, shadow: iced::Shadow::default(), snap: true }
                    }))
                    .on_press(Message::AddPanel(true));
                container(more).center_x(Length::Fill).into()
            }));
        }
        column(blocks).spacing(18).into()
    } else {
        board((0..pool.slots.len()).collect(), None, columns)
    };

    let list: Element<'a, Message> = match &state.stray {
        Some(link) => column![list, stray_card(link, words)].spacing(24).into(),
        None => list,
    };
    if !panelled {
        return column![head, list].spacing(22).into();
    }
    let panel = match backdrop_out {
        Some(k) => ui::appearing(k * k, -10.0, || backdrop_panel(state, pool, words, thumbs)),
        None => ui::appearing(ui::appear(t, 3).min(ui::appear(backdrop_age, 0)), -10.0, || backdrop_panel(state, pool, words, thumbs)),
    };
    if beside {
        return column![head, row![container(list).width(Length::Fill), container(panel).width(BACKDROP_WIDE)].spacing(PANEL_GAP)].spacing(22).into();
    }
    column![head, panel, list].spacing(22).into()
}

