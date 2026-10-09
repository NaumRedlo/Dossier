use std::time::{Duration, Instant};

use iced::animation::Easing;
use iced::widget::{button, column, container, row, text, text_input, Space};
use iced::{Animation, Background, Border, Color, Element, Length, Padding};

use crate::bot::{Plan, Subscription, Trouble};
use crate::lang::Words;
use crate::theme::{self, ACCENT, FAINT, INK, MUTED};
use crate::ui;

pub const FADE: Duration = Duration::from_millis(180);
pub const EVERY: Duration = Duration::from_secs(4);

#[derive(Debug, Clone)]
pub enum Message {
    Open,
    Close,
    SignIn,
    Plans(Result<Vec<Plan>, String>),
    Status(Result<Option<Subscription>, String>),
    Choose(String),
    Period(String),
    Currency(String),
    Email(String),
    Pay,
    Paid(Result<Subscription, Trouble>),
    OpenPage,
    Poll,
    AskCancel(bool),
    Again,
    Change(bool),
    Cancel,
    Cancelled(Result<Subscription, Trouble>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    Plans,
    Status,
    Checkout(String, String, bool),
    Cancel,
    Browse(String),
    Remember(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fault {
    Email,
    Offline,
    Provider,
    Busy,
    Exists,
    Plan,
    Account,
    Same,
    Lower,
    Unknown,
}

impl Fault {
    pub fn key(self) -> &'static str {
        match self {
            Fault::Email => "billing-fault-email",
            Fault::Offline => "billing-fault-offline",
            Fault::Provider => "billing-fault-provider",
            Fault::Busy => "billing-fault-busy",
            Fault::Exists => "billing-fault-exists",
            Fault::Plan => "billing-fault-plan",
            Fault::Account => "billing-fault-account",
            Fault::Same => "billing-fault-same",
            Fault::Lower => "billing-fault-lower",
            Fault::Unknown => "billing-fault-unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Held,
    Lower,
    Up,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Quiet,
    Good,
    Wait,
    Bad,
}

#[derive(Debug, Clone)]
pub struct State {
    pub shown: bool,
    pub fade: Animation<bool>,
    pub plans: Option<Vec<Plan>>,
    pub plans_failed: bool,
    pub held: Option<Option<Subscription>>,
    pub tier: Option<String>,
    pub period: String,
    pub currency: String,
    pub email: String,
    pub paying: bool,
    pub cancelling: bool,
    pub asking: bool,
    pub again: bool,
    pub changing: bool,
    pub fault: Option<Fault>,
}

impl State {
    pub fn new(email: String, currency: &str) -> State {
        State {
            shown: false,
            fade: Animation::new(false).duration(FADE).easing(Easing::EaseOutCubic),
            plans: None,
            plans_failed: false,
            held: None,
            tier: None,
            period: "MONTHLY".to_owned(),
            currency: currency.to_owned(),
            email,
            paying: false,
            cancelling: false,
            asking: false,
            again: false,
            changing: false,
            fault: None,
        }
    }

    pub fn subscription(&self) -> Option<&Subscription> {
        self.held.as_ref().and_then(|held| held.as_ref())
    }

    pub fn awaited(&self) -> Option<&Subscription> {
        let held = self.subscription()?;
        if matches!(held.state.as_str(), "pending" | "creating" | "unknown") {
            return Some(held);
        }
        held.change.as_deref().filter(|coming| matches!(coming.state.as_str(), "pending" | "creating" | "unknown"))
    }

    pub fn waiting(&self) -> bool {
        self.awaited().is_some()
    }

    pub fn may_change(&self) -> bool {
        self.subscription().is_some_and(|held| held.access) && !self.waiting()
    }

    fn day_price(&self, offer: &str, period: &str, currency: &str) -> Option<f64> {
        let plan = self.listed().iter().find(|plan| offer_of(plan) == offer && plan.periodicity == period && plan.currency == currency)?;
        day_of(plan)
    }

    pub fn step(&self, plan: &Plan) -> Step {
        let Some(held) = self.subscription() else { return Step::Up };
        if held.plan.id == plan.id {
            return Step::Held;
        }
        let (old, new) = (offer_of(&held.plan), offer_of(plan));
        if old == new {
            return Step::Up;
        }
        let spots = [(plan.periodicity.as_str(), plan.currency.as_str()), (held.plan.periodicity.as_str(), held.plan.currency.as_str())];
        for (period, currency) in spots {
            if let (Some(before), Some(after)) = (self.day_price(old, period, currency), self.day_price(new, period, currency)) {
                return if after < before { Step::Lower } else { Step::Up };
            }
        }
        Step::Up
    }

    pub fn carried(&self, plan: &Plan, now: i64) -> Option<i64> {
        let held = self.subscription()?;
        let mut paid = day_of(&held.plan)?;
        let wanted = day_of(plan)?;
        if held.plan.currency != plan.currency {
            let old = offer_of(&held.plan);
            paid = paid * self.day_price(old, &held.plan.periodicity, &plan.currency)? / self.day_price(old, &held.plan.periodicity, &held.plan.currency)?;
        }
        let tail = if held.state == "active" { held.carried_seconds } else { 0 };
        let left = (unix_of(&held.paid_until)? + tail - now).max(0) as f64;
        Some((left * (paid / wanted).min(50.0) / 86_400.0).floor() as i64)
    }

    pub fn animating(&self, now: Instant) -> bool {
        self.fade.is_animating(now)
    }

    pub fn settle(&mut self, now: Instant) {
        if self.shown && !self.fade.value() && !self.fade.is_animating(now) {
            self.shown = false;
        }
    }

    pub fn update(&mut self, message: Message, now: Instant) -> Vec<Effect> {
        let mut effects = Vec::new();
        match message {
            Message::Open => {
                self.shown = true;
                self.fade = Animation::new(self.fade.value() && self.shown).duration(FADE).easing(Easing::EaseOutCubic).go(true, now);
                self.fault = None;
                self.asking = false;
                self.again = false;
                if self.plans.is_none() {
                    effects.push(Effect::Plans);
                }
                effects.push(Effect::Status);
            }
            Message::Close => {
                self.fade = self.fade.clone().go(false, now);
            }
            Message::SignIn => {
                self.fade = self.fade.clone().go(false, now);
            }
            Message::Plans(Ok(plans)) => {
                self.plans_failed = false;
                self.plans = Some(plans);
                self.fit();
            }
            Message::Plans(Err(_)) => {
                self.plans_failed = true;
            }
            Message::Status(Ok(held)) => {
                self.held = Some(held);
                self.changing = self.changing && self.may_change();
            }
            Message::Status(Err(_)) => {}
            Message::Choose(tier) => {
                if self.offered(&tier).is_some() {
                    self.tier = Some(tier);
                }
            }
            Message::Period(period) => {
                self.period = period;
                self.fit();
            }
            Message::Currency(currency) => {
                self.currency = currency;
                self.fit();
            }
            Message::Email(email) => {
                self.email = email;
                if self.fault == Some(Fault::Email) {
                    self.fault = None;
                }
            }
            Message::Pay => {
                if self.can_pay() {
                    self.paying = true;
                    self.fault = None;
                    let email = self.email.trim().to_owned();
                    effects.push(Effect::Remember(email.clone()));
                    effects.push(Effect::Checkout(self.chosen().map(|plan| plan.id.clone()).unwrap_or_default(), email, self.changing));
                }
            }
            Message::Change(on) => {
                self.changing = on && self.may_change();
                self.asking = false;
                self.fault = None;
                if self.changing {
                    if let Some(held) = self.subscription() {
                        let (offer, period, currency) = (offer_of(&held.plan).to_owned(), held.plan.periodicity.clone(), held.plan.currency.clone());
                        (self.period, self.currency) = (period, currency);
                        self.fit();
                        let above = self.tiers().into_iter().map(|(tier, _)| tier).skip_while(|tier| *tier != offer).nth(1);
                        self.tier = above.filter(|tier| self.offered(tier).is_some()).or(Some(offer));
                    }
                }
            }
            Message::Paid(Ok(new)) => {
                self.paying = false;
                if let Some(url) = new.payment_url.clone() {
                    effects.push(Effect::Browse(url));
                }
                let over = self.changing && new.state != "active";
                self.changing = false;
                match self.held.as_mut().and_then(|held| held.as_mut()).filter(|held| over && held.access) {
                    Some(held) => held.change = Some(Box::new(new)),
                    None => self.held = Some(Some(new)),
                }
            }
            Message::Paid(Err(trouble)) => {
                self.paying = false;
                self.fault = Some(fault_of(&trouble));
                if let Some(held) = trouble.subscription {
                    self.held = Some(Some(held));
                }
                if self.fault == Some(Fault::Plan) {
                    self.plans = None;
                    effects.push(Effect::Plans);
                }
            }
            Message::OpenPage => {
                if let Some(url) = self.awaited().filter(|held| held.state == "pending").and_then(|held| held.payment_url.clone()) {
                    effects.push(Effect::Browse(url));
                }
            }
            Message::Poll => {
                if self.waiting() {
                    effects.push(Effect::Status);
                }
            }
            Message::Again => {
                self.again = true;
            }
            Message::AskCancel(asking) => {
                self.asking = asking;
            }
            Message::Cancel => {
                if self.asking && !self.cancelling {
                    self.cancelling = true;
                    self.fault = None;
                    effects.push(Effect::Cancel);
                }
            }
            Message::Cancelled(Ok(held)) => {
                self.cancelling = false;
                self.asking = false;
                self.held = Some(Some(held));
            }
            Message::Cancelled(Err(trouble)) => {
                self.cancelling = false;
                self.asking = false;
                self.fault = Some(fault_of(&trouble));
            }
        }
        effects
    }

    pub fn listed(&self) -> &[Plan] {
        self.plans.as_deref().unwrap_or_default()
    }

    pub fn offered(&self, tier: &str) -> Option<&Plan> {
        self.listed().iter().find(|plan| plan.offer_id == tier && plan.periodicity == self.period && plan.currency == self.currency)
    }

    pub fn chosen(&self) -> Option<&Plan> {
        self.offered(self.tier.as_deref()?)
    }

    pub fn tiers(&self) -> Vec<(String, String)> {
        let mut tiers: Vec<(String, String)> = Vec::new();
        for plan in self.listed() {
            if !tiers.iter().any(|(tier, _)| *tier == plan.offer_id) {
                tiers.push((plan.offer_id.clone(), plan_name(plan)));
            }
        }
        tiers
    }

    pub fn tint_of(&self, plan: &Plan) -> Color {
        let offer = offer_of(plan);
        tint_at(self.tiers().iter().position(|(tier, _)| tier == offer).unwrap_or(0))
    }

    pub fn periods(&self) -> Vec<String> {
        PERIODS.iter().filter(|period| self.listed().iter().any(|plan| plan.periodicity == **period)).map(|period| (*period).to_owned()).collect()
    }

    pub fn currencies(&self) -> Vec<String> {
        CURRENCIES.iter().filter(|currency| self.listed().iter().any(|plan| plan.currency == **currency)).map(|currency| (*currency).to_owned()).collect()
    }

    pub fn saving(&self) -> Option<u32> {
        let months = months_of(&self.period)?;
        if months <= 1.0 {
            return None;
        }
        let tier = self.tier.as_deref()?;
        let long: f64 = self.chosen()?.amount.parse().ok()?;
        let month: f64 = self.listed().iter().find(|plan| plan.offer_id == tier && plan.periodicity == "MONTHLY" && plan.currency == self.currency)?.amount.parse().ok()?;
        let saved = 1.0 - long / (month * months);
        (saved >= 0.005).then(|| (saved * 100.0).round() as u32)
    }

    fn fit(&mut self) {
        if self.plans.is_none() {
            return;
        }
        if !self.currencies().contains(&self.currency) {
            if let Some(first) = self.currencies().first() {
                self.currency = first.clone();
            }
        }
        if !self.periods().contains(&self.period) {
            if let Some(first) = self.periods().first() {
                self.period = first.clone();
            }
        }
        if self.chosen().is_none() {
            self.tier = self.tiers().into_iter().map(|(tier, _)| tier).find(|tier| self.offered(tier).is_some());
        }
    }

    pub fn can_pay(&self) -> bool {
        let free = if self.changing { self.may_change() && self.chosen().is_some_and(|plan| self.step(plan) == Step::Up) } else { self.subscription().is_none_or(|held| !held.access && !self.waiting()) };
        !self.paying && self.chosen().is_some() && valid_email(&self.email) && free
    }
}

pub fn valid_email(value: &str) -> bool {
    let value = value.trim();
    let mut parts = value.split('@');
    let (Some(local), Some(domain), None) = (parts.next(), parts.next(), parts.next()) else { return false };
    value.len() <= 254 && !local.is_empty() && domain.contains('.') && !domain.starts_with('.') && !domain.ends_with('.') && !value.chars().any(char::is_whitespace)
}

fn fault_of(trouble: &Trouble) -> Fault {
    match (trouble.status, trouble.reason.as_str()) {
        (0, _) => Fault::Offline,
        (400, _) => Fault::Email,
        (401 | 403, _) => Fault::Account,
        (404, _) => Fault::Plan,
        (409, "exists") => Fault::Exists,
        (409, "same") => Fault::Same,
        (409, "lower") => Fault::Lower,
        (409, "unknown") => Fault::Unknown,
        (409, _) => Fault::Busy,
        _ => Fault::Provider,
    }
}

pub const TINTS: [Color; 5] = [
    Color::from_rgb(0.498, 0.722, 0.643),
    Color::from_rgb(0.431, 0.639, 0.902),
    Color::from_rgb(0.663, 0.529, 0.910),
    Color::from_rgb(0.890, 0.659, 0.341),
    Color::from_rgb(0.886, 0.282, 0.282),
];

pub fn tint_at(at: usize) -> Color {
    TINTS[at.min(TINTS.len() - 1)]
}

pub fn offer_of(plan: &Plan) -> &str {
    if plan.offer_id.is_empty() { plan.id.split(':').next().unwrap_or_default() } else { &plan.offer_id }
}

pub const PERIODS: [&str; 4] = ["MONTHLY", "PERIOD_90_DAYS", "PERIOD_180_DAYS", "PERIOD_YEAR"];
pub const CURRENCIES: [&str; 3] = ["RUB", "USD", "EUR"];

pub fn months_of(period: &str) -> Option<f64> {
    match period {
        "MONTHLY" => Some(1.0),
        "PERIOD_90_DAYS" => Some(3.0),
        "PERIOD_180_DAYS" => Some(6.0),
        "PERIOD_YEAR" => Some(12.0),
        _ => None,
    }
}

pub fn days_of(period: &str) -> Option<i64> {
    match period {
        "MONTHLY" => Some(30),
        "PERIOD_90_DAYS" => Some(90),
        "PERIOD_180_DAYS" => Some(180),
        "PERIOD_YEAR" => Some(365),
        _ => None,
    }
}

pub fn day_of(plan: &Plan) -> Option<f64> {
    let amount: f64 = plan.amount.parse().ok()?;
    (amount > 0.0).then_some(amount / days_of(&plan.periodicity)? as f64)
}

pub fn money(amount: f64, currency: &str, words: &Words) -> String {
    let cents = (amount * 100.0).round() as i64;
    let whole = words.lang().group((cents / 100).max(0) as u64);
    let mark = if words.lang() == crate::lang::Lang::Ru { ',' } else { '.' };
    let number = match cents % 100 {
        0 => whole,
        rest if rest % 10 == 0 => format!("{whole}{mark}{}", rest / 10),
        rest => format!("{whole}{mark}{rest:02}"),
    };
    format!("{number} {}", symbol(currency))
}

pub fn period_name(period: &str, words: &Words) -> String {
    words.t(match period {
        "MONTHLY" => "billing-term-month",
        "PERIOD_90_DAYS" => "billing-term-quarter",
        "PERIOD_180_DAYS" => "billing-term-half",
        "PERIOD_YEAR" => "billing-term-year",
        _ => "billing-term-other",
    })
}

pub fn left(held: &Subscription, now: i64) -> Option<(i64, f32)> {
    let until = unix_of(&held.paid_until)?;
    let days = ((until - now) as f64 / 86_400.0).ceil().max(0.0) as i64;
    let whole = days_of(&held.plan.periodicity)? as f32;
    Some((days, (1.0 - days as f32 / whole).clamp(0.0, 1.0)))
}

pub fn symbol(currency: &str) -> &str {
    match currency {
        "RUB" => "₽",
        "USD" => "$",
        "EUR" => "€",
        other => other,
    }
}

pub fn price(plan: &Plan, words: &Words) -> String {
    money(plan.amount.parse().unwrap_or(0.0), &plan.currency, words)
}

pub fn period(plan: &Plan, words: &Words) -> String {
    words.t(match plan.periodicity.as_str() {
        "MONTHLY" => "billing-period-month",
        "PERIOD_90_DAYS" => "billing-period-quarter",
        "PERIOD_180_DAYS" => "billing-period-half",
        "PERIOD_YEAR" => "billing-period-year",
        _ => "billing-period-other",
    })
}

pub fn plan_name(plan: &Plan) -> String {
    [&plan.name, &plan.title].into_iter().find(|name| !name.trim().is_empty()).cloned().unwrap_or_default()
}

fn unix_of(value: &Option<String>) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(value.as_deref()?).ok().map(|date| date.timestamp())
}

pub fn until(held: &Subscription, words: &Words, now: i64) -> String {
    unix_of(&held.paid_until).map(|at| words.day(at, now)).unwrap_or_default()
}

pub fn line(state: &State, words: &Words, now: i64) -> (String, Tone) {
    let Some(held) = state.subscription() else {
        return match state.held {
            Some(_) => (words.t("billing-none"), Tone::Quiet),
            None => (words.t("billing-title"), Tone::Quiet),
        };
    };
    let name = plan_name(&held.plan);
    let date = until(held, words, now);
    match held.state.as_str() {
        "active" if held.access => (if date.is_empty() { name } else { words.with("billing-line-active", &[("name", name), ("date", date)]) }, Tone::Good),
        "cancelled" if held.access => (words.with("billing-line-cancelled", &[("name", name), ("date", date)]), Tone::Quiet),
        "pending" | "creating" | "unknown" => (words.t("billing-line-waiting"), Tone::Wait),
        "failed" => (words.t("billing-line-failed"), Tone::Bad),
        "expired" | "active" | "cancelled" => (words.t("billing-line-expired"), Tone::Quiet),
        _ => (words.t("billing-none"), Tone::Quiet),
    }
}

fn pill<'a>(said: String, ink: Color, fill: Color) -> Element<'a, Message> {
    let k = ui::fade();
    container(text(said).font(theme::SANS_SEMI).size(13.0).color(ui::faded(ink)))
        .padding([4, 11])
        .style(move |_| container::Style { background: Some(Background::Color(Color { a: fill.a * k, ..fill })), border: Border { radius: 13.0.into(), ..Border::default() }, ..container::Style::default() })
        .into()
}

fn segments<'a>(items: Vec<(String, bool, Message)>) -> Element<'a, Message> {
    let k = ui::fade();
    let mut inside = row![].spacing(2);
    for (label, on, press) in items {
        inside = inside.push(
            button(container(text(label).font(theme::SANS_SEMI).size(14.0)).center_y(34.0))
                .padding([0, 13])
                .style(ui::button_faded(move |_: &iced::Theme, status: button::Status| {
                    let lit = matches!(status, button::Status::Hovered | button::Status::Pressed);
                    button::Style {
                        background: Some(Background::Color(if on { ACCENT } else if lit { Color::from_rgba(1.0, 1.0, 1.0, 0.06) } else { Color::TRANSPARENT })),
                        text_color: if on { Color::WHITE } else { MUTED },
                        border: Border { radius: 7.0.into(), ..Border::default() },
                        shadow: iced::Shadow::default(),
                        snap: true,
                    }
                }))
                .on_press(press),
        );
    }
    container(inside)
        .padding(4)
        .style(move |_| container::Style { background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.3 * k))), border: Border { color: Color::from_rgba(1.0, 1.0, 1.0, 0.1 * k), width: 1.0, radius: 10.0.into() }, ..container::Style::default() })
        .into()
}

