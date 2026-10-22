use super::*;

impl Main {
    pub(super) fn billing_layer(&self) -> Element<'_, Message> {
        if !self.billing.shown {
            return Space::new().width(Length::Fill).height(Length::Fill).into();
        }
        let k = self.billing.fade.interpolate(0.0, 1.0, self.now);
        let eased = 1.0 - (1.0 - k).powi(3);
        ui::fading(ui::fade() * k, || {
            let card = crate::billing::sheet(&self.billing, &self.words, self.signed_in(), self.now_unix).map(Message::Billing);
            stack![
                iced::widget::opaque(mouse_area(ui::veil(theme::SCRIM)).on_press(Message::Billing(crate::billing::Message::Close))),
                ui::grown(container(ui::scaled(container(iced::widget::opaque(if crate::billing::warm(&self.billing, self.signed_in()) { ui::glass_warm(card, self.glass_picture(), iced::Size::new(self.width, self.height)) } else { ui::glass_deep(card, self.glass_picture(), iced::Size::new(self.width, self.height)) })).width(if crate::billing::wide(&self.billing, self.signed_in()) { (self.width - 80.0).clamp(theme::COLUMN, BILLING_SHEET) } else { theme::COLUMN }), (self.width / BILLING_WIDE).clamp(1.0, BILLING_GROWTH)))
                    .padding(Padding::ZERO.top(120.0))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x(Length::Fill), Point::new(0.5, 0.2), (1.0 - eased) * 26.0, 0.965 + 0.035 * eased),
            ]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
        })
    }

    pub(super) fn invalidate_billing_requests(&mut self) {
        self.billing_epoch = self.billing_epoch.wrapping_add(1);
        self.billing_status_pending = false;
        self.billing_plans_pending = false;
    }

    pub(super) fn reset_billing(&mut self) {
        self.invalidate_billing_requests();
        self.billing = crate::billing::State::new(String::new(), &self.billing.currency);
        self.settings.billing_email.clear();
    }

    pub(super) fn billing_asks(&mut self, plans: bool) -> Task<Message> {
        if self.settings.token.is_empty() || (!plans && (self.billing.paying || self.billing.cancelling)) {
            return Task::none();
        }
        let pending = if plans { &mut self.billing_plans_pending } else { &mut self.billing_status_pending };
        if *pending { return Task::none(); }
        *pending = true;
        let epoch = self.billing_epoch;
        let (server, token, device) = (self.settings.server.clone(), self.settings.token.clone(), self.settings.device.clone());
        if plans {
            ui::in_thread(move || Message::BillingReply(epoch, crate::billing::Message::Plans(crate::bot::plans(&server, &token, &device).map_err(|e| e.to_string()))))
        } else {
            ui::in_thread(move || Message::BillingReply(epoch, crate::billing::Message::Status(crate::bot::subscription(&server, &token, &device).map_err(|e| e.to_string()))))
        }
    }

    pub(super) fn billing_step(&mut self, message: crate::billing::Message) -> Task<Message> {
        use crate::billing::{Effect as E, Message as B};
        let waited = self.billing.waiting();
        let opened = matches!(message, B::Open);
        let sign_in = matches!(message, B::SignIn);
        let effects = self.billing.update(message, Instant::now());
        if effects.iter().any(|effect| matches!(effect, E::Checkout(..) | E::Cancel)) {
            self.invalidate_billing_requests();
        }
        let online = !self.settings.token.is_empty();
        let (server, token, device) = (self.settings.server.clone(), self.settings.token.clone(), self.settings.device.clone());
        let mut tasks = Vec::new();
        if opened && self.menu.is_some() {
            tasks.push(self.update(Message::MenuClose));
        }
        if sign_in {
            tasks.push(self.update(Message::SignIn));
        }
        for effect in effects {
            match effect {
                E::Plans if online => tasks.push(self.billing_asks(true)),
                E::Status if online => tasks.push(self.billing_asks(false)),
                E::Checkout(plan, email, change) if online => {
                    let epoch = self.billing_epoch;
                    let (server, token, device) = (server.clone(), token.clone(), device.clone());
                    tasks.push(ui::in_thread(move || Message::BillingReply(epoch, B::Paid(crate::bot::checkout(&server, &token, &device, &plan, &email, change)))));
                }
                E::Wear(show_badge, show_title, title) if online => {
                    let epoch = self.billing_epoch;
                    let (server, token, device) = (server.clone(), token.clone(), device.clone());
                    tasks.push(ui::in_thread(move || Message::BillingReply(epoch, B::Worn(crate::bot::wear(&server, &token, &device, show_badge, show_title, title).map_err(|e| e.to_string())))));
                }
                E::Cancel if online => {
                    let epoch = self.billing_epoch;
                    let (server, token, device) = (server.clone(), token.clone(), device.clone());
                    tasks.push(ui::in_thread(move || Message::BillingReply(epoch, B::Cancelled(crate::bot::cancel(&server, &token, &device)))));
                }
                E::Browse(url) => {
                    if url.starts_with("https://") {
                        let _ = open::that_detached(url);
                    }
                }
                E::Remember(email) => {
                    self.settings.billing_email = email;
                    let _ = self.settings.save();
                }
                _ => {}
            }
        }
        if waited && !self.billing.waiting() && self.billing.subscription().is_some_and(|held| held.access) {
            let done = self.words.t("billing-thanks");
            self.announce(notices::Mark::Done, done, String::new(), String::new(), String::new(), notices::Link::None);
        }
        Task::batch(tasks)
    }

    pub(super) fn donate_task(&self) -> Task<Message> {
        if !self.settings.donate_replays || self.settings.token.is_empty() {
            return Task::none();
        }
        let plays: Vec<crate::donate::Play> = self
            .entries()
            .iter()
            .filter(|entry| !crate::mixed::is_shared(&entry.path))
            .map(|entry| crate::donate::Play { path: entry.path.clone(), hash: entry.replay_hash.clone() })
            .collect();
        if plays.is_empty() {
            return Task::none();
        }
        let (server, token, device) = (self.settings.server.clone(), self.settings.token.clone(), self.settings.device.clone());
        ui::streamed(move |push| {
            let _ = crate::donate::give(&server, &token, &device, &plays, &crate::donate::ledger(), |count| {
                push(count);
            });
        })
        .map(Message::Donated)
    }

    pub(super) fn handle_billing(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Billing(message) => return self.billing_step(message),
            Message::BillingReply(epoch, message) => {
                if epoch != self.billing_epoch || self.settings.token.is_empty() {
                    return Task::none();
                }
                match &message {
                    crate::billing::Message::Status(_) => self.billing_status_pending = false,
                    crate::billing::Message::Plans(_) => self.billing_plans_pending = false,
                    _ => {}
                }
                return self.billing_step(message);
            }
            _ => unreachable!("message routed to the wrong main screen handler"),
        }
    }
}
