use super::*;
use iced::widget::column;

impl Main {
    pub fn write_notice(&mut self, mark: notices::Mark, words: String, detail: String, note: String, map_hash: String, link: notices::Link) -> u64 {
        if mark == notices::Mark::Bad && !self.gallery {
            crate::notices::log(&words, &detail, &note);
        }
        let id = self.notices.push(mark, words, detail, note, map_hash, link);
        self.scenes_due = true;
        self.bell_flash = Animation::new(true).duration(BELL_FLASH).easing(Easing::Linear).go(false, Instant::now());
        if self.menu == Some(Tab::Bell) {
            self.arrivals.insert(id, Animation::new(false).duration(NOTICE_ARRIVE).easing(Easing::EaseOutCubic).go(true, Instant::now()));
        }
        id
    }

    pub fn announce(&mut self, mark: notices::Mark, words: String, detail: String, note: String, map_hash: String, link: notices::Link) {
        let id = self.notices.push(mark, words, detail, note, map_hash, link);
        self.scenes_due = true;
        self.bell_flash = Animation::new(true).duration(BELL_FLASH).easing(Easing::Linear).go(false, Instant::now());
        let now = Instant::now();
        if self.menu == Some(Tab::Bell) {
            self.arrivals.insert(id, Animation::new(false).duration(NOTICE_ARRIVE).easing(Easing::EaseOutCubic).go(true, now));
            return;
        }
        while self.toasts.iter().filter(|t| t.shown.value()).count() >= self.toast_capacity() {
            if let Some(oldest) = self.toasts.iter_mut().find(|t| t.shown.value()) {
                oldest.shown.go_mut(false, now);
            } else {
                break;
            }
        }
        self.toasts.push(Toast {
            id,
            shown: Animation::new(false).duration(TOAST_IN).easing(Easing::EaseOutCubic).go(true, now),
            born: now,
            hovered: false,
            paused_at: None,
            stays: mark == notices::Mark::Bad,
        });
    }