const TIER_HIGH: f32 = 172.0;

fn tier_card<'a>(state: &'a State, tier: &str, name: String, words: &'a Words) -> Element<'a, Message> {
    let k = ui::fade();
    let tint = tint_at(state.tiers().iter().position(|(other, _)| other == tier).unwrap_or(0));
    let Some(plan) = state.offered(tier) else {
        let inside = column![row![crate::glyphs::glyph(crate::glyphs::Icon::Heart, 14.0, Color { a: 0.5, ..tint }), text(name).font(theme::SANS_SEMI).size(16.0).color(ui::faded(FAINT))].spacing(8).align_y(iced::Center), Space::new().height(Length::Fill), text(words.t("billing-month-only")).font(theme::SANS).size(14.0).color(ui::faded(FAINT))];
        return container(ui::dashed(container(inside).padding(16).width(Length::Fill).height(TIER_HIGH), 16.0, Color::from_rgba(1.0, 1.0, 1.0, 0.2 * k))).width(Length::FillPortion(1)).into();
    };
    let chosen = state.tier.as_deref() == Some(tier);
    let own = state.changing && state.subscription().is_some_and(|held| offer_of(&held.plan) == tier);
    let mut title = row![crate::glyphs::glyph(crate::glyphs::Icon::Heart, 14.0, tint), container(text(name).font(theme::SANS_SEMI).size(16.0).color(ui::faded(INK))).width(Length::Fill)].spacing(8).align_y(iced::Center);
    if chosen {
        title = title.push(container(crate::glyphs::glyph(crate::glyphs::Icon::Check, 12.0, Color::from_rgb(0.07, 0.03, 0.04))).center(20.0).style(move |_| container::Style { background: Some(Background::Color(Color { a: k, ..tint })), border: Border { radius: 10.0.into(), ..Border::default() }, ..container::Style::default() }));
    }
    let caption: Element<'a, Message> = if own { container(text(words.t("billing-change-now")).font(theme::MONO).size(12.0).color(ui::faded(MUTED))).padding(Padding::ZERO.left(22.0).top(2.0)).into() } else { Space::new().height(0.0).into() };
    let mut inside = column![title, caption, Space::new().height(Length::Fill), text(price(plan, words)).font(theme::SANS_SEMI).size(26.0).wrapping(text::Wrapping::None).color(ui::faded(INK)), text(period(plan, words)).font(theme::SANS).size(14.0).color(ui::faded(MUTED))].spacing(2);
    if let (Some(months), Ok(amount)) = (months_of(&plan.periodicity).filter(|months| *months > 1.0), plan.amount.parse::<f64>()) {
        inside = inside.push(container(text(words.with("billing-each-month", &[("price", money((amount / months).round(), &plan.currency, words))])).font(theme::MONO_BOLD).size(13.0).color(ui::faded(Color::from_rgb(0.94, 0.77, 0.77)))).padding(Padding::ZERO.top(6.0)));
    }
    button(container(inside).padding(16).width(Length::Fill).height(Length::Fill))
        .padding(0)
        .width(Length::FillPortion(1))
        .height(TIER_HIGH)
        .style(ui::button_faded(move |_: &iced::Theme, status: button::Status| {
            let lit = matches!(status, button::Status::Hovered | button::Status::Pressed);
            let (fill, edge, wide) = match (chosen, lit) {
                (true, _) => (Color { a: 0.26, ..tint }, tint, 1.5),
                (false, true) => (Color { a: 0.16, ..tint }, Color { a: 0.7, ..tint }, 1.0),
                (false, false) => (Color { a: 0.08, ..tint }, Color { a: 0.4, ..tint }, 1.0),
            };
            button::Style { background: Some(Background::Color(fill)), text_color: INK, border: Border { color: edge, width: wide, radius: 16.0.into() }, shadow: iced::Shadow::default(), snap: true }
        }))
        .on_press(Message::Choose(tier.to_owned()))
        .into()
}

