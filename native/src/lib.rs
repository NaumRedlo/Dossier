pub mod billing;
pub mod board;
pub mod bot;
pub mod chronicle;
pub mod checks;
pub mod client_sounds;
pub mod community;
pub mod compare;
pub mod community_screen;
pub mod crops;
pub mod donate;
pub mod frames;
pub mod dossier;
pub mod dossier_cache;
pub mod desktop;
pub mod ffmpeg;
pub mod film;
pub mod first_run;
pub mod gallery;
pub mod glide;
pub mod glyphs;
pub mod icon;
pub mod inbox;
pub mod lanes;
pub mod lang;
pub mod library;
pub mod live;
pub mod main_screen;
pub mod maps;
pub mod mixed;
pub mod modicons;
pub mod net;
pub mod news;
pub mod notices;
pub mod osu_profile;
pub mod player;
mod playback_wake;
pub mod pool_links;
pub mod pool_collections;
pub mod pool_files;
pub mod pool_card;
pub mod pool_share;
pub mod pools;
pub mod pools_screen;
pub mod render;
pub mod scan;
pub mod history;
pub mod scores;
pub mod settings;
pub mod settings_screen;
pub mod sheets;
pub mod sidebar;
pub mod sources;
pub mod theme;
pub mod tray;
pub mod unfold;
pub mod updates;
pub mod ui;
pub mod videos;
pub mod witness;
pub mod witness_delivery;
pub mod worker;

use iced::widget::{image, stack};
use iced::{Element, Length, Size, Subscription, Task, Theme};

use first_run::FirstRun;
use lang::Words;
use main_screen::Main;
use settings::Settings;

pub const WINDOW: Size = Size::new(980.0, 720.0);
pub const MINIMUM: Size = Size::new(760.0, 560.0);

pub fn minimum_window(chosen: u32) -> Size {
    ui::viewport_at(MINIMUM, ui::scale_of(chosen), 1.0)
}

pub fn refit_window<T: Send + 'static>(fit: Option<Size>, minimum: Size) -> Task<T> {
    iced::window::oldest().and_then(move |id| {
        iced::window::is_maximized(id).then(move |maximized| {
            if maximized {
                return Task::none();
            }
            iced::window::mode(id).then(move |mode| {
                if mode != iced::window::Mode::Windowed {
                    return Task::none();
                }
                let floor = iced::window::set_min_size(id, Some(minimum));
                match fit {
                    Some(size) => floor.chain(iced::window::resize(id, size)),
                    None => floor,
                }
            })
        })
    })
}

pub enum Screen {
    FirstRun(FirstRun),
    Main(Main),
}

