use super::*;
use iced::widget::column;

impl Main {
    pub(super) fn ask_who(&self) -> Task<Message> {
        if self.settings.token.is_empty() {
            return Task::none();
        }
        let (server, token, name) = (self.settings.server.clone(), self.settings.token.clone(), self.settings.device.clone());
        ui::in_thread(move || Message::Known(bot::me(&server, &token, &name)))
    }

    pub fn signed_in(&self) -> bool {
        !self.settings.token.is_empty()
    }

    pub fn has_telegram(&self) -> bool {
        self.account.as_ref().is_none_or(|me| me.telegram)
    }

    pub(super) fn sign_in_layer(&self) -> Element<'_, Message> {
        if matches!(self.pairing, Pairing::Idle) {
            return Space::new().width(Length::Fill).height(Length::Fill).into();
        }
        let w = &self.words;
        let linking = matches!(self.pairing, Pairing::Linking { .. });
        let (title, how) = if linking { ("link-telegram", "link-telegram-how") } else { ("sign-in", "sign-in-how") };
        let mut left = column![ui::title(w.t(title)), ui::why(w.t(how))].spacing(6).width(Length::Fill);
        match &self.pairing {
            Pairing::Waiting { code, .. } | Pairing::Linking { code, .. } => {
                left = left.push(container(text(code.clone()).font(theme::MONO_BOLD).size(26.0).color(ui::faded(INK))).padding(Padding::ZERO.top(12.0)));
                if !linking {
                    left = left.push(ui::cap(w.t("code-lasts")));
                }
                left = left.push(container(row![ui::dot(8.0), ui::mono_small(w.t("waiting-confirm"), MUTED)].spacing(8).align_y(iced::Center)).padding(Padding::ZERO.top(10.0)));
            }
            Pairing::Unavailable => {
                left = left.push(container(ui::cap(w.t("no-pairing-yet"))).padding(Padding::ZERO.top(12.0)));
            }
            _ => {
                left = left.push(container(ui::mono_small(w.t("asking-bot"), MUTED)).padding(Padding::ZERO.top(12.0)));
            }
        }
        let mut sides = row![left].spacing(24).align_y(iced::Top);
        if let (Some(qr), Pairing::Waiting { .. } | Pairing::Linking { .. }) = (&self.qr, &self.pairing) {
            sides = sides.push(ui::qr(qr));
        }
        let waiting = matches!(self.pairing, Pairing::Waiting { .. } | Pairing::Linking { .. });
        let mut bottom = row![ui::primary(w.t("open-telegram"), waiting.then_some(Message::OpenTelegram))].spacing(4).align_y(iced::Center);
        if matches!(self.pairing, Pairing::Waiting { osu: true, .. }) {
            bottom = bottom.push(ui::quiet(w.t("sign-in-osu"), Some(Message::OpenOsu)));
        }
        let bottom = bottom
            .push(ui::quiet(w.t("copy-link"), waiting.then_some(Message::CopyLink)))
            .push(ui::grow())
            .push(ui::quiet(w.t("later-word"), Some(Message::LaterSignIn)));
        let card = ui::sheet(sides.into(), Some(bottom.into()));
        stack![
            mouse_area(ui::veil(theme::SCRIM)).on_press(Message::LaterSignIn),
            container(container(card).width(theme::COLUMN).padding(Padding::ZERO.top(180.0)))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill),
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    pub(super) fn chats_task(&self) -> Task<Message> {
        if self.settings.token.is_empty() || !self.chats.is_empty() {
            return Task::none();
        }
        let (server, token, name) = (self.settings.server.clone(), self.settings.token.clone(), self.settings.device.clone());
        let chats = ui::in_thread(move || Message::Chats(crate::bot::chats(&server, &token, &name).unwrap_or_default()));
        Task::batch([chats, self.pin_task()])
    }

    pub(super) fn community_chat(&self) -> Option<i64> {
        self.pin.as_ref().and_then(|pin| pin.chat).or(self.settings.chat_id).filter(|id| *id < 0)
    }

