use std::time::Duration;

use iced::widget::{button, column, container, row, text, text_input, toggler};
use iced::{Element, Length, Subscription, Task};

use crate::bot::{self, Paired, Refused};
use crate::checks::{self, Outcome};
use crate::lang::{Lang, Words};
use crate::settings::Settings;
use crate::sources::{self, Source};
use crate::theme::{self, INK, MUTED};
use crate::ui::{self, Glyph, Line, Mood};

pub const STEPS: u64 = 4;
const POLL: Duration = Duration::from_secs(3);
pub const FFMPEG_HOME: &str = "https://ffmpeg.org/download.html";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Language,
    Folder,
    Device,
    Bot,
    Checks,
}

impl Step {
    fn number(self) -> u64 {
        match self {
            Step::Language => 1,
            Step::Folder => 2,
            Step::Device => 3,
            Step::Bot => 4,
            Step::Checks => 4,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pairing {
    Idle,
    Asking,
    Waiting { code: String, link: String },
    Linked { who: String },
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Check {
    Folder,
    Ffmpeg,
    Engine,
    Bot,
}

pub const CHECKS: [Check; 4] = [Check::Folder, Check::Ffmpeg, Check::Engine, Check::Bot];

#[derive(Debug, Clone)]
pub enum Message {
    PickLang(Lang),
    Continue,
    Back,
    Looked(Vec<Source>),
    Switch(usize, bool),
    Browse,
    Browsed(Option<Source>),
    BotOnly,
    Device(String),
    PairAsked(Result<(String, String), Refused>),
    Poll,
    Polled(Result<Paired, Refused>),
    OpenTelegram,
    Later,
    Checked(Check, Outcome),
    CheckAgain,
    WhereToGet,
    Finish,
}

#[derive(Clone)]
pub struct FirstRun {
    pub words: Words,
    pub settings: Settings,
    pub step: Step,
    pub looked: bool,
    pub sources: Vec<Source>,
    pub bot_only: bool,
    pub pairing: Pairing,
    pub qr: Option<ui::Qr>,
    pub checks: Vec<(Check, Option<Outcome>)>,
}

pub enum Done {
    NotYet,
    Finished(Settings),
}

impl FirstRun {
    pub fn new() -> (FirstRun, Task<Message>) {
        let settings = Settings::default();
        let made = FirstRun {
            words: Words::new(settings.lang),
            settings,
            step: Step::Language,
            looked: false,
            sources: Vec::new(),
            bot_only: false,
            pairing: Pairing::Idle,
            qr: None,
            checks: Vec::new(),
        };
        (made, Task::perform(async { sources::find() }, Message::Looked))
    }

    pub fn subscription(&self) -> Subscription<Message> {
        match self.pairing {
            Pairing::Waiting { .. } => iced::time::every(POLL).map(|_| Message::Poll),
            _ => Subscription::none(),
        }
    }

    fn live_sources(&self) -> Vec<Source> {
        self.sources.iter().filter(|s| s.on).cloned().collect()
    }

    fn start_checks(&mut self) -> Task<Message> {
        self.step = Step::Checks;
        self.checks = CHECKS.iter().map(|c| (*c, None)).collect();
        let sources = self.live_sources();
        let server = self.settings.server.clone();
        let token = self.settings.token.clone();
        let name = self.settings.device.clone();
        let bot_only = self.bot_only;
        Task::perform(async move { checks::folder_or_bot_only(&sources, bot_only) }, |o| Message::Checked(Check::Folder, o))
            .chain(Task::perform(async { checks::ffmpeg() }, |o| Message::Checked(Check::Ffmpeg, o)))
            .chain({
                let (server, token, name) = (server.clone(), token.clone(), name.clone());
                Task::perform(async move { checks::engine(&server, &token, &name) }, |o| Message::Checked(Check::Engine, o))
            })
            .chain(Task::perform(async move { checks::bot(&server, &token, &name) }, |o| Message::Checked(Check::Bot, o)))
    }

    fn ask_to_pair(&mut self) -> Task<Message> {
        if !self.settings.token.is_empty() {
            return Task::none();
        }
        self.pairing = Pairing::Asking;
        let server = self.settings.server.clone();
        let name = self.settings.device.clone();
        Task::perform(
            async move { bot::pair(&server, &name).map(|p| (p.code, p.link)) },
            Message::PairAsked,
        )
    }

    pub fn update(&mut self, message: Message) -> (Task<Message>, Done) {
        let mut done = Done::NotYet;
        let task = match message {
            Message::PickLang(lang) => {
                self.settings.lang = lang;
                self.words = Words::new(lang);
                Task::none()
            }
            Message::Continue => match self.step {
                Step::Language => {
                    self.step = Step::Folder;
                    Task::none()
                }
                Step::Folder => {
                    self.step = Step::Device;
                    Task::none()
                }
                Step::Device => {
                    self.step = Step::Bot;
                    self.ask_to_pair()
                }
                Step::Bot => self.start_checks(),
                Step::Checks => Task::none(),
            },
            Message::Back => {
                self.step = match self.step {
                    Step::Language | Step::Folder => Step::Language,
                    Step::Device => Step::Folder,
                    Step::Bot => Step::Device,
                    Step::Checks => Step::Bot,
                };
                Task::none()
            }
            Message::Looked(found) => {
                self.looked = true;
                self.sources = found;
                Task::none()
            }
            Message::Switch(index, on) => {
                if let Some(source) = self.sources.get_mut(index) {
                    source.on = on;
                }
                Task::none()
            }
            Message::Browse => Task::perform(
                async {
                    let picked = rfd::AsyncFileDialog::new().pick_folder().await?;
                    sources::read(picked.path())
                },
                Message::Browsed,
            ),
            Message::Browsed(found) => {
                if let Some(source) = found {
                    if !self.sources.iter().any(|s| s.root == source.root) {
                        self.sources.push(source);
                    }
                    self.bot_only = false;
                }
                Task::none()
            }
            Message::BotOnly => {
                self.bot_only = true;
                self.step = Step::Device;
                Task::none()
            }
            Message::Device(name) => {
                self.settings.device = name;
                Task::none()
            }
            Message::PairAsked(Ok((code, link))) => {
                let link = if link.is_empty() {
                    format!("https://t.me/OneNineEightFourGlobalBot?start=pair-{}", bot::tidy(&code))
                } else {
                    link
                };
                self.qr = qr_for(&link);
                self.pairing = Pairing::Waiting { code: bot::pretty(&code), link };
                Task::none()
            }
            Message::PairAsked(Err(_)) => {
                self.pairing = Pairing::Unavailable;
                Task::none()
            }
            Message::Poll => match &self.pairing {
                Pairing::Waiting { code, .. } => {
                    let server = self.settings.server.clone();
                    let code = bot::tidy(code);
                    Task::perform(async move { bot::paired(&server, &code) }, Message::Polled)
                }
                _ => Task::none(),
            },
            Message::Polled(Ok(Paired::Linked { token, who })) => {
                self.settings.token = token;
                self.settings.linked_as = who.clone();
                self.pairing = Pairing::Linked { who };
                Task::none()
            }
            Message::Polled(Ok(Paired::Gone)) | Message::Polled(Err(Refused::NotThere)) => self.ask_to_pair(),
            Message::Polled(_) => Task::none(),
            Message::OpenTelegram => {
                if let Pairing::Waiting { link, .. } = &self.pairing {
                    let _ = open::that_detached(link);
                }
                Task::none()
            }
            Message::Later => self.start_checks(),
            Message::Checked(which, outcome) => {
                if let Some(slot) = self.checks.iter_mut().find(|(c, _)| *c == which) {
                    slot.1 = Some(outcome);
                }
                Task::none()
            }
            Message::CheckAgain => self.start_checks(),
            Message::WhereToGet => {
                let _ = open::that_detached(FFMPEG_HOME);
                Task::none()
            }
            Message::Finish => {
                let mut settings = self.settings.clone();
                settings.sources = self.sources.clone();
                settings.bot_only = self.bot_only;
                done = Done::Finished(settings);
                Task::none()
            }
        };
        (task, done)
    }

    fn checks_done(&self) -> u64 {
        self.checks.iter().filter(|(_, o)| o.is_some()).count() as u64
    }

    fn checks_failed(&self) -> Vec<Check> {
        self.checks
            .iter()
            .filter_map(|(c, o)| match o {
                Some(Outcome::Failed(_)) => Some(*c),
                _ => None,
            })
            .collect()
    }

    fn all_checked(&self) -> bool {
        self.checks_done() == self.checks.len() as u64 && !self.checks.is_empty()
    }

    pub fn view(&self) -> Element<'_, Message> {
        let w = &self.words;
        let (head_glyph, head_words, head_count) = match self.step {
            Step::Checks if self.all_checked() && self.checks_failed().is_empty() => {
                (Glyph::Tick, w.t("everything-works"), w.of(self.checks_done(), CHECKS.len() as u64))
            }
            Step::Checks => (Glyph::Dot, w.t("checking"), w.of(self.checks_done(), CHECKS.len() as u64)),
            step => (Glyph::Dot, w.t("setting-up"), w.of(step.number(), STEPS)),
        };
        let card = match self.step {
            Step::Checks => self.checks_card(),
            _ => ui::card(ui::ledger(&self.steps_ledger(), None), Some(self.step_body())),
        };
        let column = column![
            ui::brand(),
            ui::gap(48.0),
            container(ui::headline(head_glyph, head_words, head_count)).width(Length::Fill).center_x(Length::Fill),
            ui::gap(16.0),
            card,
        ]
        .width(theme::COLUMN);
        container(column)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding([96, 0])
            .center_x(Length::Fill)
            .into()
    }

    fn steps_ledger(&self) -> Vec<Line> {
        let w = &self.words;
        let names = [
            ("step-language", Step::Language),
            ("step-folder", Step::Folder),
            ("step-device", Step::Device),
            ("step-bot", Step::Bot),
        ];
        let at = self.step.number();
        names
            .iter()
            .map(|(key, step)| {
                let n = step.number();
                let mood = if n < at {
                    Mood::Done
                } else if n == at {
                    Mood::Now
                } else {
                    Mood::Todo
                };
                Line::new(mood, w.t(key))
            })
            .collect()
    }

    fn step_body(&self) -> Element<'_, Message> {
        match self.step {
            Step::Language => self.language_body(),
            Step::Folder => self.folder_body(),
            Step::Device => self.device_body(),
            Step::Bot => self.bot_body(),
            Step::Checks => column![].into(),
        }
    }

    fn language_body(&self) -> Element<'_, Message> {
        let w = &self.words;
        let mut options = row![].spacing(8);
        for lang in Lang::ALL {
            let on = lang == w.lang();
            options = options.push(
                button(container(text(lang.own_name()).font(theme::SANS_SEMI).size(theme::LEAD).color(INK)).center(Length::Fill))
                    .width(Length::Fill)
                    .height(64.0)
                    .style(theme::choice(on))
                    .on_press(Message::PickLang(lang)),
            );
        }
        column![
            ui::heading(w.t("step-language"), w.t("language-why")),
            options,
            row![ui::grow(), ui::primary(w.t("continue"), Some(Message::Continue))].spacing(8),
        ]
        .spacing(16)
        .into()
    }

    fn source_row(&self, index: usize, source: &Source) -> Element<'_, Message> {
        let w = &self.words;
        let mut counts = Vec::new();
        if let Some(maps) = source.maps {
            counts.push(w.count("maps-label", maps));
        }
        if source.skin_count > 0 {
            counts.push(w.count("skins-label", source.skin_count));
        }
        counts.push(w.count("replays-label", source.replay_count));
        container(
            row![
                ui::tag(source.kind.tag().to_owned()),
                text(source.shown()).font(theme::MONO).size(theme::BODY).color(INK),
                ui::grow(),
                text(counts.join(" · ")).font(theme::SANS).size(theme::CAPTION).color(MUTED),
                toggler(source.on).on_toggle(move |on| Message::Switch(index, on)).size(18.0).style(theme::switch),
            ]
            .spacing(12)
            .align_y(iced::Center),
        )
        .padding([0, 12])
        .height(44.0)
        .center_y(44.0)
        .width(Length::Fill)
        .style(theme::well)
        .into()
    }