fn fault_line<'a>(fault: Fault, words: &Words) -> Element<'a, Message> {
    row![crate::glyphs::glyph(crate::glyphs::Icon::Warn, 14.0, ACCENT), text(words.t(fault.key())).font(theme::SANS).size(14.0).color(ui::faded(ACCENT))]
        .spacing(8)
        .align_y(iced::Center)
        .into()
}

pub fn bar<'a, M: 'a>(fraction: f32, ink: Color) -> Element<'a, M> {
    let k = ui::fade();
    let filled = ((fraction * 1000.0).round() as u16).clamp(1, 999);
    container(row![
        container(Space::new().height(6.0)).width(Length::FillPortion(filled)).style(move |_| container::Style { background: Some(Background::Color(Color { a: k, ..ink })), border: Border { radius: 3.0.into(), ..Border::default() }, ..container::Style::default() }),
        Space::new().width(Length::FillPortion(1000 - filled)),
    ])
    .width(Length::Fill)
    .style(move |_| container::Style { background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.12 * k))), border: Border { radius: 3.0.into(), ..Border::default() }, ..container::Style::default() })
    .into()
}

pub fn badge(held: &Subscription, words: &Words) -> (String, Color, Color) {
    match held.state.as_str() {
        "active" if held.access => (words.t("billing-badge-active"), Color::from_rgb(0.765, 0.831, 0.651), Color { a: 0.18, ..theme::NOTICE_SUCCESS }),
        "cancelled" if held.access => (words.t("billing-badge-ending"), MUTED, Color::from_rgba(1.0, 1.0, 1.0, 0.1)),
        "failed" => (words.t("billing-line-failed"), Color::from_rgb(1.0, 0.61, 0.61), Color { a: 0.24, ..ACCENT }),
        _ => (words.t("billing-badge-ended"), MUTED, Color::from_rgba(1.0, 1.0, 1.0, 0.1)),
    }
}