pub struct App {
    pub screen: Screen,
    pub backdrop: image::Handle,
    viewport: Size,
    measured: bool,
    opened: bool,
    tray: Option<tray::Tray>,
    hidden: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    FirstRun(first_run::Message),
    Main(main_screen::Message),
    Snap,
    Snapped(Option<iced::window::Id>),
    Shot(iced::window::Screenshot),
    Opened(Size),
    CloseAsked(iced::window::Id),
    Tray(tray::Said),
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
    pub flood: Option<f64>,
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
        let flood = args.iter().position(|a| a == "--flood").and_then(|i| args.get(i + 1)).and_then(|s| s.parse::<f64>().ok()).filter(|hz| *hz > 0.0);
        Some(Rehearsal { folder, snap_to, after, render, get_map, look, step, hover, videos, play, menu, prefs, skin_room, ask, pause, leave, swap, community, read, flood })
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
            let (mut main, task) = Main::new(Words::new(said.lang), said);
            main.notices = notices::Queue::default();
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
                    "bell" | "feed" => main_screen::Tab::Bell,
                    _ => main_screen::Tab::Head,
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
            let flood = match rehearsal.flood {
                Some(hz) => ui::streamed(move |push| {
                    let began = std::time::Instant::now();
                    let mut n = 0u64;
                    loop {
                        n += 1;
                        if let Some(wait) = (began + std::time::Duration::from_secs_f64(n as f64 / hz)).checked_duration_since(std::time::Instant::now()) {
                            std::thread::sleep(wait);
                        }
                        if !push(Message::Main(main_screen::Message::PointerActivity(iced::Point::new(200.0 + (n % 400) as f32, 300.0)))) {
                            return;
                        }
                    }
                }),
                None => Task::none(),
            };
            return (App { screen: Screen::Main(main), backdrop, viewport: WINDOW, measured: false, opened: false, tray: None, hidden: false }, Task::batch([task.map(Message::Main), snap, press, flood]));
        }
        if settings::first_run() {
            let (flow, task) = FirstRun::new();
            (App { screen: Screen::FirstRun(flow), backdrop, viewport: WINDOW, measured: false, opened: false, tray: None, hidden: false }, task.map(Message::FirstRun))
        } else {
            let said = Settings::load();
            let (mut main, task) = Main::new(Words::new(said.lang), said);
            let launched = main.launched();
            (App { screen: Screen::Main(main), backdrop, viewport: WINDOW, measured: false, opened: false, tray: None, hidden: false }, Task::batch([task, launched]).map(Message::Main))
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        let task = self.update_inner(message);
        let viewport = ui::viewport_at(self.viewport, 1.0, self.scale_factor());
        if let Screen::Main(main) = &mut self.screen {
            main.width = viewport.width;
            main.height = viewport.height;
        }
        task
    }

    fn update_inner(&mut self, message: Message) -> Task<Message> {
        if !frames::on() {
            return self.handle(message);
        }
        let label = match (&message, &self.screen) {
            (Message::Main(inner), Screen::Main(main)) => {
                if let main_screen::Message::Tick(at) = inner {
                    frames::frame(*at, &frames::name(&main.overlay));
                }
                frames::name(inner)
            }
            (Message::FirstRun(inner), _) => frames::name(inner),
            _ => frames::name(&message),
        };
        let from = std::time::Instant::now();
        let task = self.handle(message);
        frames::slow("update", from, &label);
        task
    }

    fn quit(&mut self) -> Task<Message> {
        if let Screen::Main(main) = &mut self.screen {
            if !main.prepare_exit() {
                return self.handle(Message::Tray(tray::Said::Show));
            }
        }
        iced::exit()
    }

    fn handle(&mut self, message: Message) -> Task<Message> {
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
                    self.keep_tray();
                    return Task::batch([task, launched]).map(Message::Main);
                }
                task.map(Message::FirstRun)
            }
            Message::Main(inner) => {
                let Screen::Main(main) = &mut self.screen else {
                    return Task::none();
                };
                let task = main.update(inner).map(Message::Main);
                self.keep_tray();
                task
            }
            Message::Opened(size) => {
                self.viewport = size;
                self.opened = true;
                self.keep_tray();
                self.handle(Message::Measure)
            }
            Message::CloseAsked(id) => match (&mut self.screen, &self.tray) {
                (Screen::Main(main), Some(_)) => {
                    self.hidden = true;
                    main.set_hidden(true);
                    tray::dock(false);
                    let hidden = iced::window::set_mode(id, iced::window::Mode::Hidden);
                    if cfg!(target_os = "linux") {
                        hidden.chain(iced::window::minimize(id, true))
                    } else {
                        hidden
                    }
                }
                _ => self.quit(),
            },
            Message::Tray(tray::Said::Show) => {
                if let Screen::Main(main) = &mut self.screen {
                    main.set_hidden(false);
                }
                let was_hidden = std::mem::replace(&mut self.hidden, false);
                tray::dock(true);
                iced::window::oldest().and_then(move |id| {
                    let shown = match (was_hidden, cfg!(target_os = "linux")) {
                        (false, _) => Task::none(),
                        (true, false) => iced::window::set_mode(id, iced::window::Mode::Windowed),
                        (true, true) => iced::window::minimize(id, false).chain(iced::window::set_mode(id, iced::window::Mode::Windowed)),
                    };
                    shown.chain(iced::window::gain_focus(id))
                })
            }
            Message::Tray(tray::Said::Quit) => self.quit(),
            Message::Measure => iced::window::oldest().and_then(|id| Task::batch([
                iced::window::monitor_size(id).map(Message::Monitor),
                iced::window::size(id).map(Message::Viewport),
            ])),
            Message::Viewport(size) => {
                if size.width <= 0.0 || size.height <= 0.0 { return Task::none(); }
                self.viewport = size;
                Task::none()
            }
            Message::Monitor(None) => Task::none(),
            Message::Monitor(Some(monitor)) => {
                let auto = ui::auto_scale_for(monitor.height);
                let first = !self.measured;
                self.measured = true;
                if !first && auto == ui::auto_scale() { return Task::none(); }
                ui::set_auto_scale(auto);
                let room = Size::new(monitor.width * 0.94, monitor.height * 0.9);
                let minimum = minimum_window(self.chosen_scale());
                let fit = Size::new(self.viewport.width.max(minimum.width.min(room.width)).min(room.width), self.viewport.height.max(minimum.height.min(room.height)).min(room.height));
                refit_window((fit != self.viewport).then_some(fit), minimum)
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
        let from = std::time::Instant::now();
        let front: Element<'_, Message> = match &self.screen {
            Screen::FirstRun(flow) => flow.view().map(Message::FirstRun),
            Screen::Main(main) => main.view().map(Message::Main),
        };
        if frames::on() {
            let label = match &self.screen {
                Screen::Main(main) => frames::name(&main.overlay),
                Screen::FirstRun(_) => "FirstRun".to_owned(),
            };
            frames::slow("view", from, &label);
        }
        ui::scaled(stack![ui::backdrop(&self.backdrop), front]
            .width(Length::Fill)
            .height(Length::Fill), self.scale_factor()).into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let screen = match &self.screen {
            Screen::FirstRun(flow) => flow.subscription().map(Message::FirstRun),
            Screen::Main(main) => main.subscription().map(Message::Main),
        };
        let window = iced::event::listen_with(|event, _, _| match event {
            iced::Event::Window(iced::window::Event::Opened { size, .. }) => Some(Message::Opened(size)),
            iced::Event::Window(iced::window::Event::Moved(_) | iced::window::Event::Rescaled(_)) => Some(Message::Measure),
            iced::Event::Window(iced::window::Event::Resized(size)) => Some(Message::Viewport(size)),
            _ => None,
        });
        let closing = iced::window::close_requests().map(Message::CloseAsked);
        let tray = match self.tray {
            Some(_) => Subscription::run(tray::events).map(Message::Tray),
            None => Subscription::none(),
        };
        Subscription::batch([screen, window, closing, tray])
    }

    fn keep_tray(&mut self) {
        let Screen::Main(main) = &self.screen else {
            return;
        };
        let wanted = self.opened && main.settings.close_to_tray && REHEARSAL.get().is_none();
        match (&mut self.tray, wanted) {
            (Some(tray), true) => tray.speak(&main.words),
            (None, true) => self.tray = tray::Tray::new(&main.words),
            (Some(_), false) if !self.hidden => self.tray = None,
            _ => {}
        }
    }

    pub fn scale_factor(&self) -> f32 {
        ui::scale_of(self.chosen_scale())
    }

    fn chosen_scale(&self) -> u32 {
        match &self.screen {
            Screen::FirstRun(flow) => flow.settings.ui_scale,
            Screen::Main(main) => main.settings.ui_scale,
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

#[cfg(test)]
mod workspace_tests {
    use super::*;
    static SCALE_TEST: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn resizing_keeps_main_and_rendering_in_the_same_coordinate_space() {
        let _lock = SCALE_TEST.lock().unwrap();
        struct Restore(u32);
        impl Drop for Restore { fn drop(&mut self) { ui::set_auto_scale(self.0); } }
        let _restore = Restore(ui::auto_scale());
        ui::set_auto_scale(100);
        let main = Main::staged(Words::new(lang::Lang::En), Settings::default(), library::Library::default(), None);
        let mut app = App { screen: Screen::Main(main), backdrop: image::Handle::from_rgba(1, 1, vec![0, 0, 0, 255]), viewport: WINDOW, measured: false, opened: false, tray: None, hidden: false };
        let _ = app.update(Message::Monitor(Some(Size::new(1920.0, 1080.0))));
        for window in [Size::new(1512.0, 840.0), Size::new(1920.0, 1080.0), Size::new(2560.0, 1440.0), WINDOW] {
            for _ in 0..3 {
                let _ = app.update(Message::Viewport(window));
                let factor = app.scale_factor();
                assert_eq!(factor, 1.0);
                assert_eq!(minimum_window(0), MINIMUM);
                assert_eq!(app.viewport, window);
                let Screen::Main(main) = &app.screen else { panic!("main") };
                let viewport = ui::viewport_at(window, 1.0, factor);
                assert_eq!((main.width, main.height), (viewport.width, viewport.height));
            }
        }
        let _ = app.update(Message::Monitor(Some(Size::new(2560.0, 1440.0))));
        assert_eq!(app.scale_factor(), 1.2);
        let minimum = minimum_window(0);
        assert!((minimum.width - 912.0).abs() < 0.001);
        assert!((minimum.height - 672.0).abs() < 0.001);
        let Screen::Main(main) = &mut app.screen else { panic!("main") };
        main.settings.ui_scale = 125;
        let window = Size::new(1920.0, 1080.0);
        let _ = app.update(Message::Viewport(window));
        assert_eq!(app.scale_factor(), 1.25);
        assert_eq!(minimum_window(125), Size::new(950.0, 700.0));
        assert_eq!(app.viewport, window);
        let Screen::Main(main) = &app.screen else { panic!("main") };
        assert_eq!((main.width, main.height), (1536.0, 864.0));
        let _ = app.update(Message::Viewport(Size::ZERO));
        assert_eq!(app.viewport, window);
    }

    #[test]
    fn queued_window_events_keep_navigation_within_the_retina_window() {
        let _lock = SCALE_TEST.lock().unwrap();
        struct Restore(u32);
        impl Drop for Restore { fn drop(&mut self) { ui::set_auto_scale(self.0); } }
        let _restore = Restore(ui::auto_scale());
        ui::set_auto_scale(100);
        let said = Settings { close_to_tray: false, ..Settings::default() };
        let main = Main::staged(Words::new(lang::Lang::Ru), said, library::Library::default(), None);
        let size = Size::new(1512.0, 840.0);
        let mut app = App { screen: Screen::Main(main), backdrop: image::Handle::from_rgba(1, 1, vec![0, 0, 0, 255]), viewport: WINDOW, measured: false, opened: false, tray: None, hidden: false };
        for _ in 0..4 {
            let _ = app.update(Message::Opened(size));
            let _ = app.update(Message::Viewport(size));
            let _ = app.update(Message::Monitor(Some(Size::new(1512.0, 982.0))));
            assert_eq!(app.scale_factor(), 0.95);
            assert_eq!(app.viewport, size);
        }
        let mut screen = iced_test::Simulator::with_size(settings(), size, app.view());
        let bounds = screen.find("Настройки").unwrap().bounds();
        let drawn = bounds * iced::Transformation::scale(app.scale_factor());
        assert!(drawn.x + drawn.width <= size.width);
        assert!(drawn.y + drawn.height < 100.0);
        assert!(drawn.height < 35.0);
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
