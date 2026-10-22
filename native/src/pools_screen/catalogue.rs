use super::*;
use iced::widget::column;

pub(super) fn published_hashes(item: &crate::bot::Publication) -> Vec<Option<String>> {
    match item.kind.as_str() {
        "collection" => item.content["hashes"].as_array().into_iter().flatten().map(|hash| hash.as_str().map(str::to_owned)).collect(),
        _ => item.content["slots"].as_array().into_iter().flatten().map(|slot| slot["hash"].as_str().map(str::to_owned)).collect(),
    }
}

pub(super) fn strip<'a>(thumbs: &HashMap<String, image::Handle>, maps: &[(Option<String>, Color)], room: f32) -> Element<'a, Message> {
    let asked = maps.len().min(STRIP_MOST).max(1) as f32;
    let each = ((room - STRIP_GAP * (asked - 1.0)) / asked).clamp(40.0, 112.0).floor();
    let high = (each * 0.56).clamp(28.0, 60.0).floor();
    let fit = (((room + STRIP_GAP) / (each + STRIP_GAP)).floor() as usize).max(2);
    let (shown, more) = if maps.len() > fit { (fit - 1, maps.len() - (fit - 1)) } else { (maps.len(), 0) };
    let k = ui::fade();
    let mut line = row![].spacing(STRIP_GAP).align_y(iced::alignment::Vertical::Top);
    for (hash, colour) in maps.iter().take(shown) {
        let colour = *colour;
        let tick = container(Space::new()).width(each).height(3.0).style(move |_| container::Style {
            background: Some(Background::Color(Color { a: colour.a * k, ..colour })),
            border: Border { radius: 1.5.into(), ..Border::default() },
            ..container::Style::default()
        });
        line = line.push(column![cover(thumbs, hash.as_deref(), each, high, 6.0), tick].spacing(4));
    }
    if more > 0 {
        line = line.push(container(mono(format!("+{more}"), 12.0, MUTED)).width(each).center_y(high).align_x(iced::alignment::Horizontal::Center));
    }
    line.into()
}

pub(super) fn cells<'a>(parts: Vec<(Col, Element<'a, Message>)>) -> iced::widget::Row<'a, Message> {
    row(parts.into_iter().map(|(col, part)| -> Element<'a, Message> {
        match col {
            Col::Fill => container(part).width(Length::Fill).clip(true).into(),
            Col::Wide(wide) => container(part).width(wide).clip(true).into(),
            Col::End(wide) => container(part).width(wide).align_x(iced::alignment::Horizontal::Right).into(),
        }
    }))
    .spacing(LEDGER_GAP)
    .align_y(iced::Center)
}

pub(super) fn ledger_line<'a>() -> Element<'a, Message> {
    let k = ui::fade();
    container(Space::new()).width(Length::Fill).height(1.0).style(move |_| container::Style { background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.04 * k))), ..container::Style::default() }).into()
}

pub(super) fn ledger_box<'a>(inside: Element<'a, Message>) -> Element<'a, Message> {
    let k = ui::fade();
    container(inside)
        .width(Length::Fill)
        .padding([6, 0])
        .style(move |theme| {
            let slab = theme::slab(theme);
            container::Style {
                background: slab.background.map(|fill| match fill { Background::Color(colour) => Background::Color(Color { a: colour.a * k, ..colour }), other => other }),
                border: Border { color: crate::theme::outline(Color { a: slab.border.color.a * k, ..slab.border.color }), width: 1.0, radius: 14.0.into() },
                ..container::Style::default()
            }
        })
        .into()
}

pub(super) fn ledger_row(_: &iced::Theme, status: button::Status) -> button::Style {
    let hot = matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        background: hot.then_some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.016))),
        text_color: INK,
        border: Border::default(),
        shadow: iced::Shadow::default(),
        snap: true,
    }
}

pub(super) fn ledger_head<'a>(parts: Vec<(Col, String)>) -> Element<'a, Message> {
    container(cells(parts.into_iter().map(|(col, label)| (col, ui::mono_small(label.to_uppercase(), MUTED))).collect())).padding([8.0, LEDGER_PAD]).into()
}

pub(super) fn ringed<'a>(inside: Element<'a, Message>, round: f32) -> Element<'a, Message> {
    let k = ui::fade();
    container(inside)
        .padding(2)
        .style(move |_| container::Style { background: Some(Background::Color(Color { a: k, ..theme::GROUND })), border: Border { radius: (round + 2.0).into(), ..Border::default() }, ..container::Style::default() })
        .into()
}

pub(super) fn overlapped<'a>(pieces: Vec<Element<'a, Message>>, step: f32, each: f32) -> Element<'a, Message> {
    let wide = each + step * pieces.len().saturating_sub(1) as f32;
    let mut layers: Vec<Element<'a, Message>> = vec![Space::new().width(wide).height(each).into()];
    layers.extend(pieces.into_iter().enumerate().map(|(at, piece)| -> Element<'a, Message> { container(piece).padding(Padding { left: at as f32 * step, ..Padding::ZERO }).into() }));
    iced::widget::Stack::with_children(layers).width(wide).height(each).into()
}

pub(super) fn cover_stack<'a>(thumbs: &HashMap<String, image::Handle>, hashes: &[Option<&str>]) -> Element<'a, Message> {
    let shown: Vec<Option<&str>> = if hashes.is_empty() { vec![None] } else { hashes.iter().take(3).copied().collect() };
    let pieces = shown.into_iter().map(|hash| ringed(cover(thumbs, hash, 40.0, 40.0, THUMB_ROUND), THUMB_ROUND)).collect();
    container(overlapped(pieces, 24.0, 44.0)).width(92.0).into()
}

pub(super) fn face<'a>(state: &'a State, name: &str) -> Element<'a, Message> {
    match state.faces.get(&name.to_lowercase()) {
        Some(face) => image(crate::crops::fitted(face, FACE, FACE, FACE / 2.0)).width(FACE).height(FACE).content_fit(iced::ContentFit::Fill).opacity(ui::fade()).into(),
        None => {
            let k = ui::fade();
            container(glyph(Icon::Person, 16.0, MUTED))
                .center(FACE)
                .style(move |_| container::Style { background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.03 * k))), border: Border { radius: (FACE / 2.0).into(), ..Border::default() }, ..container::Style::default() })
                .into()
        }
    }
}

pub(super) fn named_face<'a>(state: &'a State, name: &str, most: f32) -> Element<'a, Message> {
    let wide = (ui::text_width(name, theme::SANS_SEMI, 13.0) + 2.0).min(most);
    row![face(state, name), container(ui::moving_text(name.to_owned(), theme::SANS_SEMI, 13.0, INK)).width(wide)].spacing(7).align_y(iced::Center).into()
}

