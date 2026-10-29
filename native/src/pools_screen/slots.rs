use super::*;
use iced::widget::column;

pub(super) fn check_box<'a>(on: bool) -> Element<'a, Message> {
    let k = ui::fade();
    let inside: Element<'a, Message> = if on { glyph(Icon::Check, 13.0, Color::WHITE) } else { Space::new().into() };
    container(inside)
        .width(22.0)
        .height(22.0)
        .center(22.0)
        .style(move |_| container::Style {
            background: on.then_some(Background::Color(Color { a: k, ..ACCENT })),
            border: Border { color: crate::theme::outline(if on { Color { a: k, ..ACCENT } } else { soft(0.16, k) }), width: 1.0, radius: 6.0.into() },
            ..container::Style::default()
        })
        .into()
}

pub(super) fn slot_row<'a>(pool: &'a Pool, at: usize, selected: bool, choosing: bool, words: &'a Words, thumbs: &'a HashMap<String, image::Handle>, songs: Option<&Arc<HashMap<String, Map>>>, ail: Ail, pointed: bool, bytes: Option<(u64, Option<u64>)>) -> Element<'a, Message> {
    let slot = &pool.slots[at];
    let mut left = row![].spacing(12).align_y(iced::Center);
    if choosing {
        left = left.push(check_box(selected));
    }
    left = left.push(number_badge(category_number(pool, at), selected));
    if !pool.collection {
        left = left.push(if choosing { mod_badge(slot.mods, selected) } else { category_menu(pool, at, pointed) });
    }
    let left: Element<'a, Message> = left.into();
    let inside: Element<'a, Message> = match slot.hash.as_deref() {
        None => row![left, hatched_cover(), container(faded_text(words.t("pool-slot-empty"), 14.0, MUTED)).width(Length::Fill)].spacing(12).align_y(iced::Center).into(),
        Some(hash) => {
            let _ = songs;
            let trouble = |key: &str| -> Element<'a, Message> { row![glyph(Icon::Close, 11.0, WARNED), faded_text(words.t(key), 12.0, WARNED)].spacing(6).align_y(iced::Center).into() };
            let under: Element<'a, Message> = match (ail, slot.measure.is_some()) {
                (Ail::Missing, _) => trouble("pool-no-songs"),
                (Ail::Unmeasured, _) => trouble("pool-unmeasured"),
                (Ail::Fetching, _) => {
                    let said = match bytes {
                        Some((done, Some(total))) if total > 0 => words.with("pool-fetch-size", &[("done", megabytes(words, done)), ("total", megabytes(words, total))]),
                        _ => words.t("pool-from-mirror"),
                    };
                    row![ui::dot(8.0), faded_text(said, 12.0, INK)].spacing(6).align_y(iced::Center).into()
                }
                (Ail::Well, false) => faded_text(words.t("pool-measuring"), 12.0, FAINT),
                (Ail::Well, true) => ui::moving_text(slot.artist.clone(), theme::SANS, 12.0, MUTED),
            };
            let details = column![ui::moving_text(slot.title.clone(), theme::SANS_SEMI, 14.0, INK), under].spacing(2).width(Length::Fill);
            let stars: Element<'a, Message> = slot.measure.map(|measure| crate::dossier::star_pill(stars_of(words, measure.stars), measure.stars as f32, 15.0)).unwrap_or_else(|| Space::new().into());
            let picture = if thumbs.contains_key(hash) { cover(thumbs, Some(hash), COVER_WIDE, COVER_HIGH, COVER_ROUND) } else { hatched_cover() };
            let room = match (ail, choosing) {
                (Ail::Missing | Ail::Unmeasured, false) => 110.0,
                (Ail::Fetching, false) => 36.0,
                _ => 0.0,
            };
            row![left, picture, container(details).width(Length::Fill).clip(true), stars, Space::new().width(room)].spacing(12).align_y(iced::Center).into()
        }
    };
    let colour = if pool.collection { Color::from_rgb8(95, 95, 95) } else { slot_colour(slot) };
    let empty = slot.is_empty();
    let backed = pool.backdrop.is_some();
    let row_button = button(container(inside).height(ROW_HIGH).align_y(iced::Center))
        .padding([0, 12])
        .width(Length::Fill)
        .style(ui::button_faded(move |theme, status| {
            let mut style = row_style(selected)(theme, status);
            let hot = matches!(status, button::Status::Hovered | button::Status::Pressed);
            style.background = backed.then_some(Background::Color(Color { a: 0.82 * ui::fade(), ..theme::GROUND }));
            style.border.color = ui::faded(if selected || hot { colour } else { ui::mix(colour, theme::GROUND, 0.25) });
            style.border.width = if selected { 2.0 } else if empty && !hot { 0.0 } else { 1.0 };
            style
        }))
        .on_press(if choosing { Message::Mark(at) } else { Message::Select(Some(at)) });
    let row_button: Element<'a, Message> = if empty && !selected {
        ui::dashed(row_button, TILE_ROUND, soft(0.16, 1.0))
    } else if choosing || empty {
        row_button.into()
    } else {
        iced::widget::mouse_area(row_button).on_right_press(Message::SlotMenu(Some(at))).into()
    };
    let action: Option<Element<'a, Message>> = match (ail, choosing) {
        (Ail::Missing, false) => Some(outlined(words.t("pool-fetch-short"), Some(Message::FetchSlot(at)))),
        (Ail::Unmeasured, false) => Some(outlined(words.t("pool-fetch-short"), Some(Message::FetchSlot(at)))),
        (Ail::Fetching, false) => Some(icon_button(Icon::Close, 13.0, MUTED, Some(Message::Dismiss))),
        _ if empty && !choosing && !selected && pool.saved.is_none() => Some(icon_button(Icon::Close, 12.0, FAINT, Some(Message::Clear(at)))),
        _ => None,
    };
    match action {
        Some(action) => iced::widget::stack![row_button, container(action).align_right(Length::Fill).center_y(ROW_HIGH).padding(Padding::ZERO.right(14.0))].into(),
        None => row_button.into(),
    }
}