    pub(super) fn menu_head(&self) -> Element<'_, Message> {
        use crate::billing;
        let w = &self.words;
        let name = self.account.as_ref().map(|a| a.name.clone()).filter(|n| !n.is_empty()).unwrap_or_else(|| self.settings.linked_as.trim_start_matches('@').to_owned());
        let handle = self.account.as_ref().map(|a| a.username.clone()).filter(|u| !u.is_empty()).map(|u| format!("@{u}"));
        let (title, under) = if self.signed_in() {
            let chat = self.chat_name();
            let mut under = handle.unwrap_or(if chat == "—" { w.t("linked-status") } else { chat });
            if let Some(me) = &self.account {
                under = format!("{under}  ID {}", me.telegram_id);
            }
            (if name.is_empty() { w.t("signed-in") } else { name }, under)
        } else {
            (w.t("not-signed-in"), w.t("stays-here"))
        };
        let mut top = row![
            self.circle(48.0, false),
            {
                let worn = self.billing.badge().filter(|_| self.signed_in());
                let mut named = row![text(title).font(theme::SANS_SEMI).size(17.0).wrapping(text::Wrapping::None).color(ui::faded(INK))].spacing(5).align_y(iced::Center);
                if let Some(badge) = worn.filter(|badge| badge.show_badge) {
                    named = named.push(crate::badge::mark(badge.tier.rank, badge.stage, 20.0));
                }
                let mut lines = column![named].spacing(3).width(Length::Fill);
                if let Some(badge) = worn.filter(|badge| badge.show_title && !badge.title.name.is_empty()) {
                    lines = lines.push(text(badge.title.name.clone()).font(theme::SANS_SEMI).size(12.5).wrapping(text::Wrapping::None).color(ui::faded(crate::badge::title_colour(badge.title.rank, badge.stage))));
                }
                let lines = lines.push(ui::mono_small(under, MUTED));
                match self.online.filter(|_| self.signed_in()) {
                    Some(online) => lines.push(row![ui::dot(7.0), ui::mono_small(w.n("online-now", u64::from(online)), MUTED)].spacing(6).align_y(iced::Center)),
                    None => lines,
                }
            },
        ]
        .spacing(14)
        .align_y(iced::Center);
        if !self.signed_in() {
            top = top.push(ui::primary(w.t("sign-in"), Some(Message::SignIn)));
            return container(top).padding(22).into();
        }
        top = top.push(ui::link(w.t("sign-out"), Message::SignOut));
        let open = Message::Billing(billing::Message::Open);
        let mut lines = column![top].spacing(16);
        match self.billing.subscription() {
            Some(held) if held.access => {
                let (said, ink, fill) = billing::badge(held, w);
                let k = ui::fade();
                let badge = container(text(said).font(theme::SANS_SEMI).size(12.5).color(ui::faded(ink)))
                    .padding([3, 10])
                    .style(move |_| container::Style { background: Some(iced::Background::Color(Color { a: fill.a * k, ..fill })), border: iced::Border { radius: 12.0.into(), ..iced::Border::default() }, ..container::Style::default() });
                let mut head = row![crate::badge::mark(Some(self.billing.rank_of(&held.plan)), self.billing.stage(), 22.0), text(billing::plan_name(&held.plan)).font(theme::SANS_SEMI).size(20.0).color(ui::faded(INK)), badge, ui::grow()].spacing(10).align_y(iced::Center);
                let left = billing::left(held, self.now_unix);
                if let Some((days, _)) = left {
                    head = head.push(text(w.n("billing-days", days as u64)).font(theme::MONO_BOLD).size(13.0).color(ui::faded(INK)));
                }
                lines = lines.push(head);
                if let Some((_, fraction)) = left {
                    lines = lines.push(billing::bar(fraction, if held.state == "active" { theme::NOTICE_SUCCESS } else { theme::NOTICE_INFO }));
                }
                let told = w.with(if held.state == "active" { "billing-paid-until" } else { "billing-ends" }, &[("date", billing::until(held, w, self.now_unix))]);
                lines = lines.push(row![container(text(told).font(theme::SANS).size(14.0).color(ui::faded(INK))).width(Length::Fill), billing::outlined(w.t("billing-manage"), Some(open))].spacing(10).align_y(iced::Center));
            }
            _ => {
                let (said, tone) = billing::line(&self.billing, w, self.now_unix);
                let action = if self.billing.waiting() { billing::outlined(w.t("billing-manage"), Some(open)) } else { ui::primary(w.t("billing-subscribe"), Some(open)) };
                lines = lines.push(row![container(text(said).font(theme::SANS_SEMI).size(16.0).color(ui::faded(if tone == billing::Tone::Bad { ACCENT } else { INK }))).width(Length::Fill), action].spacing(10).align_y(iced::Center));
            }
        }
        container(lines).padding(22).into()
    }

    pub(super) fn handle_account(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SignIn => {
                self.menu = None;
                self.menu_open = Animation::new(false).duration(MENU_OPEN).easing(Easing::EaseOutCubic);
                if self.signed_in() || matches!(self.pairing, Pairing::Waiting { .. }) {
                    return Task::none();
                }
                self.pairing = Pairing::Asking;
                let server = self.settings.server.clone();
                let name = self.settings.device.clone();
                ui::in_thread(move || Message::PairAsked(bot::pair(&server, &name).map(|p| (p.code, p.link, p.osu))))
            }
            Message::PairAsked(Ok((code, link, osu))) => {
                let link = if link.is_empty() {
                    format!("https://t.me/OneNineEightFourGlobalBot?start=pair-{}", bot::tidy(&code))
                } else {
                    link
                };
                self.qr = crate::first_run::qr_for(&link);
                self.pairing = Pairing::Waiting { code: bot::pretty(&code), link, osu };
                Task::none()
            }
            Message::OpenOsu => {
                if let Pairing::Waiting { code, osu: true, .. } = &self.pairing {
                    let _ = open::that_detached(bot::osu_link(&self.settings.server, code));
                }
                Task::none()
            }
            Message::LinkTelegram => {
                self.menu = None;
                if !self.signed_in() {
                    return self.update(Message::SignIn);
                }
                if self.has_telegram() || !matches!(self.pairing, Pairing::Idle) {
                    return Task::none();
                }
                self.pairing = Pairing::Asking;
                let (server, token, name) = (self.settings.server.clone(), self.settings.token.clone(), self.settings.device.clone());
                ui::in_thread(move || Message::TelegramAsked(bot::link_telegram(&server, &token, &name).map(|p| (p.code, p.link)).map_err(|e| e.to_string())))
            }
            Message::TelegramAsked(Ok((code, link))) => {
                self.qr = crate::first_run::qr_for(&link);
                self.pairing = Pairing::Linking { code: bot::pretty(&code), link };
                Task::none()
            }
            Message::TelegramAsked(Err(_)) => {
                self.pairing = Pairing::Unavailable;
                Task::none()
            }
            Message::PairAsked(Err(_)) => {
                self.pairing = Pairing::Unavailable;
                Task::none()
            }
            Message::PresenceTick => {
                if self.presence_pending || self.settings.token.is_empty() || self.gallery {
                    return Task::none();
                }
                self.presence_pending = true;
                let (server, token, name) = (self.settings.server.clone(), self.settings.token.clone(), self.settings.device.clone());
                ui::in_thread(move || Message::PresenceAnswered(bot::presence(&server, &token, &name)))
            }
            Message::PresenceAnswered(result) => {
                self.presence_pending = false;
                if let Ok(Some(online)) = result {
                    self.online = Some(online);
                }
                Task::none()
            }
            Message::Poll => match &self.pairing {
                Pairing::Waiting { code, .. } => {
                    let server = self.settings.server.clone();
                    let code = bot::tidy(code);
                    ui::in_thread(move || Message::Polled(bot::paired(&server, &code)))
                }
                Pairing::Linking { .. } => self.ask_who(),
                _ => Task::none(),
            },
            Message::Polled(Ok(Paired::Linked { token, who })) => {
                self.reset_billing();
                self.settings.token = token;
                self.sync_dossiers();
                self.settings.linked_as = who;
                let _ = self.settings.save();
                self.pairing = Pairing::Idle;
                self.ask_who()
            }
            Message::Polled(Ok(Paired::Gone)) | Message::Polled(Err(Refused::NotThere)) => {
                self.pairing = Pairing::Idle;
                self.update(Message::SignIn)
            }
            Message::Polled(_) => Task::none(),
            Message::OpenTelegram => {
                if let Pairing::Waiting { link, .. } | Pairing::Linking { link, .. } = &self.pairing {
                    let _ = open::that_detached(link);
                }
                Task::none()
            }
            Message::CopyLink => match &self.pairing {
                Pairing::Waiting { link, .. } | Pairing::Linking { link, .. } => iced::clipboard::write(link.clone()),
                _ => Task::none(),
            },
            Message::LaterSignIn => {
                self.pairing = Pairing::Idle;
                Task::none()
            }
            Message::SignOut => {
                self.reset_billing();
                self.settings.token.clear();
                self.dossier_cache.clear();
                self.people_dossiers.clear();
                self.settings.linked_as.clear();
                let _ = self.settings.save();
                self.account = None;
                self.avatar = None;
                self.sharing = sharing::State::default();
                self.menu = None;
                self.menu_open = Animation::new(false).duration(MENU_OPEN).easing(Easing::EaseOutCubic);
                Task::none()
            }
            Message::Known(Ok(me)) => {
                let wants_avatar = me.avatar;
                let said = if me.username.is_empty() { me.name.clone() } else { format!("@{}", me.username) };
                if !said.is_empty() && self.settings.linked_as != said {
                    self.settings.linked_as = said;
                    let _ = self.settings.save();
                }
                if matches!(self.pairing, Pairing::Linking { .. }) && me.telegram {
                    self.pairing = Pairing::Idle;
                    self.chats.clear();
                    let words = self.words.t("telegram-linked");
                    self.say(words);
                }
                let chats = if me.telegram { self.chats_task() } else { Task::none() };
                self.account = Some(me);
                self.offer_shared_source();
                let inbox = Task::batch([self.inbox_task(true), self.replays_state_task(), self.replays_sync_task(true), chats]);
                if !wants_avatar {
                    return inbox;
                }
                let (server, token, name) = (self.settings.server.clone(), self.settings.token.clone(), self.settings.device.clone());
                Task::batch([
                    inbox,
                    ui::in_thread(move || {
                        let bytes = bot::avatar(&server, &token, &name).ok();
                        Message::Avatar(bytes.and_then(|b| decoded_bytes(&b, AVATAR_SIDE)))
                    }),
                ])
            }
            Message::Known(Err(_)) => Task::none(),
            Message::Avatar(handle) => {
                self.avatar = handle;
                Task::none()
            }
            Message::Chats(chats) => {
                let wanted: Vec<i64> = chats.iter().filter(|chat| chat.photo && !self.chat_faces.contains_key(&chat.id)).map(|chat| chat.id).collect();
                self.chats = chats;
                let named = self.settings.chat_id.and_then(|id| self.chats.iter().find(|chat| chat.id == id)).map(|chat| chat.title.clone());
                if let Some(title) = named.filter(|title| *title != self.settings.chat_title) {
                    self.settings.chat_title = title;
                    let _ = self.settings.save();
                }
                if wanted.is_empty() {
                    return Task::none();
                }
                let (server, token, name) = (self.settings.server.clone(), self.settings.token.clone(), self.settings.device.clone());
                ui::streamed(move |push| {
                    for id in wanted {
                        let handle = crate::bot::chat_avatar(&server, &token, &name, id)
                            .ok()
                            .and_then(|bytes| decoded_bytes(&bytes, AVATAR_SIDE));
                        if !push(Message::ChatFace(id, handle)) {
                            return;
                        }
                    }
                })
            }
            Message::ChatFace(id, handle) => {
                if let Some(handle) = handle {
                    self.chat_faces.insert(id, handle);
                }
                Task::none()
            }
            _ => unreachable!("message routed to the wrong main screen handler"),
        }
    }
}
