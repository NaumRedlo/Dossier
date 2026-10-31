use super::*;
use iced::widget::column;

impl Main {
    pub(super) fn mini_player_layer(&self) -> Element<'_, Message> {
        if !self.mini_player || self.leaving_player {
            return Space::new().width(Length::Fill).height(Length::Fill).into();
        }
        let Some(player) = &self.player else {
            return Space::new().width(Length::Fill).height(Length::Fill).into();
        };
        let still = if let Some(clip) = &self.clip {
            clip.thumb.as_deref().and_then(|url| self.news_pictures.get(&crate::community_screen::wide(url)).or_else(|| self.news_pictures.get(url)))
        } else if let Some(video) = self.open_video.and_then(|at| self.store.videos.get(at)) {
            self.thumbs.get(&video.map_hash)
        } else if let Some(got) = self.sharing.opened() {
            self.thumbs.get(&got.map_hash).or_else(|| self.sharing.thumbs.get(&got.id))
        } else { None };
        let player = player.borrow();
        let wide = (self.width - 48.0).min(356.0).max(160.0);
        let picture_h = (wide * 9.0 / 16.0).floor();
        let picture: Element<'_, Message> = match &player.frame {
            Some(frame) => crate::film::show(frame, crate::film::Fit::Contain, ui::fade()),
            None => match still {
                Some(handle) => image(handle.clone()).content_fit(ContentFit::Cover).width(Length::Fill).height(Length::Fill).opacity(0.55 * ui::fade()).into(),
                None => container(ui::fine_hatch()).width(Length::Fill).height(Length::Fill).into(),
            },
        };
        let actions: Element<'_, Message> = if self.over_controls {
            container(row![
                ui::control_button(if player.paused { ui::Control::Play } else { ui::Control::Pause }, 16.0, Some(Message::PlayerToggle), false),
                ui::control_button(ui::Control::Grow, 16.0, Some(Message::PlayerRestore), false),
                ui::control_button(ui::Control::Close, 16.0, Some(Message::ClosePlayer), false),
            ].spacing(4).align_y(iced::Center))
                .padding(8).width(Length::Fill).height(Length::Fill)
                .align_x(iced::alignment::Horizontal::Right).align_y(iced::alignment::Vertical::Bottom)
                .style(theme::under_picture(ui::fade())).into()
        } else { Space::new().into() };
        let card = iced::widget::opaque(mouse_area(
            container(stack![picture, actions]).width(wide).height(picture_h)
                .id(iced::widget::Id::new("mini-player")).style(theme::screen_bare).clip(true))
            .on_enter(Message::ControlsHover(true)).on_exit(Message::ControlsHover(false))
            .on_press(Message::PlayerToggle).on_double_click(Message::PlayerRestore)
            .interaction(iced::mouse::Interaction::Pointer));
        pin(card).x((self.width - wide - 24.0).max(12.0)).y((self.height - picture_h - 24.0).max(12.0)).into()
    }

    pub(super) fn videos_lone<'a>(&'a self, content: Element<'a, Message>) -> Element<'a, Message> {
        match self.signed_in() {
            true => stack![content, container(self.videos_switch()).padding(Padding { top: 34.0, right: 0.0, bottom: 0.0, left: 40.0 })].width(Length::Fill).height(Length::Fill).into(),
            false => content,
        }
    }

    pub(super) fn videos_listed<'a>(&'a self, head: Element<'a, Message>, lines: Vec<Element<'a, Message>>) -> Element<'a, Message> {
        let mut rows = column![].spacing(0).width(Length::Fill);
        if self.signed_in() {
            rows = rows.push(container(self.videos_switch()).padding(Padding { top: 0.0, right: 0.0, bottom: 20.0, left: 12.0 }));
        }
        rows = rows.push(head);
        for line in lines {
            rows = rows.push(line);
        }
        crate::glide::brim(scrollable(container(rows).padding(Padding { top: 34.0, right: 28.0, bottom: 24.0, left: 28.0 })).direction(ui::hidden_bar()).width(Length::Fill).height(Length::Fill)).into()
    }

    pub(super) fn videos_view(&self) -> Element<'_, Message> {
        let w = &self.words;
        let received = self.signed_in() && self.sharing.tab == sharing::Tab::Received;
        let page: Element<'_, Message> = match (received, self.sharing.videos.is_empty(), self.store.videos.is_empty()) {
            (true, true, _) => self.videos_lone(self.received_empty()),
            (true, false, _) => self.videos_listed(self.received_head(), self.sharing.videos.iter().map(|got| self.received_row(got)).collect()),
            (false, _, true) => {
                let empty = column![
                    text(w.t("no-videos")).font(theme::SANS_SEMI).size(theme::TITLE).color(ui::faded(INK)),
                    ui::cap(w.t("render-one")),
                    container(ui::primary(w.t("back-to-replays"), Some(Message::Show(Overlay::None)))).padding(Padding::ZERO.top(12.0)),
                ]
                .spacing(6)
                .align_x(iced::Center);
                self.videos_lone(container(empty).width(Length::Fill).height(Length::Fill).center(Length::Fill).into())
            }
            (false, _, false) => self.videos_listed(self.video_head(), self.store.videos.iter().enumerate().map(|(at, video)| self.video_row(at, video)).collect()),
        };
        let sheet = column![Space::new().height(theme::CONTROL_HEIGHT + 4.0 + 22.0), page].width(Length::Fill).height(Length::Fill);
        let opened = self.stage_open.interpolate(0.0, 1.0, self.now);
        let shown = !self.mini_player && opened > 0.001;
        let stage: Element<'_, Message> = match (&self.player, self.open_video.and_then(|at| self.store.videos.get(at)), self.sharing.opened()) {
            (Some(player), Some(video), _) if shown => ui::fading(opened, || self.stage(&player.borrow(), self.video_bill(video))),
            (Some(player), None, Some(got)) if shown => ui::fading(opened, || self.stage(&player.borrow(), self.received_bill(got))),
            (None, None, Some(got)) if shown => ui::fading(opened, || self.received_stage(got)),
            _ => Space::new().width(Length::Fill).height(Length::Fill).into(),
        };
        let sheet: Element<'_, Message> = match (self.player.is_some() || self.sharing.open.is_some()) && !self.mini_player && opened >= 0.999 {
            true => Space::new().width(Length::Fill).height(Length::Fill).into(),
            false => sheet.into(),
        };
        let watching = 1.0 - self.cinema.interpolate(0.0, 1.0, self.now);
        let crest: Element<'_, Message> = match watching > 0.001 {
            true => pin(ui::fading(watching, ui::brand)).x(CREST_HOME.0).y(CREST_HOME.1).into(),
            false => Space::new().width(Length::Fill).height(Length::Fill).into(),
        };
        stack![sheet, crest, stage].width(Length::Fill).height(Length::Fill).into()
    }

    pub(super) fn video_head(&self) -> Element<'_, Message> {
        let w = &self.words;
        let cell = |key: &str, width: f32| container(ui::label(w.t(key), FAINT)).width(width).align_x(iced::alignment::Horizontal::Center);
        let grow = |key: &str| container(ui::label(w.t(key), FAINT)).width(Length::Fill);
        let right = |key: &str, width: f32| container(ui::label(w.t(key), FAINT)).width(width).align_x(iced::alignment::Horizontal::Right);
        container(
            row![
                Space::new().width(VIDEO_THUMB.0 as f32),
                cell("when", VIDEO_DATE_W),
                grow("who-and-map"),
                container(ui::label(w.t("mods"), FAINT)).width(110.0),
                right("length", 56.0),
                right("size", 84.0),
            ]
            .spacing(14)
            .align_y(iced::Center),
        )
        .padding([0, 12])
        .height(24.0)
        .into()
    }

    pub(super) fn video_row(&self, at: usize, video: &videos::Video) -> Element<'_, Message> {
        let w = &self.words;
        let chosen = self.open_video == Some(at);
        let waiting = || -> Element<'_, Message> { container(ui::fine_hatch()).width(VIDEO_THUMB.0 as f32).height(VIDEO_THUMB.1 as f32).into() };
        let picture: Element<'_, Message> = match self.thumbs.get(&video.map_hash) {
            Some(handle) => {
                let shown = self.thumb_pictures.shown_of(&video.map_hash, self.now);
                let still: Element<'_, Message> = image(handle.clone())
                    .content_fit(ContentFit::Cover)
                    .width(VIDEO_THUMB.0 as f32)
                    .height(VIDEO_THUMB.1 as f32)
                    .border_radius(6.0)
                    .opacity(ui::fade() * shown * if chosen { 1.0 } else { 0.85 })
                    .into();
                if shown >= 0.999 { still } else { stack![waiting(), still].into() }
            }
            None => waiting(),
        };
        let stamp = w.compact_date(video.made_at, self.now_unix);
        let (day, time) = stamp.rsplit_once(' ').unwrap_or((&stamp, ""));
        let when = column![ui::mono_small(day.to_owned(), MUTED), ui::mono_small(time.to_owned(), FAINT)]
            .spacing(2)
            .align_x(iced::alignment::Horizontal::Center);
        let who = column![
            text(video.player.clone()).font(theme::SANS_SEMI).size(theme::LEAD).wrapping(text::Wrapping::None).color(ui::faded(INK)),
            text(ui::shortened(video.map_line(), 70)).font(theme::SANS).size(theme::BODY).wrapping(text::Wrapping::None).color(ui::faded(MUTED)),
        ]
        .spacing(2);
        let mut mods = row![].spacing(4).align_y(iced::Center);
        for acronym in video.mods.iter().filter(|acronym| crate::modicons::shown(acronym)) {
            mods = mods.push(mod_badge(acronym));
        }
        let line = row![
            container(picture).width(VIDEO_THUMB.0 as f32).height(VIDEO_THUMB.1 as f32),
            container(when).width(VIDEO_DATE_W).align_x(iced::alignment::Horizontal::Center),
            container(who).width(Length::Fill).clip(true),
            container(mods).width(110.0),
            container(ui::mono(w.length(video.length_ms), MUTED)).width(56.0).align_x(iced::alignment::Horizontal::Right),
            container(ui::mono(w.mb(video.size), MUTED)).width(84.0).align_x(iced::alignment::Horizontal::Right),
        ]
        .spacing(14)
        .align_y(iced::Center);
        button(container(line).height(VIDEO_ROW).width(Length::Fill).center_y(VIDEO_ROW))
            .padding([0, 12])
            .style(ui::button_faded(theme::row(chosen)))
            .on_press(Message::OpenVideo(at))
            .into()
    }

    pub(super) fn say(&mut self, words: String) {
        self.hint = Some((words, Instant::now()));
    }

    pub(super) fn video_bill<'a>(&'a self, video: &'a videos::Video) -> Bill<'a> {
        let w = &self.words;
        let sending_this = self.sending.as_ref().filter(|s| s.path == video.path);
        let telegram: Element<'_, Message> = match sending_this {
            Some(sending) if sending.over.is_none() => ui::progress(w.t("sending"), self.progress_shown, None),
            _ => ui::primary(w.t("to-telegram"), Some(Message::SendVideo)),
        };
        let buttons = row![
            ui::grow(),
            ui::springy(ui::quiet(w.t("in-folder"), Some(Message::RevealVideo)), 0.04),
            ui::springy(ui::quiet(w.t("delete"), Some(Message::AskDelete)), 0.04),
            ui::springy(ui::quiet(w.t("to-player"), sending_this.is_none_or(|s| s.over.is_some()).then_some(Message::Sharing(sharing::Message::Pick))), 0.04),
            ui::springy(telegram, 0.03),
        ]
        .spacing(6)
        .align_y(iced::Center);
        let at = self.open_video.unwrap_or(0);
        Bill {
            still: self.thumbs.get(&video.map_hash),
            name: video.player.clone(),
            mods: &video.mods,
            line: video.map_line(),
            buttons: buttons.into(),
            earlier: (at > 0).then_some(Message::PlayerNeighbour(-1)),
            later: (at + 1 < self.store.videos.len()).then_some(Message::PlayerNeighbour(1)),
            aside: None,
        }
    }

    pub(super) fn clip_bill<'a>(&'a self, clip: &'a Clip) -> Bill<'a> {
        let w = &self.words;
        let buttons = row![
            ui::grow(),
            ui::springy(ui::quiet(w.t("in-folder"), Some(Message::RevealVideo)), 0.04),
            ui::springy(ui::primary(w.t("act-telegram"), Some(Message::Community(crate::community_screen::Message::Open(clip.link.clone())))), 0.03),
        ]
        .spacing(6)
        .align_y(iced::Center);
        Bill {
            still: clip.thumb.as_deref().and_then(|url| self.news_pictures.get(&crate::community_screen::wide(url)).or_else(|| self.news_pictures.get(url))),
            name: clip.from.clone(),
            mods: &[],
            line: clip.said.clone(),
            buttons: buttons.into(),
            earlier: None,
            later: None,
            aside: None,
        }
    }

    pub(super) fn stage<'a>(&'a self, player: &player::Player, bill: Bill<'a>) -> Element<'a, Message> {
        let w = &self.words;
        let playing = !player.paused;
        let wide = self.widened.interpolate(0.0, 1.0, self.now);
        let round: iced::border::Radius = (PICTURE_RADIUS).into();
        let Bill { still, name, mods, line, buttons, earlier, later, aside } = bill;
        let picture: Element<'_, Message> = match &player.frame {
            Some(frame) => crate::film::show(frame, crate::film::Fit::Contain, ui::fade()),
            None => match still {
                Some(handle) => image(handle.clone())
                    .content_fit(ContentFit::Cover)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .opacity(0.35 * ui::fade())
                    .border_radius(round)
                    .into(),
                None => Space::new().width(Length::Fill).height(Length::Fill).into(),
            },
        };
        let mark: Element<'_, Message> = if self.video_request.is_some() || (!player.paused && player.frame.is_none()) {
            container(ui::turning(28.0)).center(Length::Fill).into()
        } else if player.paused && !self.asking_delete {
            let kind = if player.ended() { ui::Control::Again } else { ui::Control::Play };
            container(ui::halo(kind, 72.0, 1.0))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Center)
                .into()
        } else {
            Space::new().width(Length::Fill).height(Length::Fill).into()
        };
        let hint: Element<'_, Message> = match &self.hint {
            Some((words, since)) => {
                let gone = self.now.saturating_duration_since(*since).as_secs_f32() / HINT_SHOWN.as_secs_f32();
                let k = (1.0 - gone).clamp(0.0, 1.0);
                container(ui::fading(k, || {
                    container(text(words.clone()).font(theme::SANS_SEMI).size(theme::BODY).color(ui::faded(INK)))
                        .padding(Padding { top: 6.0, right: 12.0, bottom: 6.0, left: 12.0 })
                        .style(ui::box_at(theme::bubble, ui::fade() * k))
                }))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(iced::alignment::Horizontal::Center)
                .padding(Padding::ZERO.top(18.0))
                .into()
            }
            None => Space::new().width(Length::Fill).height(Length::Fill).into(),
        };
        let gap = STAGE_GAP - (STAGE_GAP - 16.0) * wide;
        let under_h = STAGE_UNDER * (1.0 - wide);
        let room_w = (self.width - 2.0 * gap - 2.0 * PICTURE_INSET).max(320.0);
        let room_h = (self.height - 2.0 * gap - STAGE_TOP - under_h - 2.0 * PICTURE_INSET).max(180.0);
        let screen_h = (room_w * 9.0 / 16.0).min(room_h).floor();
        let screen_w = (screen_h * 16.0 / 9.0).min(room_w).floor();
        let out = self.controls.interpolate(0.0, 1.0, self.now);
        let controls: Element<'_, Message> = match out > 0.004 && self.video_request.is_none() {
            true => ui::fading(out, || {
                let shown = self.scrubbing.unwrap_or_else(|| player.fraction());
                let length_ms = player.length_ms;
                let seek = iced::widget::canvas(ui::Seek {
                    played: shown,
                    alpha: ui::fade(),
                    at: Box::new(move |part| {
                        let ms = (part as f64 * length_ms as f64) as i64;
                        format!("{}:{:02}", ms / 60_000, (ms / 1000) % 60)
                    }),
                    on_move: Box::new(Message::Scrubbing),
                    on_drop: Box::new(Message::SeekTo),
                })
                .width(Length::Fill)
                .height(36.0);
                let at_ms = self.scrubbing.map_or_else(|| player.at_ms(), |part| (part as f64 * player.length_ms as f64) as i64);
                let clock = row![
                    ui::mono_small(w.length(at_ms), INK),
                    ui::mono_small("/".to_owned(), FAINT),
                    ui::mono_small(w.length(player.length_ms), MUTED),
                ]
                .spacing(6)
                .align_y(iced::Center);
                let sound = if self.settings.player_muted || self.settings.player_level <= 0.001 {
                    ui::Control::Hushed
                } else if self.settings.player_level < 0.5 {
                    ui::Control::Soft
                } else {
                    ui::Control::Loud
                };
                let level = iced::widget::canvas(ui::Level {
                    at: if self.settings.player_muted { 0.0 } else { self.settings.player_level },
                    alpha: ui::fade(),
                    on: Box::new(Message::PlayerLevel),
                })
                .width(64.0)
                .height(22.0);
                let keys = row![
                    ui::control_button(ui::Control::Earlier, 16.0, earlier, false),
                    ui::control_button(ui::Control::Back, 16.0, Some(Message::SeekBy(-5000)), false),
                    ui::control_button(if playing { ui::Control::Pause } else { ui::Control::Play }, 20.0, Some(Message::PlayerToggle), false),
                    ui::control_button(ui::Control::Ahead, 16.0, Some(Message::SeekBy(5000)), false),
                    ui::control_button(ui::Control::Later, 16.0, later, false),
                    container(clock).padding(Padding::ZERO.left(8.0)),
                    ui::grow(),
                    container(
                        iced::widget::canvas(ui::Speed {
                            at: self.settings.player_rate,
                            stops: player::RATES.to_vec(),
                            words: format!("{}×", w.rate(self.settings.player_rate)),
                            alpha: ui::fade(),
                            on: Box::new(Message::PlayerSpeed),
                        })
                        .width(118.0)
                        .height(26.0)
                    )
                    .padding(Padding::ZERO.right(6.0)),
                    ui::control_button(sound, 16.0, Some(Message::PlayerMute), self.settings.player_muted),
                    container(level).padding(Padding::ZERO.right(4.0)),
                    ui::control_button(ui::Control::Over, 16.0, Some(Message::PlayerLoop), self.settings.player_loop),
                    ui::control_button(
                        if self.widened.value() { ui::Control::Shrink } else { ui::Control::Grow },
                        16.0,
                        Some(Message::PlayerWiden),
                        false,
                    ),
                ]
                .spacing(2)
                .align_y(iced::Center);
                let under_picture = container(column![container(seek).padding(Padding::ZERO.right(4.0).left(4.0)), container(keys).height(40.0)].width(Length::Fill))
                    .width(Length::Fill)
                    .padding(Padding { top: 8.0, right: 10.0, bottom: 2.0, left: 10.0 })
                    .style(theme::under_picture(ui::fade()));
                let held = mouse_area(under_picture)
                    .on_enter(Message::ControlsHover(true))
                    .on_exit(Message::ControlsHover(false))
                    .on_press(Message::ControlsHover(true));
                container(ui::grown(held, iced::Point::new(0.5, 1.0), 0.0, 1.0))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_y(iced::alignment::Vertical::Bottom)
                    .into()
            }),
            false => Space::new().width(Length::Fill).height(Length::Fill).into(),
        };
        let hidden_pointer = playing && out < 0.01 && !self.asking_delete;
        let on_air = player.frame.is_some();
        let screen = mouse_area(
            container(stack![picture, mark, hint, controls].width(screen_w).height(screen_h))
                .width(screen_w)
                .height(screen_h)
                .style(ui::box_faded(if on_air { theme::screen_bare } else { theme::screen }))
                .clip(true),
        )
        .interaction(if hidden_pointer { iced::mouse::Interaction::Hidden } else { iced::mouse::Interaction::Idle })
        .on_press(Message::PlayerToggle)
        .on_double_click(Message::PlayerWiden)
        .on_move(Message::PlayerStir)
        .on_scroll(|delta| {
            let up = match delta {
                iced::mouse::ScrollDelta::Lines { y, .. } => y,
                iced::mouse::ScrollDelta::Pixels { y, .. } => y / 40.0,
            };
            Message::PlayerLouder(up)
        });
        let mut named = row![text(name).font(theme::SANS_SEMI).size(theme::LEAD).wrapping(text::Wrapping::None).color(ui::faded(INK))]
            .spacing(6)
            .align_y(iced::Center);
        for acronym in mods.iter().filter(|acronym| crate::modicons::shown(acronym)) {
            named = named.push(mod_badge(acronym));
        }
        let mut title = row![
            column![
                named,
                text(ui::shortened(line, 62)).font(theme::SANS).size(theme::CAPTION).wrapping(text::Wrapping::None).color(ui::faded(MUTED)),
            ]
            .spacing(2),
            ui::grow(),
        ]
        .spacing(12)
        .align_y(iced::Center);
        if let Some(aside) = aside {
            title = title.push(aside);
        }
        let title = title
            .push(ui::control_button(ui::Control::Mini, 18.0, Some(Message::PlayerMinimize), false))
            .push(ui::close_square(Some(Message::ClosePlayer), ui::CLOSE_SIDE));
        let mut inside = column![
            container(title)
                .height(STAGE_TOP)
                .align_y(iced::alignment::Vertical::Center)
                .padding(Padding { top: 2.0, right: PICTURE_INSET - 7.0, bottom: 0.0, left: PICTURE_INSET }),
            container(screen).width(Length::Fill).height(screen_h).center_x(Length::Fill),
        ]
        .width(Length::Fill);
        if under_h > 1.0 {
            inside = inside.push(
                container(buttons)
                    .padding(Padding { top: 0.0, right: PICTURE_INSET, bottom: 0.0, left: PICTURE_INSET })
                    .height(under_h)
                    .align_y(iced::alignment::Vertical::Center)
                    .clip(true),
            );
        }
        let card = container(inside).width(screen_w + 2.0 * PICTURE_INSET).height(screen_h + STAGE_TOP + under_h);
        let backdrop = mouse_area(ui::veil(theme::CINEMA_SCRIM)).on_press(Message::ClosePlayer);
        let opened = self.stage_open.interpolate(0.0, 1.0, self.now);
        let card = ui::grown(card, iced::Point::new(0.5, 0.5), 0.0, 0.965 + 0.035 * opened);
        stack![backdrop, container(card).width(Length::Fill).height(Length::Fill).center(Length::Fill)]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    pub(super) fn delete_card(&self) -> Element<'_, Message> {
        let w = &self.words;
        let Some(video) = self.open_video.and_then(|at| self.store.videos.get(at)) else {
            return Space::new().width(Length::Fill).height(Length::Fill).into();
        };
        let k = self.ask_fade.interpolate(0.0, 1.0, self.now);
        ui::fading(k, || {
            let picture: Element<'_, Message> = match self.thumbs.get(&video.map_hash) {
                Some(handle) => image(handle.clone())
                    .content_fit(ContentFit::Cover)
                    .width(ASK_THUMB.0)
                    .height(ASK_THUMB.1)
                    .border_radius(10.0)
                    .opacity(ui::fade())
                    .into(),
                None => container(ui::fine_hatch()).width(ASK_THUMB.0).height(ASK_THUMB.1).into(),
            };
            let who = match video.map_line().is_empty() {
                true => format!("{}  {}", video.player, w.mb(video.size)),
                false => format!("{} — {}  {}", video.player, ui::shortened(video.map_line(), 44), w.mb(video.size)),
            };
            let middle = column![
                picture,
                container(text(w.t("delete-video")).font(theme::SANS_SEMI).size(20.0).color(ui::faded(INK))).padding(Padding::ZERO.top(16.0)),
                text(who).font(theme::SANS).size(theme::CAPTION).align_x(iced::Center).color(ui::faded(MUTED)),
                text(w.t("to-the-bin")).font(theme::SANS).size(11.0).align_x(iced::Center).color(ui::faded(FAINT)),
                container(
                    row![ui::quiet(w.t("keep"), Some(Message::KeepVideo)), ui::primary(w.t("delete"), Some(Message::DeleteVideo))]
                        .spacing(8)
                        .align_y(iced::Center)
                )
                .padding(Padding::ZERO.top(18.0)),
            ]
            .spacing(6)
            .align_x(iced::Center)
            .width(ASK_WIDE);
            let risen = ui::grown(middle, iced::Point::new(0.5, 0.5), 0.0, 0.97 + 0.03 * k);
            stack![
                mouse_area(ui::veil(theme::DEEP_SCRIM)).on_press(Message::KeepVideo),
                container(risen).width(Length::Fill).height(Length::Fill).center(Length::Fill),
            ]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
        })
    }
}

const ASK_THUMB: (f32, f32) = (176.0, 99.0);
const ASK_WIDE: f32 = 400.0;
