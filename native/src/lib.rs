pub mod bot;
pub mod checks;
pub mod ffmpeg;
pub mod first_run;
pub mod gallery;
pub mod lang;
pub mod library;
pub mod live;
pub mod main_screen;
pub mod maps;
pub mod notices;
pub mod player;
pub mod render;
pub mod scan;
pub mod settings;
pub mod settings_screen;
pub mod sources;
pub mod theme;
pub mod ui;
pub mod videos;

use iced::widget::{image, stack};
use iced::{Element, Length, Size, Subscription, Task, Theme};

use first_run::FirstRun;
use lang::Words;
use main_screen::Main;
use settings::Settings;

pub const WINDOW: Size = Size::new(980.0, 720.0);

pub enum Screen {
    FirstRun(FirstRun),
    Main(Main),
}

pub struct App {
    pub screen: Screen,
    pub backdrop: image::Handle,
}

#[derive(Debug, Clone)]
pub enum Message {
    FirstRun(first_run::Message),
    Main(main_screen::Message),
    Snap,
    Snapped(Option<iced::window::Id>),
    Shot(iced::window::Screenshot),
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
    pub leave: bool,
    pub swap: bool,
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
        let leave = args.iter().any(|a| a == "--leave");
        let swap = args.iter().any(|a| a == "--swap");
        Some(Rehearsal { folder, snap_to, after, render, get_map, look, step, hover, videos, play, menu, prefs, leave, swap })
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
            } else if rehearsal.prefs {
                Task::perform(async { tokio_sleep(std::time::Duration::from_millis(1500)).await }, |_| Message::Main(main_screen::Message::Show(main_screen::Overlay::Settings)))
            } else if rehearsal.videos {
                Task::perform(async { tokio_sleep(std::time::Duration::from_millis(1500)).await }, |_| Message::Main(main_screen::Message::Show(main_screen::Overlay::Videos)))
            } else {
                Task::none()
            };
            return (App { screen: Screen::Main(main), backdrop }, Task::batch([task.map(Message::Main), snap, press]));
        }
        if settings::first_run() {
            let (flow, task) = FirstRun::new();
            (App { screen: Screen::FirstRun(flow), backdrop }, task.map(Message::FirstRun))
        } else {
            let said = Settings::load();
            let (main, task) = Main::new(Words::new(said.lang), said);
            (App { screen: Screen::Main(main), backdrop }, task.map(Message::Main))
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
                    let (main, task) = Main::new(Words::new(said.lang), said);
                    self.screen = Screen::Main(main);
                    return task.map(Message::Main);
                }
                task.map(Message::FirstRun)
            }
            Message::Main(inner) => {
                let Screen::Main(main) = &mut self.screen else {
                    return Task::none();
                };
                main.update(inner).map(Message::Main)
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
        match &self.screen {
            Screen::FirstRun(flow) => flow.subscription().map(Message::FirstRun),
            Screen::Main(main) => main.subscription().map(Message::Main),
        }
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
