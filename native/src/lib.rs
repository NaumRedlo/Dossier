pub mod board;
pub mod bot;
pub mod chronicle;
pub mod checks;
pub mod community;
pub mod community_screen;
pub mod dossier;
pub mod ffmpeg;
pub mod film;
pub mod first_run;
pub mod gallery;
pub mod glide;
pub mod glyphs;
pub mod icon;
pub mod lang;
pub mod library;
pub mod live;
pub mod main_screen;
pub mod maps;
pub mod news;
pub mod notices;
pub mod osu_profile;
pub mod player;
pub mod render;
pub mod scan;
pub mod settings;
pub mod settings_screen;
pub mod sources;
pub mod theme;
pub mod unfold;
pub mod updates;
pub mod ui;
pub mod videos;
pub mod worker;

use iced::widget::{image, stack};
use iced::{Element, Length, Size, Subscription, Task, Theme};

use first_run::FirstRun;
use lang::Words;
use main_screen::Main;
use settings::Settings;

pub const WINDOW: Size = Size::new(980.0, 720.0);
pub const MINIMUM: Size = Size::new(760.0, 560.0);

pub enum Screen {
    FirstRun(FirstRun),
    Main(Main),
}

pub struct App {
    pub screen: Screen,
    pub backdrop: image::Handle,
    viewport: Size,
    measured: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    FirstRun(first_run::Message),
    Main(main_screen::Message),
    Snap,
    Snapped(Option<iced::window::Id>),
    Shot(iced::window::Screenshot),
    Opened(Size),
    Measure,
    Monitor(Option<Size>),
    Viewport(Size),
}

pub struct Rehearsal {
    pub folder: Option<std::path::PathBuf>,
    pub snap_to: Option<std::path::PathBuf>,
    pub after: std::time::Duration,
    pub render: bool,
    pub get_map: bool,
    pub look: bool,
    pub step: bool,
    pub hover: Option<usize>,
    pub videos: bool,
    pub play: Option<usize>,
    pub menu: Option<String>,
    pub prefs: bool,
    pub skin_room: bool,
    pub ask: bool,
    pub pause: bool,
    pub leave: bool,
    pub swap: bool,
    pub community: bool,
    pub read: bool,
}

impl Rehearsal {
    pub fn from_args(args: &[String]) -> Option<Rehearsal> {
        let at = args.iter().position(|a| a == "--open")?;
        let folder = args.get(at + 1).map(std::path::PathBuf::from);
        let snap_to = args.iter().position(|a| a == "--snap").and_then(|i| args.get(i + 1)).map(std::path::PathBuf::from);
        let after = args
            .iter()
            .position(|a| a == "--after")
            .and_then(|i| args.get(i + 1))
            .and_then(|s| s.parse::<u64>().ok())
            .map_or(std::time::Duration::from_millis(2500), std::time::Duration::from_millis);
        let render = args.iter().any(|a| a == "--render-first");
        let get_map = args.iter().any(|a| a == "--get-map-first");
        let look = args.iter().any(|a| a == "--look-first");
        let step = args.iter().any(|a| a == "--step-first");
        let hover = args.iter().position(|a| a == "--hover").and_then(|i| args.get(i + 1)).and_then(|s| s.parse().ok());
        let videos = args.iter().any(|a| a == "--videos-first");
        let play = args.iter().position(|a| a == "--play").and_then(|i| args.get(i + 1)).and_then(|s| s.parse().ok());
        let menu = args.iter().position(|a| a == "--menu").and_then(|i| args.get(i + 1)).cloned();
        let prefs = args.iter().any(|a| a == "--prefs");
        let skin_room = args.iter().any(|a| a == "--skin-room");
        let ask = args.iter().any(|a| a == "--ask");
        let pause = args.iter().any(|a| a == "--pause");
        let leave = args.iter().any(|a| a == "--leave");
        let swap = args.iter().any(|a| a == "--swap");
        let community = args.iter().any(|a| a == "--community");
        let read = args.iter().any(|a| a == "--read");
        Some(Rehearsal { folder, snap_to, after, render, get_map, look, step, hover, videos, play, menu, prefs, skin_room, ask, pause, leave, swap, community, read })
    }
}

pub static REHEARSAL: std::sync::OnceLock<Rehearsal> = std::sync::OnceLock::new();