pub fn outlined<'a, M: Clone + 'a>(label: String, press: Option<M>) -> Element<'a, M> {
    let made = button(container(text(label).font(theme::SANS_SEMI).size(14.0)).center_y(36.0))
        .padding([0, 14])
        .style(ui::button_faded(|_: &iced::Theme, status: button::Status| {
            let lit = matches!(status, button::Status::Hovered | button::Status::Pressed);
            button::Style {
                background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, if lit { 0.08 } else { 0.0 }))),
                text_color: if matches!(status, button::Status::Disabled) { FAINT } else { INK },
                border: Border { color: Color::from_rgba(1.0, 1.0, 1.0, 0.16), width: 1.0, radius: 8.0.into() },
                shadow: iced::Shadow::default(),
                snap: true,
            }
        }));
    match press {
        Some(press) => made.on_press(press).into(),
        None => made.into(),
    }
}

fn lapsed(state: &State) -> bool {
    state.subscription().is_some_and(|held| !held.access && !state.waiting()) && !state.again
}

pub fn warm(state: &State, signed_in: bool) -> bool {
    signed_in && lapsed(state) && state.subscription().is_some_and(|held| held.state == "failed")
}

pub fn wide(state: &State, signed_in: bool) -> bool {
    signed_in && !lapsed(state) && (state.changing || state.subscription().is_none_or(|held| !held.access && !state.waiting())) && state.plans.as_ref().is_some_and(|plans| !plans.is_empty())
}