pub(super) fn category_menu<'a>(pool: &Pool, at: usize, lit: bool) -> Element<'a, Message> {
    let length = pool.slots.len();
    let mut categories: Vec<String> = Mod::ALL.iter().map(|mods| mods.code().to_owned()).chain(pool.categories.keys().cloned()).chain(pool.slots.iter().filter(|slot| !slot.category.is_empty()).map(|slot| slot.category.clone())).collect();
    categories.sort();
    categories.dedup();
    let selected = pool.slots.get(at).map(|slot| if slot.category.is_empty() { slot.mods.code().to_owned() } else { slot.category.clone() }).unwrap_or_else(|| "NM".into());
    let wide = (ui::text_width(&selected, theme::MONO_BOLD, 12.0) + 18.0).ceil();
    let base = pool.slots.get(at).map_or(slot_colour(&Slot::empty(Mod::Nm)), slot_colour);
    let (k, ink, muted) = (ui::fade(), ui::faded(ui::mix(base, Color::WHITE, 0.45)), ui::faded(MUTED));
    pick_list(categories, Some(selected), move |name| {
        match Mod::ALL.into_iter().find(|mods| mods.code() == name) {
            Some(mods) if at < length => Message::SetMod(at, mods),
            Some(mods) => Message::AddMod(mods),
            None => Message::Category(at, name),
        }
    }).font(theme::MONO_BOLD).text_size(12.0).padding([4, 8]).handle(pick_list::Handle::None).width(wide)
        .menu_style(|_| iced::widget::overlay::menu::Style { background: Background::Color(theme::GROUND), border: Border { color: crate::theme::outline(MUTED), width: 1.0, radius: 7.0.into() }, text_color: INK, selected_text_color: INK, selected_background: Background::Color(Color::from_rgb8(65, 30, 32)), shadow: iced::Shadow::default() })
        .style(move |_, status| pick_list::Style { text_color: ink, placeholder_color: muted, handle_color: muted, background: Background::Color(tinted(base, if matches!(status, pick_list::Status::Hovered | pick_list::Status::Opened { .. }) { 0.28 } else { 0.16 }, k)), border: Border { color: crate::theme::outline(if lit { Color { a: k, ..ACCENT } } else { Color::TRANSPARENT }), width: if lit { 2.0 } else { 0.0 }, radius: 6.0.into() } }).into()
}

pub(super) fn figure<'a>(label: String, value: String, size: f32, colour: Color) -> Element<'a, Message> {
    column![ui::label(label, FAINT), text(value).font(theme::MONO_BOLD).size(size).wrapping(text::Wrapping::None).color(ui::faded(colour))].spacing(4).into()
}

pub(super) fn profile_bars<'a>(measure: &Measure, words: &Words) -> Element<'a, Message> {
    let k = ui::fade();
    let percents = measure.percents();
    let mut list = column![ui::label(words.t("pool-slot-profile"), FAINT)].spacing(10);
    for (at, skill) in Skill::ALL.into_iter().enumerate() {
        let filled = percents[at].clamp(0, 100) as u16;
        let track = container(
            row![
                container(Space::new().height(5.0)).width(Length::FillPortion(filled.max(1))).style(move |_| container::Style { background: Some(Background::Color(Color { a: k, ..ACCENT })), border: Border { radius: 2.5.into(), ..Border::default() }, ..container::Style::default() }),
                Space::new().width(Length::FillPortion((100 - filled).max(1))),
            ],
        )
        .width(Length::Fill)
        .style(move |_| container::Style { background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.03 * k))), border: Border { radius: 2.5.into(), ..Border::default() }, ..container::Style::default() });
        list = list.push(
            row![
                container(faded_text(words.t(skill.key()), 13.0, MUTED)).width(110.0),
                track,
                container(mono(format!("{}%", percents[at]), 12.0, MUTED)).width(40.0).align_x(iced::alignment::Horizontal::Right),
            ]
            .spacing(12)
            .align_y(iced::Center),
        );
    }
    list.into()
}