impl App {
    pub fn boot() -> (App, Task<Message>) {
        let backdrop = ui::backdrop_handle();
        if let Some(rehearsal) = REHEARSAL.get() {
            let mut said = Settings::load();
            if let Some(source) = rehearsal.folder.as_deref().and_then(sources::folder_at) {
                said.sources = vec![source];
            }
            let (main, task) = Main::new(Words::new(said.lang), said);
            let snap = match &rehearsal.snap_to {
                Some(_) => {
                    let after = rehearsal.after;
                    Task::perform(async move { tokio_sleep(after).await }, |_| Message::Snap)
                }
                None => Task::none(),
            };
            let press = if rehearsal.render {
                Task::perform(async { tokio_sleep(std::time::Duration::from_millis(1500)).await }, |_| Message::Main(main_screen::Message::Render))
            } else if rehearsal.get_map {
                Task::perform(async { tokio_sleep(std::time::Duration::from_millis(1500)).await }, |_| Message::Main(main_screen::Message::GetMap))
            } else if rehearsal.look {
                Task::perform(async { tokio_sleep(std::time::Duration::from_millis(1500)).await }, |_| Message::Main(main_screen::Message::Look))
            } else if rehearsal.step {
                Task::perform(async { tokio_sleep(std::time::Duration::from_millis(2500)).await }, |_| Message::Main(main_screen::Message::Step(1)))
            } else if let Some(at) = rehearsal.hover {
                Task::perform(async { tokio_sleep(std::time::Duration::from_millis(2500)).await }, move |_| Message::Main(main_screen::Message::HoverStaged(at)))
            } else if let Some(at) = rehearsal.play {
                let asking = rehearsal.ask;
                let open = Task::perform(async { tokio_sleep(std::time::Duration::from_millis(1500)).await }, |_| Message::Main(main_screen::Message::Show(main_screen::Overlay::Videos)))
                    .chain(Task::perform(async { tokio_sleep(std::time::Duration::from_millis(1200)).await }, move |_| Message::Main(main_screen::Message::OpenVideo(at))));
                if asking {
                    open.chain(Task::perform(async { tokio_sleep(std::time::Duration::from_millis(2500)).await }, |_| Message::Main(main_screen::Message::AskDelete)))
                } else if rehearsal.pause {
                    open.chain(Task::perform(async { tokio_sleep(std::time::Duration::from_millis(2500)).await }, |_| Message::Main(main_screen::Message::PlayerToggle)))
                } else {
                    open
                }
            } else if false {
                let at = 0usize;
                Task::perform(async { tokio_sleep(std::time::Duration::from_millis(1500)).await }, |_| Message::Main(main_screen::Message::Show(main_screen::Overlay::Videos)))
                    .chain(Task::perform(async { tokio_sleep(std::time::Duration::from_millis(1200)).await }, move |_| Message::Main(main_screen::Message::OpenVideo(at))))
            } else if let Some(tab) = rehearsal.menu.clone() {
                let tab = match tab.as_str() {
                    "feed" => main_screen::Tab::Feed,
                    "stats" => main_screen::Tab::Stats,
                    _ => main_screen::Tab::Account,
                };
                Task::perform(async { tokio_sleep(std::time::Duration::from_millis(1500)).await }, move |_| Message::Main(main_screen::Message::MenuTab(tab)))
            } else if rehearsal.swap {
                Task::perform(async { tokio_sleep(std::time::Duration::from_millis(1200)).await }, |_| Message::Main(main_screen::Message::Show(main_screen::Overlay::Settings)))
                    .chain(Task::perform(async { tokio_sleep(std::time::Duration::from_millis(1200)).await }, |_| Message::Main(main_screen::Message::Show(main_screen::Overlay::Videos))))
            } else if rehearsal.leave {
                Task::perform(async { tokio_sleep(std::time::Duration::from_millis(1200)).await }, |_| Message::Main(main_screen::Message::Show(main_screen::Overlay::Settings)))
                    .chain(Task::perform(async { tokio_sleep(std::time::Duration::from_millis(1200)).await }, |_| Message::Main(main_screen::Message::Show(main_screen::Overlay::None))))
            } else if rehearsal.skin_room {
                Task::perform(async { tokio_sleep(std::time::Duration::from_millis(1200)).await }, |_| Message::Main(main_screen::Message::Show(main_screen::Overlay::Settings)))
                    .chain(Task::perform(async { tokio_sleep(std::time::Duration::from_millis(1600)).await }, |_| Message::Main(main_screen::Message::ShowSkins(true))))
            } else if rehearsal.read {
                Task::perform(async { tokio_sleep(std::time::Duration::from_millis(1500)).await }, |_| Message::Main(main_screen::Message::Show(main_screen::Overlay::Community)))
                    .chain(Task::perform(async { tokio_sleep(std::time::Duration::from_millis(1500)).await }, |_| Message::Main(main_screen::Message::ReadFirst)))
            } else if rehearsal.community {
                Task::perform(async { tokio_sleep(std::time::Duration::from_millis(1500)).await }, |_| Message::Main(main_screen::Message::Show(main_screen::Overlay::Community)))
            } else if rehearsal.prefs {
                Task::perform(async { tokio_sleep(std::time::Duration::from_millis(1500)).await }, |_| Message::Main(main_screen::Message::Show(main_screen::Overlay::Settings)))
            } else if rehearsal.videos {
                Task::perform(async { tokio_sleep(std::time::Duration::from_millis(1500)).await }, |_| Message::Main(main_screen::Message::Show(main_screen::Overlay::Videos)))
            } else {
                Task::none()
            };
            return (App { screen: Screen::Main(main), backdrop, viewport: WINDOW, measured: false }, Task::batch([task.map(Message::Main), snap, press]));
        }
        if settings::first_run() {
            let (flow, task) = FirstRun::new();
            (App { screen: Screen::FirstRun(flow), backdrop, viewport: WINDOW, measured: false }, task.map(Message::FirstRun))
        } else {
            let said = Settings::load();
            let (mut main, task) = Main::new(Words::new(said.lang), said);
            let launched = main.launched();
            (App { screen: Screen::Main(main), backdrop, viewport: WINDOW, measured: false }, Task::batch([task, launched]).map(Message::Main))
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::FirstRun(inner) => {
                let Screen::FirstRun(flow) = &mut self.screen else {
                    return Task::none();
                };
                let (task, done) = flow.update(inner);
                if let first_run::Done::Finished(said) = done {
                    let _ = said.save();
                    let (mut main, task) = Main::new(Words::new(said.lang), said);
                    let launched = main.launched();
                    self.screen = Screen::Main(main);
                    return Task::batch([task, launched]).map(Message::Main);
                }
                task.map(Message::FirstRun)
            }
            Message::Main(inner) => {
                let Screen::Main(main) = &mut self.screen else {
                    return Task::none();
                };
                main.update(inner).map(Message::Main)
            }
            Message::Opened(size) => {
                self.viewport = size;
                self.update(Message::Measure)
            }
            Message::Measure => iced::window::oldest().and_then(iced::window::monitor_size).map(Message::Monitor),
            Message::Viewport(size) => {
                self.viewport = size;
                Task::none()
            }
            Message::Monitor(None) => Task::none(),
            Message::Monitor(Some(monitor)) => {
                let now = self.scale_factor();
                let auto = ui::auto_scale_for(monitor.height * now);
                let first = !self.measured;
                self.measured = true;
                if auto == ui::auto_scale() && !first {
                    return Task::none();
                }
                ui::set_auto_scale(auto);
                let after = self.scale_factor();
                let room = Size::new(monitor.width * now / after * 0.94, monitor.height * now / after * 0.9);
                let least = Size::new(WINDOW.width.min(room.width), WINDOW.height.min(room.height));
                let fit = if first { Some(least) } else { ui::refit(self.viewport, now, after, least) };
                iced::window::oldest().and_then(move |id| {
                    let floor = iced::window::set_min_size(id, Some(MINIMUM));
                    match fit {
                        Some(size) => floor.chain(iced::window::resize(id, size)),
                        None => floor,
                    }
                })
            }
            Message::Snap => iced::window::oldest().map(Message::Snapped),
            Message::Snapped(Some(id)) => iced::window::screenshot(id).map(Message::Shot),
            Message::Snapped(None) => iced::exit(),
            Message::Shot(shot) => {
                if let Some(to) = REHEARSAL.get().and_then(|r| r.snap_to.clone()) {
                    let _ = ::image::save_buffer(&to, &shot.rgba, shot.size.width, shot.size.height, ::image::ColorType::Rgba8);
                }
                iced::exit()
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let front: Element<'_, Message> = match &self.screen {
            Screen::FirstRun(flow) => flow.view().map(Message::FirstRun),
            Screen::Main(main) => main.view().map(Message::Main),
        };
        stack![ui::backdrop(&self.backdrop), front]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let screen = match &self.screen {
            Screen::FirstRun(flow) => flow.subscription().map(Message::FirstRun),
            Screen::Main(main) => main.subscription().map(Message::Main),
        };
        let window = iced::event::listen_with(|event, _, _| match event {
            iced::Event::Window(iced::window::Event::Opened { size, .. }) => Some(Message::Opened(size)),
            iced::Event::Window(iced::window::Event::Moved(_)) => Some(Message::Measure),
            iced::Event::Window(iced::window::Event::Resized(size)) => Some(Message::Viewport(size)),
            _ => None,
        });
        Subscription::batch([screen, window])
    }

    pub fn scale_factor(&self) -> f32 {
        let chosen = match &self.screen {
            Screen::FirstRun(flow) => flow.settings.ui_scale,
            Screen::Main(main) => main.settings.ui_scale,
        };
        ui::scale_of(chosen)
    }

    pub fn theme(&self) -> Theme {
        theme::theme()
    }
}

pub fn settings() -> iced::Settings {
    iced::Settings {
        fonts: theme::FONTS.iter().map(|bytes| std::borrow::Cow::Borrowed(*bytes)).collect(),
        default_font: theme::SANS,
        default_text_size: theme::BODY.into(),
        antialiasing: true,
        ..iced::Settings::default()
    }
}


async fn tokio_sleep(after: std::time::Duration) {
    let (sender, receiver) = iced::futures::channel::oneshot::channel::<()>();
    std::thread::spawn(move || {
        std::thread::sleep(after);
        let _ = sender.send(());
    });
    let _ = receiver.await;
}