fn head<'a>(state: &State, held: &'a Subscription, words: &'a Words) -> Element<'a, Message> {
    let (said, ink, fill) = badge(held, words);
    let mut line = row![crate::glyphs::glyph(crate::glyphs::Icon::Heart, 20.0, state.tint_of(&held.plan)), text(plan_name(&held.plan)).font(theme::SANS_SEMI).size(26.0).color(ui::faded(INK)), pill(said, ink, fill), ui::grow()].spacing(12).align_y(iced::Center);
    if held.access {
        line = line.push(text(price(&held.plan, words)).font(theme::MONO_BOLD).size(16.0).color(ui::faded(INK))).push(text(period(&held.plan, words)).font(theme::SANS).size(14.0).color(ui::faded(MUTED)));
    }
    line.into()
}

fn span<'a>(held: &'a Subscription, words: &'a Words, now: i64) -> Option<Element<'a, Message>> {
    let (days, fraction) = left(held, now)?;
    let until = unix_of(&held.paid_until)?;
    let since = until - days_of(&held.plan.periodicity)? * 86_400;
    let date = |at: i64| text(words.day(at, now)).font(theme::MONO).size(13.0).color(ui::faded(MUTED));
    Some(column![
        bar(fraction, if held.state == "active" { theme::NOTICE_SUCCESS } else { theme::NOTICE_INFO }),
        row![date(since), container(text(words.n("billing-days-left", days as u64)).font(theme::MONO_BOLD).size(13.0).color(ui::faded(INK))).center_x(Length::Fill), date(until)].align_y(iced::Center),
    ].spacing(12).into())
}