pub(super) fn category_fields<'a>(pool: &'a Pool, editor: &'a Editor, at: usize, words: &'a Words) -> Element<'a, Message> {
    if pool.collection { return Space::new().into(); }
    if !editor.category_creator { return quiet_button(words.t("pool-create-category"), Message::CreateCategory(true)); }
    let name = text_input(&words.t("pool-custom-category"), &editor.category_draft).on_input(Message::CategoryDraft).font(theme::SANS).size(12.0).padding([7, 10]).style(theme::field_faded(ui::fade()));
    let mut fields = column![name].spacing(8);
    fields = fields.push(row![ui::label(words.t("pool-category-colour"), MUTED), text_input("#5ec2d0", &editor.colour_draft).on_input(move |value| Message::Colour(at, value)).font(theme::MONO).size(12.0).padding([7, 10]).style(theme::field_faded(ui::fade())).width(120.0)].spacing(12).align_y(iced::Center));
    let valid = !editor.category_draft.is_empty() && !Mod::ALL.iter().any(|m| m.code() == editor.category_draft) && parse_colour(&editor.colour_draft).is_some();
    fields = fields.push(row![ui::primary(words.t("pool-create-category"), valid.then_some(Message::SaveCategory(at))), quiet_button(words.t("pool-delete-no"), Message::CreateCategory(false))].spacing(8));
    fields.into()
}

pub(super) fn slot_panel<'a>(pool: &'a Pool, editor: &'a Editor, at: usize, wide: f32, words: &'a Words, thumbs: &'a HashMap<String, image::Handle>, songs: Option<&Arc<HashMap<String, Map>>>) -> Element<'a, Message> {
    let slot = &pool.slots[at];
    let close = icon_button(Icon::Close, 14.0, FAINT, Some(Message::Select(None)));
    let category = if pool.collection { Space::new().into() } else { category_menu(pool, at, false) };
    let mut body = column![row![number_badge(category_number(pool, at), true), category, ui::grow(), close].spacing(10).align_y(iced::Center)].spacing(16);
    body = body.push(category_fields(pool, editor, at, words));
    match slot.hash.as_deref() {
        None => {
            body = body.push(semi(words.t("pool-slot-empty"), 18.0, INK));
            body = body.push(faded_text(words.t("pool-slot-need"), 13.0, MUTED));
            body = body.push(primary_button(words.t("pool-add"), Message::AddPanel(true)));
        }
        Some(hash) => {
            body = body.push(cover(thumbs, Some(hash), wide - 32.0, 150.0, HERO_ROUND));
            body = body.push(column![wrapped_semi(slot.title.clone(), 20.0, INK), ui::moving_text(slot.artist.clone(), theme::SANS, 14.0, MUTED), ui::moving_text(slot.version.clone(), theme::SANS, 12.0, FAINT)].spacing(3).width(Length::Fill));
            match slot.measure {
                Some(measure) => {
                    body = body.push(
                        row![
                            container(figure(words.t("pool-stars"), stars_of(words, measure.stars), 26.0, crate::dossier::star_colour(measure.stars as f32))).width(Length::Fill),
                            container(figure(words.t("pool-bpm"), format!("{:.0}", measure.bpm), 18.0, INK)).width(Length::Fill),
                            container(figure(words.t("pool-length"), pools::clock(measure.length_ms), 18.0, INK)).width(Length::Fill),
                            container(figure(words.t("pool-combo"), measure.max_combo.to_string(), 18.0, INK)).width(Length::Fill),
                        ]
                        .spacing(12)
                        .align_y(iced::alignment::Vertical::Bottom),
                    );
                    let one = |value: f64| screen::decimal(words, value as f32, 1);
                    body = body.push(
                        row![
                            container(figure("AR".to_owned(), one(measure.ar), 16.0, INK)).width(Length::Fill),
                            container(figure("OD".to_owned(), one(measure.od), 16.0, INK)).width(Length::Fill),
                            container(figure("CS".to_owned(), one(measure.cs), 16.0, INK)).width(Length::Fill),
                            container(figure("HP".to_owned(), one(measure.hp), 16.0, INK)).width(Length::Fill),
                        ]
                        .spacing(12),
                    );
                    body = body.push(profile_bars(&measure, words));
                }
                None if songs.is_some_and(|songs| !songs.contains_key(hash)) => body = body.push(faded_text(words.t("pool-no-disk"), 12.0, ACCENT)),
                None => body = body.push(faded_text(words.t("pool-measuring"), 12.0, FAINT)),
            }
        }
    }
    if !slot.is_empty() {
        body = body.push(
            column![
                ui::label(words.t("pool-slot-note"), FAINT),
                text_input("", &slot.note).on_input(move |note| Message::Note(at, note)).on_paste(move |contents| Message::PasteInto(Input::Note(at), contents)).font(theme::SANS).size(13.0).padding([8, 12]).style(theme::field_faded(ui::fade())),
            ]
            .spacing(8),
        );
        body = body.push(
            row![
                quiet_button(words.t("pool-replace"), Message::Replace),
                plain_button(words.t("pool-remove"), Message::Clear(at)),
            ]
            .spacing(8)
            .align_y(iced::Center),
        );
    }
    panel_box(body.into())
}