    fn folder_body(&self) -> Element<'_, Message> {
        let w = &self.words;
        if !self.looked {
            return column![ui::heading(w.t("step-folder"), "…".to_owned())].into();
        }
        let mut body = column![].spacing(16);
        match self.sources.len() {
            0 => {
                body = body.push(ui::heading(w.t("step-folder"), w.t("folder-missing")));
                body = body.push(ui::faint(w.t("folder-missing-note")));
                body = body.push(
                    row![
                        ui::quiet(w.t("back"), Some(Message::Back)),
                        ui::grow(),
                        ui::quiet(w.t("bot-only"), Some(Message::BotOnly)),
                        ui::primary(w.t("browse"), Some(Message::Browse)),
                    ]
                    .spacing(8),
                );
            }
            1 => {
                let source = &self.sources[0];
                body = body.push(ui::heading(w.t("step-folder"), w.t("folder-found")));
                body = body.push(ui::well(
                    row![
                        text(source.shown()).font(theme::MONO).size(theme::BODY).color(INK),
                        ui::grow(),
                        ui::tag(source.kind.tag().to_owned()),
                    ]
                    .spacing(8)
                    .align_y(iced::Center)
                    .into(),
                ));
                let maps = match source.maps {
                    Some(n) => ui::tile(w.lang().group(n), w.n("maps-label", n)),
                    None => ui::tile("—".to_owned(), w.t("maps-from-mirrors")),
                };
                let skins = match source.kind {
                    sources::Kind::Lazer => ui::tile(w.lang().group(source.skin_count), w.t("skins-exported")),
                    _ => ui::tile(w.lang().group(source.skin_count), w.n("skins-label", source.skin_count)),
                };
                body = body.push(
                    row![maps, skins, ui::tile(w.lang().group(source.replay_count), w.n("replays-label", source.replay_count))].spacing(8),
                );
                body = body.push(
                    row![
                        ui::quiet(w.t("back"), Some(Message::Back)),
                        ui::grow(),
                        ui::quiet(w.t("add-another"), Some(Message::Browse)),
                        ui::primary(w.t("use-this"), Some(Message::Continue)),
                    ]
                    .spacing(8),
                );
            }
            _ => {
                body = body.push(ui::heading(w.t("step-folder"), w.t("folder-found-two")));
                let mut rows = column![].spacing(8);
                for (index, source) in self.sources.iter().enumerate() {
                    rows = rows.push(self.source_row(index, source));
                }
                body = body.push(rows);
                let any = self.sources.iter().any(|s| s.on);
                body = body.push(
                    row![
                        ui::quiet(w.t("back"), Some(Message::Back)),
                        ui::grow(),
                        ui::quiet(w.t("add-another"), Some(Message::Browse)),
                        ui::primary(w.t("use-both"), any.then_some(Message::Continue)),
                    ]
                    .spacing(8),
                );
            }
        }
        body.into()
    }

    fn device_body(&self) -> Element<'_, Message> {
        let w = &self.words;
        let ready = !self.settings.device.trim().is_empty();
        column![
            ui::heading(w.t("step-device"), w.t("device-why")),
            column![
                ui::cap(w.t("device-name")),
                text_input("", &self.settings.device)
                    .on_input(Message::Device)
                    .on_submit(Message::Continue)
                    .font(theme::MONO)
                    .size(theme::BODY)
                    .padding([6, 12])
                    .style(theme::field),
            ]
            .spacing(4),
            row![
                ui::quiet(w.t("back"), Some(Message::Back)),
                ui::grow(),
                ui::primary(w.t("continue"), ready.then_some(Message::Continue)),
            ]
            .spacing(8),
        ]
        .spacing(16)
        .into()
    }

    fn bot_body(&self) -> Element<'_, Message> {
        let w = &self.words;
        let why = if self.bot_only { w.t("bot-why-only") } else { w.t("bot-why") };
        let linked = matches!(self.pairing, Pairing::Linked { .. });
        let mut body = column![ui::heading(w.t("step-bot"), why)].spacing(16);
        match &self.pairing {
            _ => {
                let code = match &self.pairing {
                    Pairing::Waiting { code, .. } => code.clone(),
                    Pairing::Linked { .. } => "".to_owned(),
                    _ => "····-····".to_owned(),
                };
                let mut left = column![].spacing(12).width(Length::Fill);
                if !linked {
                    left = left.push(column![
                        ui::cap(w.t("code-cap")),
                        text(code).font(theme::MONO_BOLD).size(theme::CODE).color(INK),
                    ]);
                    left = left.push(row![ui::primary(
                        w.t("open-telegram"),
                        matches!(self.pairing, Pairing::Waiting { .. }).then_some(Message::OpenTelegram)
                    )]);
                }
                let status = match &self.pairing {
                    Pairing::Linked { who } => row![
                        ui::glyph(Glyph::Tick),
                        text(if who.is_empty() { w.t("linked") } else { w.who("linked-to", who) })
                            .font(theme::SANS)
                            .size(theme::BODY)
                            .color(INK)
                    ],
                    Pairing::Unavailable => row![
                        ui::glyph(Glyph::None),
                        text(w.t("no-pairing-yet")).font(theme::SANS).size(theme::BODY).color(MUTED)
                    ],
                    _ => row![
                        ui::glyph(Glyph::Dot),
                        text(w.t("waiting-telegram")).font(theme::SANS).size(theme::BODY).color(MUTED)
                    ],
                };
                left = left.push(status.spacing(10).align_y(iced::Center));
                let mut sides = row![left].spacing(24).align_y(iced::Top);
                if let (Some(qr), false) = (&self.qr, linked) {
                    sides = sides.push(ui::qr(qr));
                }
                body = body.push(sides);
                if !self.bot_only && !linked {
                    body = body.push(row![ui::link(w.t("later"), Message::Later)]);
                }
                body = body.push(
                    row![
                        ui::quiet(w.t("back"), Some(Message::Back)),
                        ui::grow(),
                        ui::primary(w.t("continue"), linked.then_some(Message::Continue)),
                    ]
                    .spacing(8),
                );
            }
        }
        body.into()
    }

    fn check_line(&self, which: Check, outcome: &Option<Outcome>, reached: bool) -> Line {
        let w = &self.words;
        let name = match which {
            Check::Folder => w.t("check-folder"),
            Check::Ffmpeg => w.t("check-ffmpeg"),
            Check::Engine => w.t("check-engine"),
            Check::Bot => w.t("check-bot"),
        };
        match outcome {
            Some(Outcome::Passed(detail)) => {
                let detail = match which {
                    Check::Engine => format!("{}, {}", detail, w.t("same-build")),
                    Check::Folder if self.bot_only => detail.clone(),
                    Check::Folder => {
                        let live = self.live_sources();
                        let maps: u64 = live.iter().filter_map(|s| s.maps).sum();
                        let replays: u64 = live.iter().map(|s| s.replay_count).sum();
                        format!("{} · {}", w.count("maps-label", maps), w.count("replays-label", replays))
                    }
                    _ => detail.clone(),
                };
                Line::new(Mood::Done, name).detail(detail)
            }
            Some(Outcome::Failed(detail)) => {
                let line = Line::new(Mood::Failed, name);
                match which {
                    Check::Ffmpeg => line
                        .detail(w.t("not-installed"))
                        .note(if self.bot_only { w.t("ffmpeg-why-only") } else { w.t("ffmpeg-why") }, Some(w.t("where-to-get"))),
                    Check::Bot if detail.is_empty() => line.detail(w.t("not-linked")),
                    _ => line.detail(if detail.is_empty() { w.t("no-answer") } else { detail.clone() }),
                }
            }
            None if reached => Line::new(Mood::Now, name).detail(match which {
                Check::Engine | Check::Bot => w.t("asking-bot"),
                _ => String::new(),
            }),
            None => Line::new(Mood::Todo, name),
        }
    }

    fn checks_card(&self) -> Element<'_, Message> {
        let w = &self.words;
        let mut lines = Vec::new();
        let mut reached = true;
        for (which, outcome) in &self.checks {
            lines.push(self.check_line(*which, outcome, reached));
            if outcome.is_none() {
                reached = false;
            }
        }
        let failed = self.checks_failed();
        let finished = self.all_checked();
        let ledger = ui::ledger(&lines, Some(Message::WhereToGet));
        if !finished {
            return ui::card(ledger, None);
        }
        let hard = self.bot_only && (failed.contains(&Check::Ffmpeg) || failed.contains(&Check::Bot));
        let buttons = if failed.is_empty() {
            row![ui::grow(), ui::primary(w.t("open-dossier"), Some(Message::Finish))]
        } else if hard {
            row![ui::grow(), ui::primary(w.t("check-again"), Some(Message::CheckAgain))]
        } else {
            row![
                ui::quiet(w.t("check-again"), Some(Message::CheckAgain)),
                ui::grow(),
                ui::primary(w.t("continue-anyway"), Some(Message::Finish)),
            ]
        };
        ui::card(ledger, Some(buttons.spacing(8).into()))
    }
}

pub fn qr_for(link: &str) -> Option<ui::Qr> {
    let code = qrcode::QrCode::with_error_correction_level(link.as_bytes(), qrcode::EcLevel::H).ok()?;
    let n = code.width();
    let colors = code.to_colors();
    let cells = (0..n)
        .map(|y| (0..n).map(|x| colors[y * n + x] == qrcode::Color::Dark).collect())
        .collect();
    Some(ui::Qr::new(cells))
}

impl FirstRun {
    pub fn staged(step: Step, lang: Lang, sources: Vec<Source>, bot_only: bool, pairing: Pairing, checks: Vec<(Check, Option<Outcome>)>) -> FirstRun {
        let mut settings = Settings::default();
        settings.lang = lang;
        settings.device = "MacBook Pro".to_owned();
        let qr = match &pairing {
            Pairing::Waiting { link, .. } => qr_for(link),
            _ => None,
        };
        FirstRun {
            words: Words::new(lang),
            settings,
            step,
            looked: true,
            sources,
            bot_only,
            pairing,
            qr,
            checks,
        }
    }
}