pub fn sheet<'a>(state: &'a State, words: &'a Words, signed_in: bool, now: i64) -> Element<'a, Message> {
    let close = || outlined(words.t("close"), Some(Message::Close));
    let told = |said: String| text(said).font(theme::SANS).size(15.0).color(ui::faded(INK));
    let mut body = column![].spacing(18);
    if !signed_in {
        body = body.push(text(words.t("billing-heading")).font(theme::SANS_SEMI).size(26.0).color(ui::faded(INK))).push(told(words.t("billing-sign-in")));
        body = body.push(row![ui::primary(words.t("sign-in"), Some(Message::SignIn)), ui::grow(), close()].align_y(iced::Center));
        return container(body).padding(28).into();
    }
    match state.subscription() {
        Some(held) if held.access && !state.changing && !state.waiting() => {
            let active = held.state == "active";
            body = body.push(head(state, held, words));
            body = body.push(told(words.with(if active { "billing-paid-renews" } else { "billing-ends-cancelled" }, &[("date", until(held, words, now))])));
            if let Some(span) = span(held, words, now) {
                body = body.push(span);
            }
            if active && held.carried_seconds >= 86_400 {
                body = body.push(text(words.n("billing-carried", (held.carried_seconds / 86_400) as u64)).font(theme::SANS).size(14.0).color(ui::faded(MUTED)));
            }
            let mut foot = row![].spacing(8).align_y(iced::Center);
            if active && state.asking {
                foot = foot.push(ui::primary(words.t(if state.cancelling { "billing-cancelling" } else { "billing-cancel-yes" }), (!state.cancelling).then_some(Message::Cancel))).push(ui::quiet(words.t("billing-cancel-keep"), Some(Message::AskCancel(false))));
            } else if active {
                foot = foot.push(button(text(words.t("billing-cancel")).font(theme::SANS_SEMI).size(14.0)).padding([10, 14]).style(ui::button_faded(theme::danger_words)).on_press(Message::AskCancel(true)));
            }
            if !state.asking && state.plans.as_ref().is_some_and(|plans| !plans.is_empty()) {
                foot = foot.push(outlined(words.t("billing-change"), Some(Message::Change(true))));
            }
            body = body.push(foot.push(ui::grow()).push(close()));
        }
        Some(_) if state.waiting() => {
            let Some(held) = state.awaited() else { return container(body).padding(28).into() };
            let k = ui::fade();
            let dot = container(Space::new().width(11.0).height(11.0)).style(move |_| container::Style { background: Some(Background::Color(Color { a: k, ..ACCENT })), border: Border { radius: 5.5.into(), ..Border::default() }, ..container::Style::default() });
            body = body.push(row![dot, text(words.t("billing-waiting")).font(theme::SANS_SEMI).size(26.0).color(ui::faded(INK)), ui::grow(), text(price(&held.plan, words)).font(theme::MONO_BOLD).size(16.0).color(ui::faded(INK)), text(period(&held.plan, words)).font(theme::SANS).size(14.0).color(ui::faded(MUTED))].spacing(12).align_y(iced::Center));
            body = body.push(told(words.t("billing-waiting-how")));
            if let Some(old) = state.subscription().filter(|old| old.access) {
                body = body.push(text(words.with("billing-change-keeps", &[("name", plan_name(&old.plan))])).font(theme::SANS).size(14.0).color(ui::faded(MUTED)));
            }
            let mut foot = row![].spacing(8).align_y(iced::Center);
            if held.state == "pending" && held.payment_url.is_some() {
                foot = foot.push(ui::primary(words.t("billing-open-page"), Some(Message::OpenPage)));
            }
            body = body.push(foot.push(ui::grow()).push(close()));
        }
        Some(held) if lapsed(state) && !state.changing => {
            body = body.push(head(state, held, words));
            body = body.push(told(match (held.state.as_str(), until(held, words, now)) {
                ("failed", date) if !date.is_empty() => words.with("billing-failed-since", &[("date", date)]),
                ("failed", _) => words.t("billing-note-failed"),
                _ => words.t("billing-note-expired"),
            }));
            body = body.push(row![ui::primary(words.t("billing-again"), Some(Message::Again)), ui::grow(), close()].align_y(iced::Center));
        }
        _ => {
            let shut = button(crate::glyphs::glyph(crate::glyphs::Icon::Close, 16.0, MUTED)).padding(12).style(ui::button_faded(theme::bare)).on_press(if state.changing { Message::Change(false) } else { Message::Close });
            let title = text(words.t(if state.changing { "billing-change-heading" } else { "billing-heading" })).font(theme::SANS_SEMI).size(26.0).color(ui::faded(INK));
            match (&state.plans, state.plans_failed) {
                (Some(plans), _) if !plans.is_empty() => {
                    let periods = state.periods().into_iter().map(|period| (period_name(&period, words), period == state.period, Message::Period(period.clone()))).collect();
                    let currencies = state.currencies().into_iter().map(|currency| (symbol(&currency).to_owned(), currency == state.currency, Message::Currency(currency.clone()))).collect();
                    body = body.push(row![title, ui::grow(), segments(periods), segments(currencies), shut].spacing(12).align_y(iced::Center));
                    if let Some(saved) = state.saving() {
                        body = body.push(pill(words.with(&format!("billing-saving-{}", state.period.to_lowercase()), &[("n", saved.to_string())]), Color::WHITE, ACCENT));
                    }
                    let mut cards = row![].spacing(12);
                    for (tier, name) in state.tiers() {
                        cards = cards.push(tier_card(state, &tier, name, words));
                    }
                    body = body.push(cards);
                    let field = text_input(&words.t("billing-email-hint"), &state.email)
                        .on_input(Message::Email)
                        .on_submit(Message::Pay)
                        .font(theme::SANS)
                        .size(15.0)
                        .padding([12, 14])
                        .width(340.0)
                        .style(theme::field_faded(ui::fade()));
                    let step = state.chosen().filter(|_| state.changing).map(|plan| state.step(plan));
                    let why = match (step, state.chosen(), state.subscription()) {
                        (Some(Step::Held), _, _) => words.t("billing-change-same"),
                        (Some(Step::Lower), _, Some(held)) => words.with("billing-change-lower", &[("date", until(held, words, now))]),
                        (Some(Step::Up), Some(plan), _) => match state.carried(plan, now).filter(|days| *days > 0) {
                            Some(days) => format!("{} {}", words.t("billing-change-note"), words.n("billing-change-days", days as u64)),
                            None => words.t("billing-change-note"),
                        },
                        _ => words.t("billing-email-why"),
                    };
                    let label = match (state.paying, state.chosen()) {
                        (true, _) => words.t("billing-creating"),
                        (false, Some(plan)) if state.changing => words.with("billing-change-pay", &[("price", price(plan, words))]),
                        (false, Some(plan)) => words.with("billing-pay", &[("price", price(plan, words))]),
                        (false, None) => words.t("billing-pay-plain"),
                    };
                    body = body.push(row![
                        column![ui::mono_small(words.t("billing-email").to_uppercase(), MUTED), field].spacing(8),
                        container(text(why).font(theme::SANS).size(14.0).color(ui::faded(MUTED))).width(Length::Fill),
                        ui::primary(label, state.can_pay().then_some(Message::Pay)),
                    ].spacing(18).align_y(iced::Bottom));
                }
                (Some(_), _) => body = body.push(row![title, ui::grow(), shut].align_y(iced::Center)).push(told(words.t("billing-no-plans"))),
                (None, true) => body = body.push(row![title, ui::grow(), shut].align_y(iced::Center)).push(told(words.t("billing-plans-failed"))),
                (None, false) => body = body.push(row![title, ui::grow(), shut].align_y(iced::Center)).push(told(words.t("billing-loading"))),
            }
        }
    }
    if let Some(fault) = state.fault {
        body = body.push(fault_line(fault, words));
    }
    container(body).padding(28).into()
}