pub(super) fn makers_sheet<'a>(state: &'a State, names: &[String], words: &'a Words) -> Element<'a, Message> {
    let k = ui::fade();
    let mut list = column![].spacing(2);
    for pair in names.chunks(2) {
        list = list.push(row(pair.iter().map(|name| container(author_chip(state, name)).width(Length::FillPortion(1)).into()).collect::<Vec<Element<'a, Message>>>()).spacing(4));
    }
    let head = row![ui::mono_small(words.t("pool-mappoolers").to_uppercase(), MUTED), ui::grow(), mono(names.len().to_string(), 13.0, INK)].align_y(iced::Center);
    container(column![container(head).padding([0, 9]), list].spacing(8))
        .width(340.0)
        .padding([14, 7])
        .style(move |_| container::Style {
            background: Some(Background::Color(Color { a: k, ..Color::from_rgb8(0x1d, 0x0b, 0x0f) })),
            border: Border { color: crate::theme::outline(soft(0.16, k)), width: 1.0, radius: 12.0.into() },
            shadow: iced::Shadow { color: Color::from_rgba(0.0, 0.0, 0.0, 0.45 * k), offset: iced::Vector::new(0.0, 16.0), blur_radius: 40.0 },
            ..container::Style::default()
        })
        .into()
}

pub(super) fn makers_cell<'a>(state: &'a State, key: String, names: &[String], wide: f32, words: &'a Words) -> Element<'a, Message> {
    if names.is_empty() {
        return faded_text(words.t("ledger-no-mappoolers"), 13.0, FAINT);
    }
    let each = (wide - 10.0) / names.len() as f32 - FACE - 7.0;
    if names.len() <= NAMED_MOST && each >= 56.0 {
        return row(names.iter().map(|name| named_face(state, name, each)).collect::<Vec<_>>()).spacing(10).align_y(iced::Center).into();
    }
    let faces = overlapped(names.iter().take(FACES_MOST).map(|name| ringed(face(state, name), FACE / 2.0)).collect(), 18.0, FACE + 4.0);
    let mut line = row![faces].spacing(8).align_y(iced::Center);
    if names.len() > FACES_MOST {
        let k = ui::fade();
        line = line.push(container(mono(format!("+{}", names.len() - FACES_MOST), 11.5, INK)).padding([2, 7]).style(move |_| container::Style {
            background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.03 * k))),
            border: Border { color: crate::theme::outline(soft(0.1, k)), width: 1.0, radius: 11.0.into() },
            ..container::Style::default()
        }));
    }
    let open = state.makers.as_deref() == Some(key.as_str());
    let anchor = button(line).padding([3, 5]).style(ui::button_faded(surface_style(open))).on_press(Message::Makers(Some(key)));
    ui::under(anchor, open.then(|| makers_sheet(state, names, words)), Message::Makers(None))
}

pub(super) fn stars_span<'a>(words: &Words, low: Option<f64>, high: Option<f64>) -> Element<'a, Message> {
    let pill = |stars: f64| crate::dossier::star_pill(stars_of(words, stars), stars as f32, 13.5);
    match (low, high) {
        (Some(low), Some(high)) if (high - low).abs() >= 0.005 => row![pill(low), pill(high)].spacing(6).align_y(iced::Center).into(),
        (Some(low), _) => pill(low),
        _ => Space::new().into(),
    }
}

pub(super) struct Shape {
    makers: Option<f32>,
    stars: bool,
    length: bool,
    bars: bool,
    changed: bool,
    publisher: bool,
}

pub(super) fn shape(width: f32, collection: bool) -> Shape {
    Shape {
        makers: (!collection).then_some(if width >= 1000.0 { 270.0 } else if width >= 760.0 { 220.0 } else { 104.0 }),
        stars: width >= 620.0,
        length: width >= 860.0,
        bars: width >= 1060.0,
        changed: width >= 700.0,
        publisher: width >= 640.0,
    }
}

pub(super) fn draft_row<'a>(state: &'a State, pool: &'a Pool, words: &'a Words, thumbs: &'a HashMap<String, image::Handle>, shape: &Shape, today: i64, room: Option<f32>) -> Element<'a, Message> {
    let hashes: Vec<Option<&str>> = pool.slots.iter().filter(|slot| !slot.is_empty()).map(|slot| slot.hash.as_deref()).collect();
    let name = if pool.name.is_empty() { words.t(if pool.collection { "collections-new" } else { "pool-untitled" }) } else { pool.name.clone() };
    let facts = pool.facts();
    let milliseconds: i64 = pool.slots.iter().filter_map(|slot| slot.measure.as_ref()).map(|measure| measure.length_ms).sum();
    let title = text(name).font(theme::SANS_SEMI).size(if room.is_some() { 16.0 } else { 15.0 }).color(ui::faded(INK)).wrapping(text::Wrapping::None);
    let lead: Element<'a, Message> = if room.is_some() { title.into() } else { row![cover_stack(thumbs, &hashes), title].spacing(12).align_y(iced::Center).into() };
    let mut parts: Vec<(Col, Element<'a, Message>)> = vec![(Col::Fill, lead)];
    if let Some(wide) = shape.makers {
        parts.push((Col::Wide(wide), makers_cell(state, format!("draft:{}", pool.id), &pool.authors, wide, words)));
    }
    parts.push((Col::Wide(44.0), mono(facts.cards.to_string(), 13.0, INK)));
    if shape.stars {
        parts.push((Col::Wide(STARS_WIDE), stars_span(words, facts.low, facts.high)));
    }
    if shape.length {
        parts.push((Col::Wide(64.0), mono(if milliseconds > 0 { words.length_said(milliseconds) } else { String::new() }, 13.0, INK)));
    }
    if shape.bars {
        parts.push((Col::Wide(96.0), bars(pool)));
    }
    if shape.changed {
        parts.push((Col::End(132.0), mono(words.day(pool.changed_at, today), 12.5, MUTED)));
    }
    let Some(room) = room.filter(|_| !pool.slots.is_empty()) else {
        return button(container(cells(parts)).center_y(Length::Fill)).padding([0.0, LEDGER_PAD]).height(64.0).width(Length::Fill).style(ui::button_faded(ledger_row)).on_press(Message::Open(pool.id.clone())).into();
    };
    let grey = Color::from_rgb8(95, 95, 95);
    let maps: Vec<(Option<String>, Color)> = pool.slots.iter().map(|slot| (slot.hash.clone(), if pool.collection { grey } else { slot_colour(slot) })).collect();
    let inside = column![container(cells(parts)).height(44.0).center_y(44.0), strip(thumbs, &maps, room)].spacing(8);
    button(inside).padding([12.0, LEDGER_PAD]).width(Length::Fill).style(ui::button_faded(ledger_row)).on_press(Message::Open(pool.id.clone())).into()
}

pub(super) fn new_row<'a>(words: &Words, collection: bool) -> Element<'a, Message> {
    let k = ui::fade();
    let plus = container(glyph(Icon::Plus, 18.0, MUTED)).center(40.0).style(move |_| container::Style {
        background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.14 * k))),
        border: Border { color: crate::theme::outline(soft(0.08, k)), width: 1.0, radius: THUMB_ROUND.into() },
        ..container::Style::default()
    });
    let label = text(words.t(if collection { "collections-new" } else { "pools-new" })).font(theme::SANS_SEMI).size(15.0).color(ui::faded(MUTED));
    button(container(row![container(plus).padding(2), label].spacing(12).align_y(iced::Center)).center_y(Length::Fill)).padding([0.0, LEDGER_PAD]).height(64.0).width(Length::Fill).style(ui::button_faded(ledger_row)).on_press(Message::New).into()
}