pub(super) fn finding_card<'a>(finding: &'a Finding, editor: &'a Editor, pool: &'a Pool, words: &'a Words, thumbs: &'a HashMap<String, image::Handle>) -> Element<'a, Message> {
    match finding {
        Finding::Asking => mono(words.t("pool-link-asking"), 12.0, FAINT),
        Finding::Refused(mode) => {
            let inside = column![
                row![glyph(Icon::Warn, 16.0, ACCENT), semi(words.with("pool-mode-title", &[("mode", pool_links::mode_name(mode).to_owned())]), 14.5, INK)].spacing(10).align_y(iced::Center),
                para(words.t("pool-mode-text"), 13.0, MUTED),
                quiet_button(words.t("pool-got-it"), Message::Dismiss),
            ]
            .spacing(10);
            panel_box(inside.into())
        }
        Finding::Missing => {
            let inside = column![row![glyph(Icon::Warn, 16.0, ACCENT), semi(words.t("pool-missing"), 14.0, INK)].spacing(10).align_y(iced::Center), quiet_button(words.t("pool-got-it"), Message::Dismiss)].spacing(10);
            panel_box(inside.into())
        }
        Finding::Silent(_) => {
            let inside = column![row![glyph(Icon::Warn, 16.0, ACCENT), semi(words.t("pool-silent"), 14.0, INK)].spacing(10).align_y(iced::Center), quiet_button(words.t("pool-retry"), Message::Retry)].spacing(10);
            panel_box(inside.into())
        }
        Finding::Fetching(fetching) => fetching_card(fetching, words),
        Finding::Found(candidate) => candidate_card(candidate, editor, pool, words, thumbs),
    }
}

pub(super) fn stage_of(step: Option<&crate::maps::Step>) -> usize {
    use crate::maps::Step;
    match step {
        None | Some(Step::Looking) => 0,
        Some(Step::Found(_) | Step::Downloading { .. }) => 1,
        Some(Step::Unpacking) => 2,
        Some(_) => 3,
    }
}

pub(super) fn fetching_card<'a>(fetching: &'a Fetching, words: &'a Words) -> Element<'a, Message> {
    use crate::maps::Step;
    let stage = stage_of(fetching.step.as_ref());
    let done_n = fetching.total - fetching.queue.len();
    let megabytes = |bytes: u64| screen::decimal(words, bytes as f32 / 1_048_576.0, 1);
    let download = match &fetching.step {
        Some(Step::Downloading { done, total: Some(total), .. }) => words.with("pool-step-download-of", &[("done", megabytes(*done)), ("total", megabytes(*total))]),
        _ => words.t("pool-step-download"),
    };
    let lines = [
        words.t(if stage == 0 { "pool-step-looking" } else { "pool-step-found" }),
        download,
        words.t("pool-step-unpack"),
        words.t("pool-step-check"),
    ];
    let mut list = column![].spacing(8);
    for (at, line) in lines.into_iter().enumerate() {
        let mark: Element<'a, Message> = if at < stage {
            glyph(Icon::Check, 14.0, ACCENT)
        } else if at == stage {
            container(container(Space::new()).width(7.0).height(7.0).style({ let dot = ui::faded(ACCENT); move |_| container::Style { background: Some(Background::Color(dot)), border: Border { radius: 3.5.into(), ..Border::default() }, ..container::Style::default() } })).center(14.0).into()
        } else {
            Space::new().width(14.0).into()
        };
        let colour = if at == stage { INK } else if at < stage { MUTED } else { FAINT };
        list = list.push(row![mark, faded_text(line, 13.5, colour)].spacing(10).align_y(iced::Center));
    }
    let inside = column![
        row![semi(words.with("pool-fetching", &[("n", (done_n + 1).min(fetching.total).to_string()), ("total", fetching.total.to_string())]), 14.5, INK), ui::grow(), icon_button(Icon::Close, 14.0, FAINT, Some(Message::Dismiss))].align_y(iced::Center),
        list,
    ]
    .spacing(12);
    panel_box(inside.into())
}