    pub(super) fn bell_button(&self) -> Element<'_, Message> {
        let open = self.menu == Some(Tab::Bell) && self.menu_open.value();
        let unseen = self.notices.unseen();
        let flash = (std::f32::consts::PI * self.bell_flash.interpolate(0.0, 1.0, self.now)).sin().max(0.0);
        let busy = self.busy();
        let beat = 0.5 - 0.5 * (std::f32::consts::TAU * self.now.saturating_duration_since(self.section_at).as_secs_f32() / BELL_PULSE).cos();
        let face = button(container(crate::glyphs::glyph(crate::glyphs::Icon::Bell, 16.0, if open { Color::WHITE } else { INK })).center(CIRCLE_SIDE))
            .padding(0)
            .width(CIRCLE_SIDE)
            .height(CIRCLE_SIDE)
            .style(ui::button_faded(move |_: &iced::Theme, status: button::Status| {
                let lit = matches!(status, button::Status::Hovered | button::Status::Pressed);
                button::Style {
                    background: Some(iced::Background::Color(if open { Color { a: 0.3, ..ACCENT } } else if flash > 0.001 { Color { a: 0.06 + 0.44 * flash, ..ACCENT } } else { Color::from_rgba(1.0, 1.0, 1.0, if lit { 0.12 } else { 0.05 }) })),
                    text_color: INK,
                    border: iced::Border { color: crate::theme::outline(if open || busy || flash > 0.001 { Color { a: if open { 0.7 } else if busy { 0.3 + 0.6 * beat } else { 0.12 + 0.7 * flash }, ..if open || busy { ACCENT } else { ui::mix(Color::WHITE, ACCENT, flash.min(1.0)) } } } else { Color::from_rgba(1.0, 1.0, 1.0, 0.07) }), width: 1.0, radius: (CIRCLE_SIDE / 2.0).into() },
                    shadow: iced::Shadow::default(),
                    snap: true,
                }
            }))
            .on_press(Message::MenuTab(Tab::Bell));
        let face: Element<'_, Message> = face.into();
        if unseen == 0 || open {
            return face;
        }
        let count = if unseen > 9 { "9+".to_owned() } else { unseen.to_string() };
        let wide = (ui::text_width(&count, theme::MONO_BOLD, 11.0).ceil() + 10.0).max(BELL_BADGE);
        let badge = container(text(count).font(theme::MONO_BOLD).size(11.0).line_height(iced::widget::text::LineHeight::Absolute(BELL_BADGE.into())).wrapping(text::Wrapping::None).color(ui::faded(Color::WHITE)))
            .center_x(wide)
            .center_y(BELL_BADGE)
            .style(ui::box_faded(|_| container::Style {
                background: Some(iced::Background::Color(ACCENT)),
                border: iced::Border { radius: (BELL_BADGE / 2.0).into(), ..iced::Border::default() },
                ..container::Style::default()
            }));
        stack![face, pin(badge).x(CIRCLE_SIDE + 4.0 - wide).y(-4.0)].width(CIRCLE_SIDE + 6.0).height(CIRCLE_SIDE).into()
    }

    pub(super) fn bell_panel(&self) -> Element<'_, Message> {
        let w = &self.words;
        let mut head = row![text(w.t("bell-title")).font(theme::SANS_SEMI).size(16.0).color(ui::faded(INK)), ui::grow()].align_y(iced::Center);
        if !self.notices.notices.is_empty() {
            head = head.push(ui::quiet(w.t("bell-clear"), Some(Message::ClearNotices)));
        }
        let mut list = column![].spacing(0).width(Length::Fill);
        for tile in self.bell_tiles() {
            list = list.push(tile);
        }
        let height = (self.height - MENU_TOP - 110.0).max(140.0);
        let scroll = scrollable(list).direction(ui::hidden_bar()).height(Length::Shrink).width(Length::Fill);
        column![container(head).height(36.0).align_y(iced::Center), container(scroll).max_height(height)].spacing(10).padding(16).into()
    }

    pub(super) fn scenes_for_notices(&self) -> Task<Message> {
        let wanted: Vec<(String, PathBuf)> = self
            .notices
            .notices
            .iter()
            .take(6)
            .filter(|n| !n.map_hash.is_empty() && !self.scenes.contains_key(&n.map_hash))
            .filter_map(|n| self.background_for(&n.map_hash).map(|bg| (n.map_hash.clone(), bg)))
            .collect();
        if wanted.is_empty() {
            return Task::none();
        }
        ui::streamed(move |push| {
            for (hash, path) in wanted {
                let handle = decoded(&path, SCENE_WIDTH, None);
                if !push(Message::Scene(hash, handle)) {
                    return;
                }
            }
        })
    }

    pub(super) fn bell_tiles(&self) -> Vec<Element<'_, Message>> {
        let w = &self.words;
        let mut tiles = Vec::new();
        let job = |words: String, detail: String, fraction: f32| -> Element<'_, Message> {
            column![
                row![
                    ui::dot(8.0),
                    text(words).font(theme::SANS_SEMI).size(theme::CAPTION).color(ui::faded(INK)),
                    container(text(detail.to_owned()).font(theme::SANS).size(theme::CAPTION).wrapping(text::Wrapping::None).color(ui::faded(MUTED))).width(Length::Fill).clip(true),
                    text(format!("{:.0}%", fraction * 100.0)).font(theme::MONO_BOLD).size(theme::CAPTION).color(ui::faded(INK)),
                ]
                .spacing(8)
                .align_y(iced::Center)
                .height(20.0),
                ui::thread(fraction),
            ]
            .spacing(6)
            .into()
        };
        if let Some(rendering) = self.rendering.as_ref().filter(|r| !r.is_over()) {
            let who = self.entries().iter().find(|e| e.path == rendering.path).map(|e| e.title().unwrap_or_default()).unwrap_or_default();
            tiles.push(self.tile(job(w.t("drawing"), who, self.progress_shown)));
        }
        for queued in &self.queued {
            let who = self.entries().iter().find(|e| e.path == queued.path).map(|e| e.title().unwrap_or_default()).unwrap_or_default();
            tiles.push(self.tile(job(w.t("render-queued"), who, 0.0)));
        }
        for fetching in self.fetching.iter().filter(|f| !f.is_over()) {
            let title = self.entries().iter().find(|e| e.map_hash == fetching.hash).map(|e| e.title().unwrap_or_default()).unwrap_or_default();
            tiles.push(self.tile(job(w.t("fetch-downloading"), title, fetching.shown)));
        }
        if let Some(sending) = self.sending.as_ref().filter(|s| s.over.is_none()) {
            let title = self.store.videos.iter().find(|v| v.path == sending.path).map(|v| v.map_line()).unwrap_or_default();
            tiles.push(self.tile(job(w.t("sending"), title, self.progress_shown)));
        }
        if false {
            let clear = row![
                Space::new().width(Length::Fill),
                ui::quiet(w.t("clear-feed"), Some(Message::ClearNotices)),
            ]
            .align_y(iced::Center);
            tiles.push(self.card(container(clear).padding(Padding::ZERO.bottom(2.0)).into(), false));
        }
        let mut tiles: Vec<Element<'_, Message>> = tiles.into_iter().map(|tile| container(tile).padding(Padding::ZERO.bottom(8.0)).into()).collect();
        for notice in &self.notices.notices {
            let alive = match (self.leaving.get(&notice.id), self.arrivals.get(&notice.id)) {
                (Some(going), _) => going.interpolate(0.0, 1.0, self.now),
                (None, Some(coming)) => coming.interpolate(0.0, 1.0, self.now),
                _ => 1.0,
            };
            let tile = ui::fading(ui::fade() * alive, || {
                ui::grown(self.notification_card(notice, None, BELL_W - 32.0), Point::new(1.0, 0.5), 0.0, 1.0).shifted((1.0 - alive) * 24.0)
            });
            let going = self.leaving.contains_key(&notice.id);
            tiles.push(ui::collapsing(container(tile).padding(Padding::ZERO.bottom(8.0)), if going { alive } else { 1.0 }));
        }
        if tiles.is_empty() {
            tiles.push(container(text(w.t("bell-empty")).font(theme::SANS).size(14.0).color(ui::faded(MUTED))).width(Length::Fill).padding([28, 0]).center_x(Length::Fill).into());
        }
        tiles
    }

    pub(super) fn error_layer(&self) -> Element<'_, Message> {
        let Some(notice) = self.error_shown.and_then(|id| self.notices.get(id)) else {
            return Space::new().width(Length::Fill).height(Length::Fill).into();
        };
        let w = &self.words;
        let k = self.error_fade.interpolate(0.0, 1.0, self.now);
        ui::fading(ui::fade() * k, || {
            let fade = ui::fade();
            let mark = container(text("!").font(theme::MONO_BOLD).size(20.0).color(ui::faded(Color::WHITE)))
                .center(40.0)
                .style(move |_| container::Style { background: Some(iced::Background::Color(Color { a: fade, ..ACCENT })), border: iced::Border { radius: 20.0.into(), ..iced::Border::default() }, ..container::Style::default() });
            let mut head = column![text(notice.words.clone()).font(theme::SANS_SEMI).size(20.0).color(ui::faded(INK))].spacing(4).width(Length::Fill);
            if !notice.detail.is_empty() {
                head = head.push(text(notice.detail.clone()).font(theme::SANS).size(15.0).color(ui::faded(MUTED)));
            }
            let mut lines = column![row![mark, head].spacing(16).align_y(iced::Center)].spacing(18);
            if !notice.note.is_empty() {
                lines = lines.push(
                    container(text(notice.note.clone()).font(theme::MONO).size(14.0).color(ui::faded(INK)))
                        .padding([12, 14])
                        .width(Length::Fill)
                        .style(move |_| container::Style { background: Some(iced::Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.3 * fade))), border: iced::Border { color: crate::theme::outline(Color::from_rgba(1.0, 1.0, 1.0, 0.08 * fade)), width: 1.0, radius: 10.0.into() }, ..container::Style::default() }),
                );
            }
            let mut bottom = row![ui::mono_small(format!("{}  {}", w.day(notice.at, self.now_unix), w.clock(notice.at)), FAINT), ui::grow()].spacing(8).align_y(iced::Center);
            if matches!(notice.link, notices::Link::RenderAgain(_)) {
                bottom = bottom.push(ui::primary(w.t("once-more"), Some(Message::ToastLink(notice.id))));
            }
            bottom = bottom.push(crate::billing::outlined(w.t("close"), Some(Message::HideError)));
            let card = ui::glass_warm(container(lines.push(bottom)).padding(26), self.glass_picture(), iced::Size::new(self.width, self.height));
            let card = ui::grown(iced::widget::opaque(card), Point::new(0.5, 0.0), -(1.0 - k) * 12.0, 0.96 + 0.04 * k);
            stack![
                iced::widget::opaque(mouse_area(ui::veil(theme::SCRIM)).on_press(Message::HideError)),
                container(container(card).width(theme::COLUMN).padding(Padding::ZERO.top(180.0)))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x(Length::Fill),
            ]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
        })
    }

    pub(super) fn notice_mark(&self, notice: &notices::Notice, side: f32) -> Element<'_, Message> {
        let glyph = match notice.mark {
            notices::Mark::Done => "✓",
            notices::Mark::Bad => "!",
            notices::Mark::Plain => "i",
        };
        let tint = match notice.mark { notices::Mark::Bad => ACCENT, notices::Mark::Done => theme::NOTICE_SUCCESS, notices::Mark::Plain => theme::NOTICE_INFO };
        if notice.map_hash.is_empty() || !self.thumbs.contains_key(&notice.map_hash) {
            let sign: Element<'_, Message> = if notice.mark == notices::Mark::Done { crate::glyphs::glyph(crate::glyphs::Icon::Check, 20.0, ui::faded(tint)) } else { text(glyph).font(theme::MONO_BOLD).size(20.0).color(ui::faded(tint)).into() };
            return container(sign)
                .width(side).height(side).center(side)
                .style(ui::box_faded(move |_| container::Style {
                    background: Some(iced::Background::Color(Color { a: 0.08, ..tint })),
                    border: iced::Border { radius: (side / 2.0).into(), ..iced::Border::default() }, ..container::Style::default()
                })).into();
        }
        let picture: Element<'_, Message> = match self.thumbs.get(&notice.map_hash) {
            Some(handle) if !notice.map_hash.is_empty() => ui::clipped(image(handle.clone())
                .content_fit(ContentFit::Cover)
                .width(side)
                .height(side)
                .border_radius(8.0)
                .opacity(ui::fade() * 0.9)),
            _ => container(Space::new().width(side).height(side)).style(ui::box_faded(theme::chip)).into(),
        };
        let sign: Element<'_, Message> = if notice.mark == notices::Mark::Done { crate::glyphs::glyph(crate::glyphs::Icon::Check, 11.0, ui::faded(INK)) } else { text(glyph).font(theme::MONO_BOLD).size(11.0).color(ui::faded(INK)).into() };
        let badge = container(sign)
            .width(BADGE)
            .height(BADGE)
            .center(BADGE)
            .style(ui::box_faded(theme::badge_of(tint)));
        let reach = side + BADGE_OUT;
        stack![
            container(picture).width(reach).height(reach),
            pin(badge).x(side + BADGE_OUT - BADGE).y(side + BADGE_OUT - BADGE),
        ]
        .width(reach)
        .height(reach)
        .into()
    }

    pub(super) fn toast_capacity(&self) -> usize {
        (((self.height - TOAST_TOP - 24.0) / (TOAST_MAX_H + 10.0)).floor() as usize).clamp(1, TOASTS_AT_MOST)
    }

    pub(super) fn toast_layer(&self) -> Element<'_, Message> {
        let width = TOAST_W.min((self.width - 32.0).max(240.0));
        let home = (self.width - 40.0 - width).max(16.0);
        let mut cards = column![].width(width);
        for toast in &self.toasts {
            let Some(notice) = self.notices.get(toast.id) else { continue; };
            let k = toast.shown.interpolate(0.0, 1.0, self.now);
            let sensed = ui::fading(ui::fade() * k, || {
                ui::grown(mouse_area(iced::widget::opaque(self.notification_card(notice, Some(toast), width)))
                    .on_enter(Message::ToastHover(toast.id, true)).on_exit(Message::ToastHover(toast.id, false)),
                    Point::new(1.0, 0.5), 0.0, 1.0).shifted((1.0 - k) * 24.0)
            });
            cards = cards.push(ui::collapsing(container(sensed).padding(Padding::ZERO.bottom(10.0)), k));
        }
        pin(cards).x(home).y(TOAST_TOP).into()
    }

    pub(super) fn notification_card(&self, notice: &notices::Notice, toast: Option<&Toast>, width: f32) -> Element<'_, Message> {
        let bad = notice.mark == notices::Mark::Bad;
        let close = button(iced::widget::canvas(ui::Cross { colour: MUTED }).width(20.0).height(20.0))
            .padding(0).style(ui::button_faded(theme::notice_action(false)))
            .on_press(if toast.is_some() { Message::ToastClose(notice.id) } else { Message::DismissNotice(notice.id) });
        let title_limit = ((width - 92.0) / 7.0).floor().max(12.0) as usize;
        let title = ui::clipped(container(text(notice_preview(&notice.words, title_limit)).font(theme::SANS_SEMI).size(13.0)
            .wrapping(text::Wrapping::None).color(ui::faded(INK))).width(Length::Fill).height(20.0).center_y(20.0));
        let header = row![title, close].spacing(6).align_y(iced::Center);
        let detail = if notice.detail.is_empty() && bad { notice.note.clone() } else { notice.detail.clone() };
        let mut copy = column![header].spacing(4).width(Length::Fill);
        if !detail.is_empty() {
            let detail_limit = (((width - 70.0) / 6.7) * 3.0).floor().max(36.0) as usize;
            let description = container(text(notice_preview(&detail, detail_limit)).font(theme::SANS).size(12.0)
                .line_height(iced::Pixels(14.0)).wrapping(text::Wrapping::WordOrGlyph)
                .color(ui::faded(MUTED))).width(Length::Fill).max_height(42.0).clip(true);
            copy = copy.push(description);
        }
        let top = row![self.notice_mark(notice, 40.0), copy].spacing(10).align_y(iced::Center);
        let action = |label: String, message, primary| button(text(label).font(theme::SANS_SEMI).size(theme::CAPTION))
            .padding([3, 7]).style(ui::button_faded(theme::notice_action(primary))).on_press(message);
        let listed = toast.is_none();
        let solid = |label: String, message: Message, primary: bool| -> Element<'_, Message> {
            if primary { ui::primary(label, Some(message)) } else { crate::billing::outlined(label, Some(message)) }
        };
        let mut footer = if listed { row![Space::new().width(50.0)].spacing(8).align_y(iced::Center) } else { row![Space::new().width(Length::Fill)].spacing(4).align_y(iced::Center).height(24.0) };
        if bad && !listed {
            footer = footer.push(action(self.words.t("notice-details"), Message::ShowError(notice.id), false));
        }
        let link = match notice.link {
            notices::Link::OpenVideo(_) | notices::Link::Replay(_) | notices::Link::Received(_) | notices::Link::Duels => Some("open"),
            notices::Link::RenderAgain(_) => Some("once-more"),
            notices::Link::Update => Some("update-now"),
            notices::Link::Page(_) => Some("whats-new"),
            notices::Link::None => None,
        };
        if let Some(key) = link {
            footer = if listed { footer.push(solid(self.words.t(key), Message::ToastLink(notice.id), true)) } else { footer.push(action(self.words.t(key), Message::ToastLink(notice.id), true)) };
        }
        if bad && listed {
            footer = footer.push(solid(self.words.t("notice-details"), Message::ShowError(notice.id), false));
        }
        let mut content = column![top].spacing(if listed { 8 } else { 2 });
        if bad || link.is_some() { content = content.push(footer); }
        let hovered = toast.is_some_and(|t| t.hovered);
        let floating = toast.is_some();
        let face = container(content).padding([8, 10]).width(width).max_height(TOAST_MAX_H);
        let card = container(face)
            .width(width).max_height(TOAST_MAX_H).style(ui::box_faded(move |theme| match (floating, bad) {
                (true, _) => theme::notification(hovered)(theme),
                (false, true) => container::Style { background: Some(iced::Background::Color(Color { a: 0.14, ..ACCENT })), border: iced::Border { color: crate::theme::outline(Color { a: 0.4, ..ACCENT }), width: 1.0, radius: 12.0.into() }, ..container::Style::default() },
                (false, false) => container::Style { background: Some(iced::Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.24))), border: iced::Border { color: crate::theme::outline(Color::from_rgba(1.0, 1.0, 1.0, 0.05)), width: 1.0, radius: 12.0.into() }, ..container::Style::default() },
            })).clip(true);
        card.id(iced::widget::Id::new(if toast.is_some() { "toast-card" } else { "notice-card" })).into()
    }

    pub(super) fn handle_notifications(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SeenAll => {
                self.notices.see_all();
                Task::none()
            }
            Message::ClearNotices => {
                let now = Instant::now();
                self.error_shown = None;
                for toast in self.toasts.iter_mut() {
                    toast.shown.go_mut(false, now);
                }
                let ids: Vec<u64> = self.notices.notices.iter().map(|notice| notice.id).collect();
                for (at, id) in ids.into_iter().enumerate() {
                    let wait = CLEAR_STEP * at.min(CLEAR_STEPS) as u32;
                    self.leaving.insert(id, Animation::new(true).duration(CLEAR_LEAVE).delay(wait).easing(Easing::EaseOutCubic).go(false, now));
                }
                self.arrivals.clear();
                Task::none()
            }
            Message::ToastHover(id, over) => {
                if let Some(toast) = self.toasts.iter_mut().find(|t| t.id == id) {
                    toast.hover(over, Instant::now());
                }
                Task::none()
            }
            Message::ToastClose(id) => {
                let now = Instant::now();
                if let Some(toast) = self.toasts.iter_mut().find(|t| t.id == id) {
                    toast.shown.go_mut(false, now);
                }
                Task::none()
            }
            Message::DismissNotice(id) => {
                let now = Instant::now();
                if self.error_shown == Some(id) {
                    self.error_shown = None;
                }
                for toast in self.toasts.iter_mut().filter(|t| t.id == id) {
                    toast.shown.go_mut(false, now);
                }
                let arrival = self.arrivals.get(&id).cloned();
                self.leaving.entry(id).or_insert_with(|| arrival.unwrap_or_else(|| Animation::new(true).duration(NOTICE_LEAVE).easing(Easing::EaseOutCubic))).go_mut(false, now);
                Task::none()
            }
            Message::ShowError(id) => {
                self.error_shown = Some(id);
                self.error_fade = Animation::new(false).duration(crate::billing::FADE).easing(Easing::EaseOutCubic).go(true, Instant::now());
                Task::none()
            }
            Message::HideError => {
                self.error_fade.go_mut(false, Instant::now());
                Task::none()
            }
            Message::ToastLink(id) => {
                self.error_shown = None;
                let link = self.notices.get(id).map(|n| n.link.clone());
                let _ = self.update(Message::ToastClose(id));
                match link {
                    Some(notices::Link::OpenVideo(path)) => {
                        let at = self.store.videos.iter().position(|v| v.path == path);
                        let shown = self.update(Message::Show(Overlay::Videos));
                        match at {
                            Some(at) => shown.chain(self.update(Message::OpenVideo(at))),
                            None => shown,
                        }
                    }
                    Some(notices::Link::Update) => self.get_update(true),
                    Some(notices::Link::Received(id)) => self.update(Message::Sharing(sharing::Message::Open(id))),
                    Some(notices::Link::Duels) => self.update(Message::Community(crate::community_screen::Message::Duels(crate::duels_screen::Message::Open))),
                    Some(notices::Link::Replay(path)) => {
                        let shown = self.update(Message::Show(Overlay::None));
                        match self.entries().iter().position(|e| e.path == path) {
                            Some(at) => {
                                self.search.clear();
                                shown.chain(self.choose(at))
                            }
                            None => shown,
                        }
                    }
                    Some(notices::Link::Page(page)) => {
                        let _ = open::that_detached(page);
                        Task::none()
                    }
                    Some(notices::Link::RenderAgain(replay)) => {
                        let at = self.entries().iter().position(|e| e.path == replay);
                        match at {
                            Some(at) => {
                                let chosen = self.choose(at);
                                chosen.chain(self.update(Message::Render))
                            }
                            None => Task::none(),
                        }
                    }
                    _ => Task::none(),
                }
            }
            _ => unreachable!("message routed to the wrong main screen handler"),
        }
    }
}