pub(super) fn published_row<'a>(state: &'a State, item: &'a crate::bot::Publication, words: &'a Words, thumbs: &'a HashMap<String, image::Handle>, shape: &Shape, collection: bool, room: Option<f32>) -> Element<'a, Message> {
    let count = if collection { item.content["hashes"].as_array().map_or(0, Vec::len) } else { item.content["slots"].as_array().map_or(0, |slots| slots.iter().filter(|slot| slot["hash"].is_string()).count()) };
    let mut parts: Vec<(Col, Element<'a, Message>)> = vec![(Col::Fill, text(item.name.clone()).font(theme::SANS_SEMI).size(15.0).color(ui::faded(INK)).wrapping(text::Wrapping::None).into())];
    if let Some(wide) = shape.makers {
        let names: Vec<String> = item.content["authors"].as_array().into_iter().flatten().filter_map(|name| name.as_str().map(str::to_owned)).collect();
        parts.push((Col::Wide(wide), makers_cell(state, format!("published:{}", item.id), &names, wide, words)));
    }
    if shape.publisher {
        let publisher: Element<'a, Message> = match item.content["compiler"].as_str().filter(|name| !name.is_empty()) {
            Some(name) => named_face(state, name, 150.0),
            None => Space::new().into(),
        };
        parts.push((Col::Wide(190.0), publisher));
    }
    parts.push((Col::Wide(44.0), mono(count.to_string(), 13.0, INK)));
    parts.push((Col::End(132.0), faded_text(words.t(if item.mine { "ledger-edit" } else { "catalogue-copy" }), 13.0, if item.mine { INK } else { MUTED })));
    let Some(room) = room.filter(|_| count > 0) else {
        return button(container(cells(parts)).center_y(Length::Fill)).padding([0.0, LEDGER_PAD]).height(56.0).width(Length::Fill).style(ui::button_faded(ledger_row)).on_press(Message::OpenPublication(item.id.clone())).into();
    };
    let grey = Color::from_rgb8(95, 95, 95);
    let parsed = (!collection).then(|| serde_json::to_vec(&item.content).ok().and_then(|bytes| pool_share::from_file(&bytes, 0).ok())).flatten();
    let maps: Vec<(Option<String>, Color)> = match &parsed {
        Some(pool) => pool.slots.iter().map(|slot| (slot.hash.clone(), slot_colour(slot))).collect(),
        None => published_hashes(item).into_iter().map(|hash| (hash, grey)).collect(),
    };
    let inside = column![container(cells(parts)).height(44.0).center_y(44.0), strip(thumbs, &maps, room)].spacing(8);
    button(inside).padding([12.0, LEDGER_PAD]).width(Length::Fill).style(ui::button_faded(ledger_row)).on_press(Message::OpenPublication(item.id.clone())).into()
}

pub(super) fn refusal_line<'a>(refused: &pool_share::Refused, words: &Words) -> Element<'a, Message> {
    let key = match refused {
        pool_share::Refused::NotAPool => "pool-refused-not",
        pool_share::Refused::Newer => "pool-refused-newer",
        pool_share::Refused::Damaged => "pool-refused-damaged",
    };
    faded_text(words.t(key), 13.0, ACCENT)
}

pub(super) fn shelf<'a>(state: &'a State, words: &'a Words, thumbs: &'a HashMap<String, image::Handle>, width: f32, t: f32, head_t: f32, clocks: &Clocks) -> Element<'a, Message> {
    let collection = state.collection_shelf;
    let shape = shape(width, collection);
    let room = (width - 2.0 * LEDGER_PAD - 2.0).max(120.0);
    let drafts_room = state.roomy_drafts().then_some(room);
    let published_room = state.roomy_published().then_some(room);
    let head = move || -> Element<'a, Message> {
        let tab = |label: String, lit: bool, press: Message| -> Element<'a, Message> {
            button(container(text(label).font(theme::SANS_SEMI).size(13.5).color(ui::faded(if lit { INK } else { MUTED }))).center_y(CONTROL)).padding([0, 14]).style(ui::button_faded(theme::bare)).on_press(press).into()
        };
        let at = if collection { 3 } else { match state.shelf { Shelf::Mine => 0, Shelf::Saved => 1, Shelf::Published => 2 } };
        let tabs = ui::sliding(
            row![
                tab(words.t("shelf-mine"), at == 0, Message::Shelf(Shelf::Mine)),
                tab(words.t("shelf-saved"), at == 1, Message::Shelf(Shelf::Saved)),
                tab(words.t("catalogue-published"), at == 2, Message::Shelf(Shelf::Published)),
                tab(words.t("catalogue-collections"), at == 3, Message::Catalogue(true)),
            ].spacing(4),
            at,
            ui::Pill { fill: Color::from_rgba(0.886, 0.282, 0.282, 0.15), edge: Color::from_rgba(0.886, 0.282, 0.282, 0.4), radius: 8.0, underline: None },
        );
        let mut line = row![semi(words.t("pools-title"), 22.0, INK), tabs, ui::grow()].spacing(16).align_y(iced::Center);
        if !collection && width >= 860.0 {
            line = line.push(quiet_button(words.t("pools-open"), Message::OpenFile));
        }
        line.push(primary_button(words.t(if collection { "catalogue-new" } else { "pools-new" }), Message::New)).into()
    };
    let mut page = column![ui::appearing(ui::appear(head_t, 0), 10.0, head)].spacing(18);
    if let Some(refused) = &state.refused {
        page = page.push(ui::appearing(ui::appear(t, 1), 8.0, || refusal_line(refused, words)));
    }
    if !collection {
        return showcase(state, page, words, thumbs, width, t, clocks);
    }
    if collection {
        return collections_showcase(state, page, words, thumbs, width, t, clocks);
    }
    let drafts_head = || -> Element<'a, Message> {
        let mut parts = vec![(Col::Fill, words.t("catalogue-drafts"))];
        if let Some(wide) = shape.makers { parts.push((Col::Wide(wide), words.t("pool-mappoolers"))); }
        parts.push((Col::Wide(44.0), words.t("ledger-cards")));
        if shape.stars { parts.push((Col::Wide(STARS_WIDE), words.t("ledger-stars"))); }
        if shape.length { parts.push((Col::Wide(64.0), words.t("ledger-length"))); }
        if shape.bars { parts.push((Col::Wide(96.0), words.t("ledger-difficulty"))); }
        if shape.changed { parts.push((Col::End(132.0), words.t("ledger-changed"))); }
        ledger_head(parts)
    };
    let mut drafts = column![ui::appearing(ui::appear(t, 1), 8.0, drafts_head)];
    drafts = drafts.push(ledger_line()).push(ui::appearing(ui::appear(t, 2), 8.0, || new_row(words, collection)));
    for (at, pool) in state.list.iter().filter(|pool| pool.collection == collection).enumerate() {
        drafts = drafts.push(ledger_line()).push(ui::appearing(ui::appear(t, 3 + at), 8.0, || draft_row(state, pool, words, thumbs, &shape, clocks.today, drafts_room)));
    }
    page = page.push(ledger_box(drafts.into()));
    if collection {
        if let Some(collections) = &state.collections {
            page = page.push(ui::appearing(ui::appear(t, 3), 8.0, || ui::mono_small(words.t("catalogue-game-collections"), MUTED)));
            for (at, found) in collections.iter().enumerate() {
                page = page.push(ui::appearing(ui::appear(t, 3 + at), 8.0, || quiet_button(found.name.clone(), Message::ImportCollection(at))));
            }
        }
    }
    let published_head = || -> Element<'a, Message> {
        let mut parts: Vec<(Col, Element<'a, Message>)> = vec![(Col::Fill, ui::mono_small(words.t("catalogue-published").to_uppercase(), MUTED))];
        if let Some(wide) = shape.makers { parts.push((Col::Wide(wide), ui::mono_small(words.t("pool-mappoolers").to_uppercase(), MUTED))); }
        if shape.publisher { parts.push((Col::Wide(190.0), ui::mono_small(words.t("pool-publisher").to_uppercase(), MUTED))); }
        parts.push((Col::Wide(44.0), ui::mono_small(words.t("ledger-cards").to_uppercase(), MUTED)));
        parts.push((Col::End(132.0), quiet_button(words.t("catalogue-refresh"), Message::Catalogue(collection))));
        container(cells(parts)).padding([2.0, LEDGER_PAD]).into()
    };
    let mut published = column![ui::appearing(ui::appear(t, 4), 8.0, published_head)];
    if state.catalogue_loading {
        published = published.push(ledger_line()).push(ui::appearing(ui::appear(t, 5), 8.0, || container(faded_text(words.t("catalogue-loading"), 13.0, MUTED)).padding([14.0, LEDGER_PAD]).into()));
    }
    if state.publications.is_empty() && !state.catalogue_loading {
        published = published.push(ledger_line()).push(ui::appearing(ui::appear(t, 5), 8.0, || container(faded_text(words.t("catalogue-empty"), 13.0, MUTED)).padding([14.0, LEDGER_PAD]).into()));
    }
    for (at, item) in state.publications.iter().filter(|item| (item.kind == "collection") == collection).enumerate() {
        let seen = t.min(clocks.of(&item.id));
        published = published.push(ledger_line()).push(ui::appearing(ui::appear(seen, 5 + at), 8.0, || published_row(state, item, words, thumbs, &shape, collection, published_room)));
    }
    page = page.push(ledger_box(published.into()));
    if let Some(error) = &state.catalogue_error {
        page = page.push(ui::appearing(ui::appear(t, 5), 8.0, || catalogue_error(error, words)));
    }
    if state.catalogue_more {
        page = page.push(quiet_button(words.t("catalogue-more"), Message::CatalogueMore));
    }
    page.into()
}