pub(super) fn candidate_card<'a>(candidate: &'a Candidate, _editor: &'a Editor, _pool: &'a Pool, words: &'a Words, thumbs: &'a HashMap<String, image::Handle>) -> Element<'a, Message> {
    let found = &candidate.found;
    let many = found.difficulties.len() > 1;
    let title = if many { words.n("pool-link-set", found.difficulties.len() as u64) } else { words.t("pool-link-found") };
    let heading = row![glyph(Icon::Chain, 15.0, ACCENT), semi(title, 14.5, INK)].spacing(10).align_y(iced::Center);
    let local_cover = candidate.found.difficulties.get(candidate.choice.unwrap_or(0)).and_then(|difficulty| thumbs.get(&difficulty.hash));
    let thumb: Element<'a, Message> = match local_cover.or(candidate.cover.as_ref()) {
        Some(handle) => image(crate::crops::fitted(handle, PANEL_WIDE - 64.0, 110.0, 12.0)).content_fit(iced::ContentFit::Fill).width(Length::Fill).height(110.0).border_radius(12.0).opacity(ui::fade()).into(),
        None => container(Space::new())
            .width(Length::Fill)
            .height(110.0)
            .style(|_| container::Style { background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.012))), border: Border { color: crate::theme::outline(Color::from_rgba(1.0, 1.0, 1.0, 0.04)), width: 1.0, radius: 10.0.into() }, ..container::Style::default() })
            .into(),
    };
    let names = column![wrapped_semi(found.title.clone(), 17.0, INK), ui::moving_text(found.artist.clone(), theme::SANS, 13.5, MUTED)].spacing(2).width(Length::Fill);
    let mut list = column![].spacing(6);
    for (at, difficulty) in found.difficulties.iter().enumerate() {
        let lit = candidate.choice == Some(at);
        let colour = crate::dossier::star_colour(difficulty.stars as f32);
        let dot = container(Space::new()).width(9.0).height(9.0).style(move |_| container::Style { background: Some(Background::Color(colour)), border: Border { radius: 4.5.into(), ..Border::default() }, ..container::Style::default() });
        let line = row![dot, wrapped_semi(difficulty.version.clone(), 13.5, INK), container(mono(stars_of(words, difficulty.stars), 13.0, colour)).width(48.0).align_x(iced::Right)].spacing(10).align_y(iced::Center);
        let made: Element<'a, Message> = if many {
            button(line).padding([8, 12]).width(Length::Fill).style(ui::button_faded(row_style(lit))).on_press(Message::Choose(at)).into()
        } else {
            container(line).padding([4, 4]).into()
        };
        list = list.push(made);
    }
    let mut actions = row![].spacing(8).align_y(iced::Center);
    let put_label = words.t("pool-link-put");
    actions = actions.push(if candidate.choice.is_some() { primary_button(put_label, Message::Confirm) } else { ui::primary(put_label, None) });
    actions = actions.push(plain_button(words.t("pool-link-cancel"), Message::Dismiss));
    let inside = column![heading, thumb, names, list, actions].spacing(12);
    panel_box(inside.into())
}