pub fn tint(tone: Tone) -> Color {
    match tone {
        Tone::Quiet => FAINT,
        Tone::Good => theme::NOTICE_SUCCESS,
        Tone::Wait => theme::NOTICE_INFO,
        Tone::Bad => ACCENT,
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn plan(offer: &str, period: &str, currency: &str, amount: &str) -> Plan {
        Plan { id: format!("{offer}:{currency}:{period}"), offer_id: offer.into(), title: "Subscription".into(), name: offer.to_uppercase(), description: String::new(), currency: currency.into(), periodicity: period.into(), amount: amount.into() }
    }

    fn catalogue() -> Vec<Plan> {
        vec![
            plan("a", "MONTHLY", "RUB", "90.0"), plan("a", "MONTHLY", "USD", "5.0"),
            plan("b", "MONTHLY", "RUB", "240.0"), plan("b", "MONTHLY", "USD", "7.0"),
            plan("b", "PERIOD_YEAR", "RUB", "2400.0"), plan("b", "PERIOD_YEAR", "USD", "70.0"),
            plan("b", "PERIOD_90_DAYS", "RUB", "690.0"),
        ]
    }

    fn held(state: &str, access: bool, until: Option<&str>) -> Subscription {
        Subscription { state: state.into(), access, plan: plan("b", "MONTHLY", "RUB", "240.0"), paid_until: until.map(str::to_owned), cancelled_at: None, created_at: None, payment_url: (state == "pending").then(|| "https://pay.example/1".to_owned()), carried_seconds: 0, change: None }
    }

    fn open() -> State {
        let mut state = State::new(String::new(), "RUB");
        state.update(Message::Open, Instant::now());
        state.update(Message::Plans(Ok(catalogue())), Instant::now());
        state
    }

    #[test]
    fn opening_asks_for_the_plans_once_and_always_for_the_status() {
        let mut state = State::new("me@example.org".into(), "RUB");
        assert_eq!(state.update(Message::Open, Instant::now()), vec![Effect::Plans, Effect::Status]);
        state.update(Message::Plans(Ok(catalogue())), Instant::now());
        state.update(Message::Close, Instant::now());
        assert_eq!(state.update(Message::Open, Instant::now()), vec![Effect::Status]);
    }

    #[test]
    fn tiers_periods_and_currencies_come_from_the_catalogue_and_the_choice_always_names_a_real_price() {
        let mut state = open();
        assert_eq!(state.tiers(), vec![("a".to_owned(), "A".to_owned()), ("b".to_owned(), "B".to_owned())]);
        assert_eq!(state.periods(), vec!["MONTHLY", "PERIOD_90_DAYS", "PERIOD_YEAR"]);
        assert_eq!(state.currencies(), vec!["RUB", "USD"]);
        assert_eq!(state.chosen().map(|plan| plan.id.as_str()), Some("a:RUB:MONTHLY"));
        state.update(Message::Period("PERIOD_YEAR".into()), Instant::now());
        assert_eq!(state.chosen().map(|plan| plan.id.as_str()), Some("b:RUB:PERIOD_YEAR"), "a tier without that term gives way to one that has it");
        assert!(state.offered("a").is_none());
        state.update(Message::Choose("a".into()), Instant::now());
        assert_eq!(state.tier.as_deref(), Some("b"), "a tier that is not sold for the term cannot be chosen");
        state.update(Message::Currency("USD".into()), Instant::now());
        assert_eq!(state.chosen().map(|plan| plan.id.as_str()), Some("b:USD:PERIOD_YEAR"));
        state.update(Message::Period("PERIOD_90_DAYS".into()), Instant::now());
        assert!(state.chosen().is_none() || state.chosen().is_some_and(|plan| plan.currency == "USD"), "no price is invented for a missing combination");
        let mut euro = State::new(String::new(), "EUR");
        euro.update(Message::Plans(Ok(catalogue())), Instant::now());
        assert_eq!(euro.currency, "RUB", "a currency the catalogue lacks falls back to one it has");
    }

    #[test]
    fn a_longer_term_tells_how_much_it_saves_against_paying_monthly() {
        let mut state = open();
        state.update(Message::Choose("b".into()), Instant::now());
        assert_eq!(state.saving(), None);
        state.update(Message::Period("PERIOD_YEAR".into()), Instant::now());
        assert_eq!(state.saving(), Some(17));
        state.update(Message::Period("PERIOD_90_DAYS".into()), Instant::now());
        assert_eq!(state.saving(), Some(4));
    }

    #[test]
    fn paying_needs_a_real_looking_email_and_sends_the_chosen_price_once() {
        let mut state = open();
        assert!(state.update(Message::Pay, Instant::now()).is_empty());
        for bad in ["", "me", "me@", "@example.org", "me@example", "me@@example.org", "m e@example.org", "me@.org", "me@example."] {
            state.update(Message::Email(bad.into()), Instant::now());
            assert!(!state.can_pay(), "{bad}");
        }
        state.update(Message::Email("  me@example.org ".into()), Instant::now());
        assert_eq!(state.update(Message::Pay, Instant::now()), vec![Effect::Remember("me@example.org".into()), Effect::Checkout("a:RUB:MONTHLY".into(), "me@example.org".into(), false)]);
        assert!(state.update(Message::Pay, Instant::now()).is_empty());
        let effects = state.update(Message::Paid(Ok(held("pending", false, None))), Instant::now());
        assert_eq!(effects, vec![Effect::Browse("https://pay.example/1".into())]);
        assert!(state.waiting() && !state.can_pay());
        assert_eq!(state.update(Message::Poll, Instant::now()), vec![Effect::Status]);
        state.update(Message::Status(Ok(Some(held("active", true, Some("2999-01-01T00:00:00Z"))))), Instant::now());
        assert!(!state.waiting() && state.update(Message::Poll, Instant::now()).is_empty());
        state.update(Message::Status(Err("offline".into())), Instant::now());
        assert_eq!(state.subscription().map(|held| held.state.as_str()), Some("active"));
    }

    #[test]
    fn every_refusal_has_its_own_words() {
        let mut state = open();
        let said = |status: u16, reason: &str, held: Option<Subscription>| Trouble { status, reason: reason.into(), subscription: held };
        for (trouble, fault) in [
            (said(0, "timeout", None), Fault::Offline), (said(400, "", None), Fault::Email), (said(401, "", None), Fault::Account), (said(403, "", None), Fault::Account),
            (said(409, "busy", Some(held("pending", false, None))), Fault::Busy), (said(409, "exists", Some(held("active", true, None))), Fault::Exists),
            (said(502, "", None), Fault::Provider), (said(503, "", None), Fault::Provider),
        ] {
            state.update(Message::Paid(Err(trouble.clone())), Instant::now());
            assert_eq!(state.fault, Some(fault), "{trouble:?}");
            assert!(!state.paying);
        }
        assert_eq!(state.update(Message::Paid(Err(said(404, "unknown plan", None))), Instant::now()), vec![Effect::Plans]);
        assert!(state.plans.is_none());
    }

    #[test]
    fn cancelling_needs_a_second_yes() {
        let mut state = open();
        state.update(Message::Status(Ok(Some(held("active", true, Some("2999-01-01T00:00:00Z"))))), Instant::now());
        assert!(state.update(Message::Cancel, Instant::now()).is_empty());
        state.update(Message::AskCancel(true), Instant::now());
        assert_eq!(state.update(Message::Cancel, Instant::now()), vec![Effect::Cancel]);
        assert!(state.update(Message::Cancel, Instant::now()).is_empty());
        state.update(Message::Cancelled(Ok(held("cancelled", true, Some("2999-01-01T00:00:00Z")))), Instant::now());
        assert!(!state.cancelling && !state.asking);
        state.update(Message::AskCancel(true), Instant::now());
        state.update(Message::Cancel, Instant::now());
        state.update(Message::Cancelled(Err(Trouble { status: 502, reason: String::new(), subscription: None })), Instant::now());
        assert_eq!(state.fault, Some(Fault::Provider));
    }

    #[test]
    fn closing_fades_out_before_the_sheet_is_gone() {
        let mut state = State::new(String::new(), "RUB");
        let now = Instant::now();
        state.update(Message::Open, now);
        state.update(Message::Close, now);
        state.settle(now);
        assert!(state.shown);
        state.settle(now + FADE * 3);
        assert!(!state.shown);
    }

    #[test]
    fn money_drops_empty_cents_groups_thousands_and_follows_the_language() {
        let ru = Words::new(crate::lang::Lang::Ru);
        let en = Words::new(crate::lang::Lang::En);
        assert_eq!(money(90.0, "RUB", &ru), "90 ₽");
        assert_eq!(money(6.5, "EUR", &ru), "6,5 €");
        assert_eq!(money(6.5, "EUR", &en), "6.5 €");
        assert_eq!(money(199.99, "USD", &en), "199.99 $");
        assert!(money(19900.0, "RUB", &ru).starts_with("19") && money(19900.0, "RUB", &ru).ends_with("900 ₽") && money(19900.0, "RUB", &ru) != "19900 ₽");
        for term in PERIODS.iter().chain(["SOMETHING"].iter()) {
            assert!(!period_name(term, &ru).starts_with("billing-") && !period(&plan("a", term, "RUB", "1"), &ru).starts_with("billing-"), "{term}");
        }
    }

    #[test]
    fn days_left_and_the_bar_follow_the_paid_through_date() {
        let now = 1_790_000_000;
        let until = chrono::DateTime::from_timestamp(now + 21 * 86_400, 0).unwrap().to_rfc3339();
        let (days, fraction) = left(&held("active", true, Some(&until)), now).unwrap();
        assert_eq!(days, 21);
        assert!((fraction - 0.3).abs() < 0.01);
        assert!(left(&held("active", true, None), now).is_none());
    }

    #[test]
    fn the_menu_line_and_the_badge_tell_the_state_in_a_few_words() {
        let words = Words::new(crate::lang::Lang::En);
        let mut state = State::new(String::new(), "USD");
        state.held = Some(None);
        assert_eq!(line(&state, &words, 0), (words.t("billing-none"), Tone::Quiet));
        for (held, tone) in [(held("pending", false, None), Tone::Wait), (held("failed", false, None), Tone::Bad), (held("expired", false, None), Tone::Quiet), (held("active", true, Some("2999-01-01T00:00:00Z")), Tone::Good)] {
            state.held = Some(Some(held.clone()));
            let (said, got) = line(&state, &words, 0);
            assert_eq!(got, tone, "{held:?}");
            assert!(!said.is_empty() && !said.contains("billing-"), "{said}");
            assert!(!badge(&held, &words).0.contains("billing-"));
        }
    }
    #[test]
    fn a_held_subscription_changes_to_a_higher_tier_and_shows_what_is_carried_over() {
        let mut state = open();
        let now = chrono::DateTime::parse_from_rfc3339("2026-10-01T12:00:00Z").unwrap().timestamp();
        state.email = "me@example.org".into();
        state.update(Message::Status(Ok(Some(Subscription { plan: plan("a", "MONTHLY", "RUB", "90.0"), ..held("active", true, Some("2026-10-21T12:00:00Z")) }))), Instant::now());
        assert!(state.may_change() && !state.can_pay());
        state.update(Message::Change(true), Instant::now());
        assert!(state.changing && state.tier.as_deref() == Some("b") && state.period == "MONTHLY" && state.currency == "RUB");
        let up = state.chosen().unwrap().clone();
        assert_eq!(state.step(&up), Step::Up);
        assert_eq!(state.carried(&up, now), Some(7), "twenty days at 90 are seven whole days at 240");
        assert_eq!(state.update(Message::Pay, Instant::now()), vec![Effect::Remember("me@example.org".into()), Effect::Checkout("b:RUB:MONTHLY".into(), "me@example.org".into(), true)]);
        let coming = Subscription { plan: up.clone(), ..held("pending", false, None) };
        assert_eq!(state.update(Message::Paid(Ok(coming)), Instant::now()), vec![Effect::Browse("https://pay.example/1".into())]);
        assert!(!state.changing && state.waiting());
        assert_eq!(state.subscription().map(|held| held.plan.id.as_str()), Some("a:RUB:MONTHLY"), "the held tier stays until the payment passes");
        assert_eq!(state.awaited().map(|held| held.plan.id.as_str()), Some("b:RUB:MONTHLY"));
        assert_eq!(state.update(Message::OpenPage, Instant::now()), vec![Effect::Browse("https://pay.example/1".into())]);
        assert_eq!(state.update(Message::Poll, Instant::now()), vec![Effect::Status]);
        state.update(Message::Status(Ok(Some(Subscription { carried_seconds: 7 * 86_400, ..held("active", true, Some("2026-11-01T12:00:00Z")) }))), Instant::now());
        assert!(!state.waiting() && state.subscription().is_some_and(|held| held.carried_seconds == 7 * 86_400));
    }

    #[test]
    fn the_held_plan_and_a_lower_tier_cannot_be_paid_as_a_change() {
        let mut state = open();
        state.email = "me@example.org".into();
        state.update(Message::Status(Ok(Some(held("active", true, Some("2026-10-21T12:00:00Z"))))), Instant::now());
        state.update(Message::Change(true), Instant::now());
        assert_eq!(state.tier.as_deref(), Some("b"), "with no tier above, the held one stays picked");
        assert_eq!(state.step(&state.chosen().unwrap().clone()), Step::Held);
        assert!(!state.can_pay());
        state.update(Message::Choose("a".into()), Instant::now());
        assert_eq!(state.step(&state.chosen().unwrap().clone()), Step::Lower);
        assert!(!state.can_pay());
        state.update(Message::Choose("b".into()), Instant::now());
        state.update(Message::Period("PERIOD_YEAR".into()), Instant::now());
        assert_eq!(state.step(&state.chosen().unwrap().clone()), Step::Up, "another term of the same tier is a change");
        assert!(state.can_pay());
        state.update(Message::Currency("USD".into()), Instant::now());
        let now = chrono::DateTime::parse_from_rfc3339("2026-10-01T12:00:00Z").unwrap().timestamp();
        let wanted = state.chosen().unwrap().clone();
        assert_eq!(state.carried(&wanted, now), Some((20.0_f64 * (7.0 / 30.0) / (70.0 / 365.0)).floor() as i64), "the paid price is weighed in the new currency through the catalogue");
        state.update(Message::Change(false), Instant::now());
        assert!(!state.changing && !state.can_pay());
        for (reason, fault) in [("same", Fault::Same), ("lower", Fault::Lower), ("unknown", Fault::Unknown)] {
            state.update(Message::Paid(Err(Trouble { status: 409, reason: reason.into(), subscription: None })), Instant::now());
            assert_eq!(state.fault, Some(fault));
        }
    }

}