pub(super) const MOSAIC: usize = 8;
pub(super) const GAME_MOST: usize = 60;
pub(super) const HERO_COVERS: usize = 15;
pub(super) const CARD_GAP: f32 = 16.0;
pub(super) const CARD_TOP: f32 = 128.0;
pub(super) const CARD_FILL: Color = Color::from_rgb(0.09, 0.039, 0.055);

pub(super) struct Shown {
    title: String,
    makers: Vec<String>,
    hashes: Vec<Option<String>>,
    low: Option<f64>,
    high: Option<f64>,
    mods: Vec<(Mod, usize)>,
    when: String,
    plain: bool,
    backing: Option<String>,
}

pub fn backing_key(pool: &Pool) -> Option<String> {
    pool.backdrop.as_ref().map(|backdrop| match &backdrop.from {
        pools::BackdropFrom::Map(hash) => format!("backdrop:{}:{hash}", pool.id),
        pools::BackdropFrom::File(path) => format!("backdrop:{}:{}", pool.id, path.display()),
    })
}

pub(super) fn shown_of(pool: &Pool, words: &Words, when: String) -> Shown {
    let facts = pool.facts();
    let filled: Vec<&Slot> = pool.slots.iter().filter(|slot| !slot.is_empty()).collect();
    Shown {
        title: if pool.name.is_empty() { words.t(if pool.collection { "collections-new" } else { "pool-untitled" }) } else { pool.name.clone() },
        makers: pool.authors.clone(),
        hashes: filled.iter().map(|slot| slot.hash.clone()).collect(),
        low: facts.low,
        high: facts.high,
        mods: Mod::ALL.into_iter().map(|mods| (mods, filled.iter().filter(|slot| slot.mods == mods).count())).filter(|(_, count)| *count > 0).collect(),
        when,
        plain: pool.collection,
        backing: backing_key(pool),
    }
}

pub(super) fn plate<'a>(inside: Element<'a, Message>) -> Element<'a, Message> {
    let k = ui::fade();
    container(inside)
        .padding([3, 10])
        .style(move |_| container::Style { background: Some(Background::Color(Color::from_rgba(0.05, 0.02, 0.03, 0.66 * k))), border: Border { radius: 11.0.into(), ..Border::default() }, ..container::Style::default() })
        .into()
}

pub(super) fn word_plate<'a>(said: String) -> Element<'a, Message> {
    plate(text(said).font(theme::SANS_SEMI).size(12.0).color(ui::faded(INK)).into())
}

pub(super) fn tile<'a>(thumbs: &HashMap<String, image::Handle>, hash: Option<&str>, wide: f32, high: f32, round: f32) -> Element<'a, Message> {
    let k = ui::fade();
    match hash.and_then(|hash| thumbs.get(hash)) {
        Some(handle) => image(crate::crops::fitted(handle, wide, high, round)).content_fit(iced::ContentFit::Fill).width(Length::Fill).height(high).border_radius(round).opacity(k).into(),
        None => container(Space::new())
            .width(Length::Fill)
            .height(high)
            .style(move |_| container::Style { background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.03 * k))), border: Border { radius: round.into(), ..Border::default() }, ..container::Style::default() })
            .into(),
    }
}

pub(super) fn mosaic<'a>(thumbs: &HashMap<String, image::Handle>, hashes: &[Option<String>], across: usize, down: usize, wide: f32, high: f32, gap: f32, round: f32) -> Element<'a, Message> {
    let each_wide = ((wide - gap * (across as f32 - 1.0)) / across as f32).max(1.0);
    let each_high = ((high - gap * (down as f32 - 1.0)) / down as f32).max(1.0);
    let mut lines = column![].spacing(gap);
    for line in 0..down {
        let mut tiles = row![].spacing(gap);
        for at in 0..across {
            let spot = line * across + at;
            let hash = match hashes.len() {
                0 => None,
                held if held >= across => hashes[spot % held].as_deref(),
                _ => hashes.get(spot).and_then(|hash| hash.as_deref()),
            };
            tiles = tiles.push(tile(thumbs, hash, each_wide, each_high, round));
        }
        lines = lines.push(tiles);
    }
    lines.into()
}