pub(super) fn add_drop<'a>(state: &'a State, editor: &'a Editor, pool: &'a Pool, words: &'a Words, thumbs: &'a HashMap<String, image::Handle>) -> Element<'a, Message> {
    let mut body = column![].spacing(14);
    let sources = [(SourceTab::Search, "pool-tab-search"), (SourceTab::Collections, "pool-tab-collections"), (SourceTab::Suggest, "pool-tab-suggest")];
    let buttons = row(sources.iter().map(|(source, key)| {
        let lit = editor.source == *source;
        button(text(words.t(key)).font(theme::SANS_SEMI).size(13.5).color(ui::faded(if lit { INK } else { MUTED })).width(Length::Fill).center())
            .padding([8, 12]).width(Length::Fill).style(ui::button_faded(theme::bare)).on_press(Message::Source(*source)).into()
    })).spacing(6);
    let active = sources.iter().position(|(source, _)| *source == editor.source).unwrap_or(0);
    let tabs = ui::sliding(buttons, active, ui::Pill { fill: Color::from_rgba(0.886, 0.282, 0.282, 0.15), edge: Color::from_rgba(0.886, 0.282, 0.282, 0.4), radius: 8.0, underline: None });
    body = body.push(tabs);
    if matches!(editor.source, SourceTab::Collections | SourceTab::Best) {
        if let Some(Notice::Already(at)) = &state.notice {
            body = body.push(faded_text(words.with("pool-already", &[("n", (at + 1).to_string())]), 13.0, ACCENT));
        }
        if let Some(finding) = &state.finding {
            body = body.push(finding_card(finding, editor, pool, words, thumbs));
        }
        body = body.push(if editor.source == SourceTab::Best { best_panel(state, words, thumbs) } else { collection_panel(state, editor, words, thumbs) });
        return add_box(body.into());
    }
    if editor.source == SourceTab::Suggest {
        body = body.push(suggestion_panel(state, words, thumbs));
        return add_box(body.into());
    }
    if let Some(Notice::Already(at)) = &state.notice {
        body = body.push(faded_text(words.with("pool-already", &[("n", (at + 1).to_string())]), 13.0, ACCENT));
    }
    if let Some(Notice::NotALink) = &state.notice {
        body = body.push(para(words.t("pool-not-link"), 13.0, ACCENT));
    }
    if let Some(finding) = &state.finding {
        body = body.push(finding_card(finding, editor, pool, words, thumbs));
    } else if state.songs.is_none() {
        body = body.push(mono(words.t("pool-reading"), 12.0, FAINT));
    } else if editor.query.trim().is_empty() {
        body = body.push(para(words.t("pool-search-start"), 13.0, MUTED));
    } else {
        let found = state.search(&editor.query);
        if found.is_empty() {
            body = body.push(faded_text(words.t("pool-nothing"), 13.0, MUTED));
        }
        let mut list = column![].spacing(2);
        for (hash, map) in found {
            let inside = row![
                cover(thumbs, Some(&hash), 56.0, 32.0, THUMB_ROUND),
                column![semi(map.title.clone(), 13.5, INK), faded_text(map.artist.clone(), 12.0, MUTED), mono(map.version.clone(), 11.0, FAINT)].spacing(1).width(Length::Fill),
                glyph(Icon::Plus, 16.0, MUTED),
            ]
            .spacing(12)
            .align_y(iced::Center);
            list = list.push(button(inside).padding([6, 8]).width(Length::Fill).style(ui::button_faded(ui::calm(theme::row(false)))).on_press(Message::Put(hash)));
        }
        body = body.push(results_list(list));
    }
    add_box(body.into())
}

pub(super) fn add_box<'a>(inside: Element<'a, Message>) -> Element<'a, Message> {
    menu_box(container(scrollable(inside).style(ui::thin_scroll).direction(ui::hidden_bar())).max_height(ADD_HIGH).into(), ADD_WIDE, 14.0, 14.0, Color::from_rgb(0.071, 0.027, 0.039))
}

pub(super) const ADD_WIDE: f32 = 560.0;
pub(super) const ADD_HIGH: f32 = 330.0;

pub(super) fn add_row<'a>(pool: &'a Pool, at: usize, editor: &'a Editor, words: &'a Words) -> Element<'a, Message> {
    let k = ui::fade();
    let colour = if pool.collection { Color::from_rgb8(95, 95, 95) } else { pool.slots.get(at).map_or(ACCENT, slot_colour) };
    let mut left = row![number_badge(category_number(pool, at), true)].spacing(12).align_y(iced::Center);
    if !pool.collection {
        left = left.push(category_menu(pool, at, false));
    }
    let field = text_input(&words.t("pool-search-hint"), &editor.query)
        .id(iced::widget::Id::new("pool-search"))
        .on_input(Message::Query)
        .on_paste(|contents| Message::PasteInto(Input::Query, contents))
        .font(theme::SANS)
        .size(14.0)
        .padding(0)
        .style(ui::bare_input(ui::fade()))
        .width(Length::Fill);
    let inside = row![left, glyph(Icon::Search, 14.0, MUTED), field, icon_button(Icon::Close, 14.0, FAINT, Some(Message::Select(None)))].spacing(12).align_y(iced::Center);
    container(inside)
        .height(ROW_HIGH)
        .align_y(iced::Center)
        .padding([0, 12])
        .width(Length::Fill)
        .style(move |_| container::Style { background: Some(Background::Color(tinted(colour, 0.08, k))), border: Border { color: crate::theme::outline(Color { a: k, ..colour }), width: 2.0, radius: TILE_ROUND.into() }, ..container::Style::default() })
        .into()
}