pub(super) fn makers_line<'a>(state: &'a State, names: &[String], words: &Words, size: f32) -> Element<'a, Message> {
    if names.is_empty() {
        return container(faded_text(words.t("shelf-no-makers"), size, FAINT)).height(FACE + 4.0).center_y(FACE + 4.0).into();
    }
    let faces = overlapped(names.iter().take(3).map(|name| ringed(face(state, name), FACE / 2.0)).collect(), 18.0, FACE + 4.0);
    row![faces, container(text(names.join(", ")).font(theme::SANS_SEMI).size(size).color(ui::faded(INK)).wrapping(text::Wrapping::None)).width(Length::Fill).clip(true)].spacing(8).align_y(iced::Center).into()
}

pub(super) fn fact<'a>(value: String, label: String) -> Element<'a, Message> {
    column![text(value).font(theme::MONO_BOLD).size(15.0).color(ui::faded(INK)), faded_text(label, 12.0, MUTED)].spacing(1).into()
}

pub(super) fn mod_strip<'a>(mods: &[(Mod, usize)], legend: bool) -> Element<'a, Message> {
    let k = ui::fade();
    let mut bar = row![].spacing(3);
    let mut said = row![].spacing(12);
    for (mods, count) in mods {
        let tint = mod_tint(*mods);
        bar = bar.push(container(Space::new().height(5.0)).width(Length::FillPortion(*count as u16)).style(move |_| container::Style { background: Some(Background::Color(Color { a: k, ..tint })), border: Border { radius: 2.5.into(), ..Border::default() }, ..container::Style::default() }));
        said = said.push(row![text(mods.code()).font(theme::MONO_BOLD).size(11.0).color(ui::faded(tint)), text(count.to_string()).font(theme::MONO).size(11.0).color(ui::faded(MUTED))].spacing(4));
    }
    if mods.is_empty() {
        return Space::new().height(0.0).into();
    }
    if legend { column![bar, said].spacing(6).into() } else { bar.into() }
}

pub(super) fn card_style(_: &iced::Theme, status: button::Status) -> button::Style {
    let hot = matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        background: Some(Background::Color(CARD_FILL)),
        text_color: INK,
        border: Border { color: crate::theme::outline(soft(if hot { 0.3 } else { 0.12 }, 1.0)), width: 1.0, radius: 16.0.into() },
        shadow: iced::Shadow::default(),
        snap: true,
    }
}

pub(super) fn shade<'a>(from: f32) -> Element<'a, Message> {
    let k = ui::fade();
    let fade = iced::gradient::Linear::new(iced::Radians(std::f32::consts::PI))
        .add_stop(0.0, Color { a: from * k, ..CARD_FILL })
        .add_stop(0.5, Color { a: 0.55 * k, ..CARD_FILL })
        .add_stop(1.0, Color { a: k, ..CARD_FILL });
    container(Space::new()).width(Length::Fill).height(Length::Fill).style(move |_| container::Style { background: Some(Background::Gradient(fade.into())), ..container::Style::default() }).into()
}

pub(super) fn pool_card<'a>(state: &'a State, shown: Shown, mark: Element<'a, Message>, foot: Option<Element<'a, Message>>, press: Option<Message>, wide: f32, words: &'a Words, thumbs: &'a HashMap<String, image::Handle>) -> Element<'a, Message> {
    let k = ui::fade();
    let top = row![mark, ui::grow(), plate(text(shown.when.clone()).font(theme::MONO).size(12.0).color(ui::faded(INK)).into())].spacing(8).align_y(iced::Center);
    let title = text(shown.title.clone()).font(theme::SANS_SEMI).size(22.0).color(ui::faded(INK)).wrapping(text::Wrapping::None);
    let head = iced::widget::stack![
        container(match shown.backing.as_ref().and_then(|hash| thumbs.get(hash)) {
            Some(picture) => Element::from(image(crate::crops::fitted(picture, wide - 22.0, CARD_TOP - 10.0, 6.0)).content_fit(iced::ContentFit::Fill).width(Length::Fill).height(CARD_TOP - 10.0).opacity(k)),
            None => mosaic(thumbs, &shown.hashes, 4, 2, wide - 22.0, CARD_TOP - 10.0, 4.0, 6.0),
        }).padding(Padding { left: 10.0, right: 10.0, top: 10.0, bottom: 0.0 }),
        shade(0.0),
        container(top).padding([18, 18]).width(Length::Fill),
        container(container(title).width(Length::Fill).clip(true)).padding(Padding { left: 16.0, right: 16.0, bottom: 4.0, top: 0.0 }).height(Length::Fill).align_y(iced::alignment::Vertical::Bottom),
    ]
    .width(Length::Fill)
    .height(CARD_TOP);
    let mut facts = row![].spacing(18).align_y(iced::alignment::Vertical::Bottom);
    facts = facts.push(ui::grow()).push(stars_span(words, shown.low, shown.high));
    let mut body = if shown.plain { column![facts].spacing(12) } else { column![makers_line(state, &shown.makers, words, 14.0), facts, mod_strip(&shown.mods, true)].spacing(12) };
    if let Some(foot) = foot {
        let line = container(Space::new().height(1.0)).width(Length::Fill).style(move |_| container::Style { background: Some(Background::Color(soft(0.08, k))), ..container::Style::default() });
        body = body.push(column![line, foot].spacing(10));
    }
    let inside = column![container(head).clip(true), container(body).padding(Padding { left: 16.0, right: 16.0, top: 8.0, bottom: 14.0 })];
    let made = button(container(inside).clip(true)).padding(1).width(Length::FillPortion(1)).style(ui::button_faded(card_style));
    match press {
        Some(press) => made.on_press(press).into(),
        None => made.into(),
    }
}

pub(super) fn hero<'a>(state: &'a State, pool: &'a Pool, words: &'a Words, thumbs: &'a HashMap<String, image::Handle>, width: f32, today: i64) -> Element<'a, Message> {
    let shown = shown_of(pool, words, words.day(pool.changed_at, today));
    let beside = width >= 900.0;
    let side = if beside { (width * 0.4).clamp(320.0, 520.0) } else { 0.0 };
    let mark: Element<'a, Message> = match (&pool.saved, pool.published_revision) {
        (Some(_), _) => word_plate(words.t("shelf-mark-saved")),
        (None, 0) => word_plate(words.t("shelf-draft")),
        (None, _) => word_plate(words.t("shelf-mark-published")),
    };
    let mut facts = row![].spacing(28).align_y(iced::alignment::Vertical::Bottom);
    if shown.low.is_some() {
        facts = facts.push(column![stars_span(words, shown.low, shown.high), faded_text(words.t("shelf-stars"), 12.0, MUTED)].spacing(3));
    }
    facts = facts.push(fact(shown.when.clone(), words.t("shelf-changed")));
    let left = column![
        row![mark].spacing(10).align_y(iced::Center),
        container(text(shown.title.clone()).font(theme::SANS_SEMI).size(30.0).color(ui::faded(INK)).wrapping(text::Wrapping::None)).width(Length::Fill).clip(true),
        makers_line(state, &shown.makers, words, 14.0),
        Space::new().height(Length::Fill),
        facts,
        mod_strip(&shown.mods, true),
    ]
    .spacing(14)
    .width(Length::Fill);
    let mut inside = row![left].spacing(32);
    if beside {
        let held = shown.backing.as_ref().and_then(|hash| thumbs.get(hash));
        let tone = held.and_then(crate::crops::tone);
        let k = ui::fade();
        inside = inside.push(container(match held {
            Some(picture) => Element::from(image(crate::crops::fitted(picture, side - 16.0, 218.0, 10.0)).content_fit(iced::ContentFit::Fill).width(side - 16.0).height(218.0).opacity(ui::fade())),
            None => mosaic(thumbs, &shown.hashes, 4, 3, side, 234.0, 8.0, 8.0),
        }).padding(if held.is_some() { 8.0 } else { 0.0 }).width(side).style(move |_| match tone {
            Some(tone) => container::Style { background: Some(Background::Color(Color { a: k, ..ui::mix(Color::from_rgb8(tone[0], tone[1], tone[2]), theme::GROUND, 0.45) })), border: Border { radius: 14.0.into(), ..Border::default() }, ..container::Style::default() },
            None => container::Style::default(),
        }));
    }
    button(container(inside).padding([26, 30]).height(288.0))
        .padding(1)
        .width(Length::Fill)
        .style(ui::button_faded(|_: &iced::Theme, status: button::Status| {
            let hot = matches!(status, button::Status::Hovered | button::Status::Pressed);
            let fade = iced::gradient::Linear::new(iced::Radians(std::f32::consts::FRAC_PI_2))
                .add_stop(0.0, Color::from_rgb(0.105, 0.043, 0.062))
                .add_stop(1.0, Color::from_rgb(0.20, 0.07, 0.10));
            button::Style { background: Some(Background::Gradient(fade.into())), text_color: INK, border: Border { color: crate::theme::outline(soft(if hot { 0.3 } else { 0.12 }, 1.0)), width: 1.0, radius: 20.0.into() }, shadow: iced::Shadow::default(), snap: true }
        }))
        .on_press(Message::Open(pool.id.clone()))
        .into()
}

pub(super) fn tiled<'a>(cards: Vec<Element<'a, Message>>, across: usize) -> Element<'a, Message> {
    let mut lines = column![].spacing(CARD_GAP);
    let mut line = row![].spacing(CARD_GAP);
    let mut held = 0;
    for card in cards {
        line = line.push(card);
        held += 1;
        if held == across {
            lines = lines.push(line);
            line = row![].spacing(CARD_GAP);
            held = 0;
        }
    }
    if held > 0 {
        for _ in held..across {
            line = line.push(Space::new().width(Length::FillPortion(1)));
        }
        lines = lines.push(line);
    }
    lines.into()
}

pub(super) fn missing_of(state: &State, pool: &Pool) -> Option<usize> {
    let songs = state.songs.as_ref()?;
    Some(pool.slots.iter().filter_map(|slot| slot.hash.as_ref()).filter(|hash| !songs.contains_key(*hash)).count())
}

pub(super) fn game_tile<'a>(found: &'a Collection, at: usize, words: &'a Words, thumbs: &'a HashMap<String, image::Handle>, wide: f32) -> Element<'a, Message> {
    let covers: Vec<Option<String>> = found.hashes.iter().take(4).cloned().map(Some).collect();
    let name = if found.name.is_empty() { words.t("collections-new") } else { found.name.clone() };
    let inside = row![
        container(mosaic(thumbs, &covers, 2, 2, 84.0, 52.0, 2.0, 4.0)).width(84.0),
        column![container(text(name).font(theme::SANS_SEMI).size(15.0).color(ui::faded(INK)).wrapping(text::Wrapping::None)).width(Length::Fill).clip(true), faded_text(words.n("shelf-maps", found.hashes.len() as u64), 13.0, MUTED)].spacing(3).width(Length::Fill),
    ]
    .spacing(12)
    .align_y(iced::Center);
    let _ = wide;
    button(container(inside).padding([10, 12])).padding(1).width(Length::FillPortion(1)).style(ui::button_faded(card_style)).on_press(Message::ImportCollection(at)).into()
}

pub(super) fn collections_showcase<'a>(state: &'a State, mut page: iced::widget::Column<'a, Message>, words: &'a Words, thumbs: &'a HashMap<String, image::Handle>, width: f32, t: f32, clocks: &Clocks) -> Element<'a, Message> {
    let across = if width >= 960.0 { 3 } else if width >= 620.0 { 2 } else { 1 };
    let wide = ((width - CARD_GAP * (across as f32 - 1.0)) / across as f32).floor();
    let caption = |key: &str| -> Element<'a, Message> { container(ui::mono_small(words.t(key).to_uppercase(), MUTED)).padding(Padding { top: 6.0, ..Padding::ZERO }).into() };
    let note = |key: &str| -> Element<'a, Message> { container(faded_text(words.t(key), 14.0, MUTED)).padding([6, 2]).into() };
    let mine: Vec<&Pool> = state.list.iter().filter(|pool| pool.collection && pool.saved.is_none()).collect();
    page = page.push(ui::appearing(ui::appear(t, 1), 8.0, || caption("shelf-own-collections")));
    if mine.is_empty() {
        page = page.push(ui::appearing(ui::appear(t, 1), 8.0, || note("shelf-empty-collections")));
    }
    let cards = mine.iter().enumerate().map(|(at, pool)| {
        let mark = match pool.published_revision {
            0 => word_plate(words.t("shelf-draft")),
            _ => word_plate(words.t("shelf-mark-published")),
        };
        ui::appearing(ui::appear(t, 2 + at / across), 10.0, || pool_card(state, shown_of(pool, words, words.day(pool.changed_at, clocks.today)), mark, None, Some(Message::Open(pool.id.clone())), wide, words, thumbs))
    }).collect();
    page = page.push(tiled(cards, across));
    if let Some(found) = state.collections.as_ref().filter(|found| !found.is_empty()) {
        page = page.push(ui::appearing(ui::appear(t, 3), 8.0, || caption("catalogue-game-collections")));
        let tiles = found.iter().enumerate().take(GAME_MOST).map(|(at, found)| ui::appearing(ui::appear(t, 3 + at / across), 8.0, || game_tile(found, at, words, thumbs, wide))).collect();
        page = page.push(tiled(tiles, across));
    }
    page = page.push(ui::appearing(ui::appear(t, 4), 8.0, || row![caption("catalogue-published"), ui::grow(), quiet_button(words.t("catalogue-refresh"), Message::Catalogue(true))].align_y(iced::Center).into()));
    let items: Vec<&crate::bot::Publication> = state.publications.iter().filter(|item| item.kind == "collection").collect();
    if state.catalogue_loading && items.is_empty() {
        page = page.push(ui::appearing(ui::appear(t, 5), 8.0, || note("catalogue-loading")));
    } else if items.is_empty() {
        page = page.push(ui::appearing(ui::appear(t, 5), 8.0, || note("catalogue-empty")));
    }
    let cards = items.iter().enumerate().map(|(at, item)| {
        let hashes = published_hashes(item);
        let shown = Shown { title: item.name.clone(), makers: Vec::new(), hashes, low: None, high: None, mods: Vec::new(), when: format!("#{}", item.revision), plain: true, backing: None };
        let held = state.list.iter().any(|pool| pool.saved.as_ref().is_some_and(|saved| saved.id == item.id));
        let action: Element<'a, Message> = if item.mine {
            Space::new().into()
        } else if held {
            plate(text(words.t("shelf-saved-mark")).font(theme::SANS_SEMI).size(12.0).color(ui::faded(Color::from_rgb(0.765, 0.831, 0.651))).into())
        } else {
            outlined(words.t("shelf-save"), Some(Message::SavePublication(item.id.clone())))
        };
        let owner: Element<'a, Message> = if item.mine { faded_text(words.t("shelf-yours"), 13.0, MUTED) } else { Space::new().into() };
        let foot = row![container(owner).width(Length::Fill), action].spacing(8).align_y(iced::Center);
        let mark = if item.code.is_empty() { Space::new().into() } else { plate(text(item.code.clone()).font(theme::MONO_BOLD).size(12.0).color(ui::faded(INK)).into()) };
        let seen = t.min(clocks.of(&item.id));
        ui::appearing(ui::appear(seen, 5 + at / across), 10.0, || pool_card(state, shown, mark, Some(foot.into()), item.mine.then(|| Message::OpenPublication(item.id.clone())), wide, words, thumbs))
    }).collect();
    page = page.push(tiled(cards, across));
    if let Some(error) = &state.catalogue_error {
        page = page.push(ui::appearing(ui::appear(t, 5), 8.0, || catalogue_error(error, words)));
    }
    if state.catalogue_more {
        page = page.push(quiet_button(words.t("catalogue-more"), Message::CatalogueMore));
    }
    page.into()
}