pub(super) fn best_panel<'a>(state: &'a State, words: &'a Words, thumbs: &'a HashMap<String, image::Handle>) -> Element<'a, Message> {
    let refresh = quiet_button(words.t("pool-best-refresh"), Message::RefreshBest);
    let Some(scores) = &state.best else {
        return column![para(words.t("pool-best-no-profile"), 13.0, MUTED), refresh].spacing(12).into();
    };
    if scores.is_empty() {
        return column![para(words.t("pool-best-empty"), 13.0, MUTED), refresh].spacing(12).into();
    }
    if state.songs.is_none() {
        return para(words.t("pool-reading"), 13.0, MUTED);
    }
    let mut list = column![].spacing(2);
    for (at, score) in scores.iter().enumerate() {
        let hash = score.hash.to_ascii_lowercase();
        let local = state.songs.as_ref().is_some_and(|songs| songs.contains_key(&hash));
        let mut names = column![semi(score.title.clone(), 13.5, INK), faded_text(score.artist.clone(), 12.0, MUTED), mono(score.version.clone(), 11.0, FAINT)].spacing(1).width(Length::Fill);
        if !local {
            names = names.push(faded_text(words.t("pool-best-missing"), 11.5, FAINT));
        }
        let inside = row![
            cover(thumbs, Some(&hash), 56.0, 32.0, THUMB_ROUND),
            names,
            mono(format!("{} pp", words.lang().group(score.pp.max(0.0).round() as u64)), 12.0, MUTED),
        ]
        .spacing(12)
        .align_y(iced::Center);
        list = list.push(button(inside).padding([6, 8]).width(Length::Fill).style(ui::button_faded(ui::calm(theme::row(false)))).on_press(Message::Best(at)));
    }
    column![refresh, results_list(list)].spacing(10).into()
}

pub(super) fn suggestion_panel<'a>(state: &'a State, words: &'a Words, thumbs: &'a HashMap<String, image::Handle>) -> Element<'a, Message> {
    let lean = match &state.screen { Screen::Editor(editor) => editor.lean, _ => None };
    let Some(suggestion) = &state.suggestions else {
        return para(words.t("pool-suggest-no-slot"), 13.0, MUTED);
    };
    let Some(target) = suggestion.target else {
        return para(words.t("pool-suggest-needs-map"), 13.0, MUTED);
    };
    let Some(maps) = &suggestion.maps else {
        return para(words.t("pool-suggest-running"), 13.0, MUTED);
    };
    let known: HashSet<&str> = maps.iter().map(|(hash, _)| hash.as_str()).collect();
    let fresh: Vec<&crate::bot::Offered> = match &suggestion.offered {
        Some(Ok(found)) => found.iter().filter(|item| !known.contains(item.md5.as_str()) && state.songs.as_ref().is_none_or(|songs| !songs.contains_key(&item.md5))).take(12).collect(),
        _ => Vec::new(),
    };
    if maps.is_empty() && fresh.is_empty() {
        return para(words.t(if suggestion.offered.is_none() { "pool-suggest-running" } else { "pool-suggest-empty" }), 13.0, MUTED);
    }
    let mut list = column![].spacing(2);
    for (hash, measure) in maps {
        let Some(map) = state.songs.as_ref().and_then(|songs| songs.get(hash)) else { continue };
        let mut inside = row![
            cover(thumbs, Some(hash), 56.0, 32.0, THUMB_ROUND),
            column![semi(map.title.clone(), 13.5, INK), faded_text(map.artist.clone(), 12.0, MUTED), mono(map.version.clone(), 11.0, FAINT)].spacing(1).width(Length::Fill),
        ]
        .spacing(12)
        .align_y(iced::Center);
        if let Some(at) = lean.and_then(|skill| Skill::ALL.iter().position(|other| *other == skill)) {
            inside = inside.push(mono(format!("{}%", measure.percents()[at]), 12.5, INK));
        }
        inside = inside.push(mono(stars_of(words, measure.stars), 12.5, crate::dossier::star_colour(measure.stars as f32)));
        list = list.push(button(inside).padding([6, 8]).width(Length::Fill).style(ui::button_faded(ui::calm(theme::row(false)))).on_press(Message::Put(hash.clone())));
    }
    if !fresh.is_empty() {
        list = list.push(container(ui::label(words.t("pool-suggest-online"), MUTED)).padding([8, 8]));
    }
    for item in fresh {
        let inside = row![
            column![semi(item.title.clone(), 13.5, INK), faded_text(item.artist.clone(), 12.0, MUTED), mono(item.version.clone(), 11.0, FAINT)].spacing(1).width(Length::Fill),
            mono(stars_of(words, item.stars), 12.5, crate::dossier::star_colour(item.stars as f32)),
        ]
        .spacing(12)
        .align_y(iced::Center);
        list = list.push(button(inside).padding([6, 8]).width(Length::Fill).style(ui::button_faded(ui::calm(theme::row(false)))).on_press(Message::TakeOffered(item.id)));
    }
    let mut told = column![faded_text(words.with("pool-suggest-target", &[("stars", stars_of(words, target))]), 12.5, MUTED)].spacing(4);
    if let Some(skill) = lean {
        told = told.push(faded_text(words.t(skill.lean_key()), 13.5, INK));
    }
    column![told, results_list(list)].spacing(12).into()
}