pub(super) fn small_primary<'a>(label: String, press: Option<Message>) -> Element<'a, Message> {
    let made = button(container(text(label).font(theme::SANS_SEMI).size(14.0).wrapping(text::Wrapping::None)).center_y(CONTROL))
        .padding([0, 12])
        .style(ui::button_faded(|_: &iced::Theme, status: button::Status| {
            let lit = matches!(status, button::Status::Hovered | button::Status::Pressed);
            let quiet = matches!(status, button::Status::Disabled);
            button::Style { background: Some(Background::Color(if quiet { soft(0.08, 1.0) } else if lit { ACCENT } else { Color::from_rgb(0.788, 0.227, 0.227) })), text_color: if quiet { FAINT } else { Color::WHITE }, border: Border { radius: 8.0.into(), ..Border::default() }, shadow: iced::Shadow::default(), snap: true }
        }));
    match press {
        Some(press) => made.on_press(press).into(),
        None => made.into(),
    }
}

pub(super) fn megabytes(words: &Words, bytes: u64) -> String {
    let value = format!("{:.1}", bytes as f64 / 1_048_576.0);
    if words.lang() == crate::lang::Lang::Ru { value.replace('.', ",") } else { value }
}

pub(super) fn saved_foot<'a>(state: &'a State, pool: &'a Pool, words: &'a Words) -> Element<'a, Message> {
    let good = Color::from_rgb(0.765, 0.831, 0.651);
    let said = |line: String, ink: Color| -> Element<'a, Message> { container(text(line).font(theme::SANS).size(13.0).color(ui::faded(ink))).width(Length::Fill).into() };
    let pull = state.pull.as_ref().filter(|pull| pull.pool == pool.id);
    let free = state.fetching.is_none() && !state.pull.as_ref().is_some_and(Pull::running);
    let again = |label: &str| outlined(words.t(label), free.then(|| Message::Pull(pool.id.clone())));
    let newer = state.newer_of(pool);
    let line: iced::widget::Row<'a, Message> = match (pull, missing_of(state, pool)) {
        (running, _) if newer.is_some() && !running.is_some_and(Pull::running) => {
            let held: HashSet<&str> = pool.slots.iter().filter_map(|slot| slot.hash.as_deref()).collect();
            let added = newer.map_or(0, |item| published_hashes(item).into_iter().flatten().filter(|hash| !held.contains(hash.as_str())).count());
            let told = if added > 0 { words.n("shelf-renew-added", added as u64) } else { words.t("shelf-renew-changed") };
            row![said(told, INK), small_primary(words.t("shelf-renew"), Some(Message::Renew(pool.id.clone())))]
        }
        (Some(pull), _) if pull.running() => {
            let part = match &pull.step {
                Some(crate::maps::Step::Downloading { done, total: Some(total), .. }) if *total > 0 => Some((*done as f32 / *total as f32, format!("{} / {} {}", megabytes(words, *done), megabytes(words, *total), words.t("shelf-megabytes")))),
                _ => None,
            };
            let fraction = ((pull.done() as f32 + part.as_ref().map_or(0.0, |(part, _)| *part)) / pull.total.max(1) as f32).clamp(0.02, 1.0);
            let mut line = row![text(words.of(pull.done() as u64 + 1, pull.total as u64)).font(theme::MONO_BOLD).size(13.0).color(ui::faded(INK)), container(crate::billing::bar(fraction, ACCENT)).width(Length::Fill)];
            if let Some((_, bytes)) = part {
                line = line.push(text(bytes).font(theme::MONO).size(12.0).color(ui::faded(MUTED)));
            }
            line.push(outlined(words.t("shelf-stop"), Some(Message::StopPull)))
        }
        (Some(Pull { over: Some(Err(why)), .. }), _) => {
            let key = match why.as_str() { "running" => "shelf-collect-running", "none" => "shelf-collect-none", "empty" => "shelf-collect-empty", _ => "shelf-collect-failed" };
            row![said(words.t(key), Color::from_rgb(1.0, 0.61, 0.61)), again("shelf-again")]
        }
        (Some(Pull { over: Some(Ok(_)), lost, .. }), _) if *lost > 0 => row![said(words.n("shelf-lost", *lost as u64), Color::from_rgb(1.0, 0.61, 0.61)), again("shelf-again")],
        (Some(Pull { over: None, .. }), _) => row![said(words.t("shelf-collecting"), MUTED)],
        (_, None) => row![said(words.t("shelf-checking"), FAINT)],
        (_, Some(0)) if pool.saved.as_ref().is_some_and(|saved| saved.collected) => row![glyph(Icon::Check, 13.0, good), said(words.t("shelf-collected"), good)],
        (_, Some(0)) => row![glyph(Icon::Check, 13.0, good), said(words.t("shelf-all-here"), good), again("shelf-collect")],
        (_, Some(missing)) => row![said(words.n("shelf-missing", missing as u64), MUTED), small_primary(words.t("shelf-pull"), free.then(|| Message::Pull(pool.id.clone())))],
    };
    row![Space::new().height(36.0), container(line.spacing(10).align_y(iced::Center)).width(Length::Fill)].align_y(iced::Center).into()
}

pub(super) fn showcase<'a>(state: &'a State, mut page: iced::widget::Column<'a, Message>, words: &'a Words, thumbs: &'a HashMap<String, image::Handle>, width: f32, t: f32, clocks: &Clocks) -> Element<'a, Message> {
    let across = if width >= 960.0 { 3 } else if width >= 620.0 { 2 } else { 1 };
    let wide = ((width - CARD_GAP * (across as f32 - 1.0)) / across as f32).floor();
    let note = |key: &str| -> Element<'a, Message> { container(faded_text(words.t(key), 14.0, MUTED)).padding([18, 2]).into() };
    let mark_of = |pool: &Pool| -> Element<'a, Message> {
        match (&pool.saved, pool.published_revision) {
            (Some(_), _) => word_plate(words.t("shelf-mark-saved")),
            (None, 0) => word_plate(words.t("shelf-draft")),
            (None, _) => word_plate(words.t("shelf-mark-published")),
        }
    };
    match state.shelf {
        Shelf::Mine => {
            let mine: Vec<&Pool> = state.list.iter().filter(|pool| !pool.collection && pool.saved.is_none()).collect();
            match mine.split_first() {
                None => page = page.push(ui::appearing(ui::appear(t, 1), 8.0, || note("shelf-empty-mine"))),
                Some((first, rest)) => {
                    page = page.push(ui::appearing(ui::appear(t, 1), 10.0, || hero(state, first, words, thumbs, width, clocks.today)));
                    let cards = rest.iter().enumerate().map(|(at, pool)| ui::appearing(ui::appear(t, 2 + at / across), 10.0, || pool_card(state, shown_of(pool, words, words.day(pool.changed_at, clocks.today)), mark_of(pool), None, Some(Message::Open(pool.id.clone())), wide, words, thumbs))).collect();
                    page = page.push(tiled(cards, across));
                }
            }
        }
        Shelf::Saved => {
            let saved: Vec<&Pool> = state.list.iter().filter(|pool| pool.saved.is_some()).collect();
            let mark_of = |pool: &Pool| -> Element<'a, Message> {
                match state.newer_of(pool) {
                    Some(item) => {
                        let k = ui::fade();
                        let out = container(text(words.with("shelf-out", &[("n", item.revision.to_string())])).font(theme::MONO_BOLD).size(12.0).color(ui::faded(Color::from_rgb(1.0, 0.85, 0.66))))
                            .padding([3, 10])
                            .style(move |_| container::Style { background: Some(Background::Color(Color::from_rgba(0.84, 0.537, 0.31, 0.34 * k))), border: Border { radius: 11.0.into(), ..Border::default() }, ..container::Style::default() });
                        let mut marks = row![mark_of(pool), out].spacing(6).align_y(iced::Center);
                        if pool.collection {
                            marks = marks.push(word_plate(words.t("shelf-collection")));
                        }
                        marks.into()
                    }
                    None if pool.collection => row![mark_of(pool), word_plate(words.t("shelf-collection"))].spacing(6).align_y(iced::Center).into(),
                    None => mark_of(pool),
                }
            };
            if saved.is_empty() {
                page = page.push(ui::appearing(ui::appear(t, 1), 8.0, || note("shelf-empty-saved")));
            }
            let cards = saved.iter().enumerate().map(|(at, pool)| {
                let foot = saved_foot(state, pool, words);
                ui::appearing(ui::appear(t, 1 + at / across), 10.0, || pool_card(state, shown_of(pool, words, words.day(pool.changed_at, clocks.today)), mark_of(pool), Some(foot), Some(Message::Open(pool.id.clone())), wide, words, thumbs))
            }).collect();
            page = page.push(tiled(cards, across));
        }
        Shelf::Published => {
            let items: Vec<&crate::bot::Publication> = state.publications.iter().filter(|item| item.kind == "pool").collect();
            if state.catalogue_loading && items.is_empty() {
                page = page.push(ui::appearing(ui::appear(t, 1), 8.0, || note("catalogue-loading")));
            } else if items.is_empty() {
                page = page.push(ui::appearing(ui::appear(t, 1), 8.0, || note("catalogue-empty")));
            }
            let cards = items.iter().enumerate().filter_map(|(at, item)| {
                let pool = serde_json::to_vec(&item.content).ok().and_then(|bytes| pool_share::from_file(&bytes, 0).ok())?;
                let mut shown = shown_of(&pool, words, String::new());
                shown.title = item.name.clone();
                shown.when = format!("#{}", item.revision);
                let held = state.list.iter().any(|pool| pool.saved.as_ref().is_some_and(|saved| saved.id == item.id));
                let action: Element<'a, Message> = if item.mine {
                    Space::new().into()
                } else if held {
                    plate(text(words.t("shelf-saved-mark")).font(theme::SANS_SEMI).size(12.0).color(ui::faded(Color::from_rgb(0.765, 0.831, 0.651))).into())
                } else {
                    outlined(words.t("shelf-save"), Some(Message::SavePublication(item.id.clone())))
                };
                let publisher: Element<'a, Message> = match pool.compiler.as_str() {
                    "" => Space::new().width(Length::Fill).into(),
                    name => row![ui::mono_small(words.t("shelf-published-by").to_uppercase(), MUTED), container(named_face(state, name, 120.0)).width(Length::Fill).clip(true)].spacing(8).align_y(iced::Center).width(Length::Fill).into(),
                };
                let foot = row![publisher, action].spacing(8).align_y(iced::Center);
                let mark = if item.code.is_empty() { Space::new().into() } else { plate(text(item.code.clone()).font(theme::MONO_BOLD).size(12.0).color(ui::faded(INK)).into()) };
                let seen = t.min(clocks.of(&item.id));
                Some(ui::appearing(ui::appear(seen, 1 + at / across), 10.0, || pool_card(state, shown, mark, Some(foot.into()), item.mine.then(|| Message::OpenPublication(item.id.clone())), wide, words, thumbs)))
            }).collect();
            page = page.push(tiled(cards, across));
            if let Some(error) = &state.catalogue_error {
                page = page.push(ui::appearing(ui::appear(t, 2), 8.0, || catalogue_error(error, words)));
            }
            if state.catalogue_more {
                page = page.push(quiet_button(words.t("catalogue-more"), Message::CatalogueMore));
            }
        }
    }
    page.into()
}

pub(super) fn catalogue_error<'a>(error: &str, words: &Words) -> Element<'a, Message> {
    let said = words.t(match error { "401" => "catalogue-sign-in", "403" => "catalogue-profile-required", "409" => "catalogue-conflict", "400" => "catalogue-invalid", _ => "catalogue-unavailable" });
    let k = ui::fade();
    container(row![glyph(Icon::Warn, 15.0, ACCENT), faded_text(said, 13.0, ACCENT)].spacing(10).align_y(iced::Center))
        .padding([8, 14])
        .style(move |_| container::Style {
            background: Some(Background::Color(Color::from_rgba(0.886, 0.282, 0.282, 0.018 * k))),
            border: Border { color: crate::theme::outline(Color::from_rgba(0.886, 0.282, 0.282, 0.16 * k)), width: 1.0, radius: 10.0.into() },
            ..container::Style::default()
        })
        .into()
}