pub(super) fn collection_panel<'a>(state: &'a State, editor: &'a Editor, words: &'a Words, thumbs: &'a HashMap<String, image::Handle>) -> Element<'a, Message> {
    let Some(collections) = &state.collections else {
        return para(words.t("pool-collections-reading"), 13.0, MUTED);
    };
    if collections.is_empty() {
        return para(words.t("pool-collections-empty"), 13.0, MUTED);
    }
    if state.songs.is_none() {
        return para(words.t("pool-reading"), 13.0, MUTED);
    }
    let songs = state.songs.as_ref();
    let Some(collection) = editor.collection.and_then(|at| collections.get(at)) else {
        let mut list = column![].spacing(4);
        for (at, collection) in collections.iter().enumerate() {
            list = list.push(button(semi(collection.name.clone(), 13.5, INK)).padding([9, 10]).width(Length::Fill).style(ui::button_faded(ui::calm(theme::row(false)))).on_press(Message::Collection(Some(at))));
        }
        return results_list(list);
    };
    let heading = quiet_button(words.t("pool-collection-back"), Message::Collection(None));
    let wanted = editor.query.trim().to_lowercase();
    let mut sets: Vec<(Option<(&str, &str)>, Vec<&String>)> = Vec::new();
    for hash in &collection.hashes {
        match songs.and_then(|songs| songs.get(hash)) {
            Some(map) => {
                if !wanted.is_empty() && !format!("{} {} {}", map.title, map.artist, map.version).to_lowercase().contains(&wanted) {
                    continue;
                }
                let key = Some((map.artist.as_str(), map.title.as_str()));
                match sets.iter_mut().find(|(held, _)| *held == key) {
                    Some((_, hashes)) => hashes.push(hash),
                    None => sets.push((key, vec![hash])),
                }
            }
            None if wanted.is_empty() => sets.push((None, vec![hash])),
            None => {}
        }
    }
    let pages = sets.len().div_ceil(COLLECTION_PAGE).max(1);
    let page = editor.collection_page.min(pages - 1);
    let mut list = column![].spacing(2);
    for (key, hashes) in sets.iter().skip(page * COLLECTION_PAGE).take(COLLECTION_PAGE) {
        let first = hashes[0];
        let Some((artist, title)) = key else {
            let row = row![
                cover(thumbs, None, 56.0, 32.0, THUMB_ROUND),
                column![mono(first.clone(), 11.0, MUTED), faded_text(words.t("pool-collection-missing"), 12.0, FAINT)].spacing(2).width(Length::Fill),
                glyph(Icon::Plus, 16.0, MUTED),
            ]
            .spacing(12)
            .align_y(iced::Center);
            list = list.push(button(row).padding([6, 8]).width(Length::Fill).style(ui::button_faded(ui::calm(theme::row(false)))).on_press(Message::CollectionHash(first.clone())));
            continue;
        };
        let versions = row(hashes.iter().map(|hash| {
            let version = songs.and_then(|songs| songs.get(*hash)).map(|map| map.version.clone()).unwrap_or_default();
            button(mono(version, 11.5, INK)).padding([4, 8]).style(ui::button_faded(theme::filter_chip(false))).on_press(Message::CollectionHash((*hash).clone())).into()
        }))
        .spacing(6)
        .wrap();
        let line = row![
            cover(thumbs, Some(first), 56.0, 32.0, THUMB_ROUND),
            column![semi((*title).to_owned(), 13.5, INK), faded_text((*artist).to_owned(), 12.0, MUTED), versions].spacing(4).width(Length::Fill),
        ]
        .spacing(12)
        .align_y(iced::Center);
        list = list.push(container(line).padding([6, 8]).width(Length::Fill));
    }
    let mut body = column![heading, results_list(list)].spacing(10);
    if pages > 1 {
        let mut controls = row![].spacing(10).align_y(iced::Center);
        if page > 0 {
            controls = controls.push(quiet_button(words.t("pool-collection-prev"), Message::CollectionPage(page - 1)));
        }
        controls = controls.push(ui::grow()).push(mono(format!("{} / {}", page + 1, pages), 11.5, MUTED));
        if page + 1 < pages {
            controls = controls.push(quiet_button(words.t("pool-collection-next"), Message::CollectionPage(page + 1)));
        }
        body = body.push(controls);
    }
    body.into()
}

