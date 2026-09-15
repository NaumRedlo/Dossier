pub mod bot;
pub mod checks;
pub mod first_run;
pub mod gallery;
pub mod lang;
pub mod settings;
pub mod sources;
pub mod theme;
pub mod ui;

use iced::widget::{container, image, stack, text};
use iced::{Element, Length, Size, Subscription, Task, Theme};

use first_run::FirstRun;
use lang::Words;
use settings::Settings;

pub const WINDOW: Size = Size::new(980.0, 720.0);

pub enum Screen {
    FirstRun(FirstRun),
    Main(Words),
}

pub struct App {
    pub screen: Screen,
    pub backdrop: image::Handle,
}

#[derive(Debug, Clone)]
pub enum Message {
    FirstRun(first_run::Message),
}

impl App {
    pub fn boot() -> (App, Task<Message>) {
        let backdrop = ui::backdrop_handle();
        if settings::first_run() {
            let (flow, task) = FirstRun::new();
            (App { screen: Screen::FirstRun(flow), backdrop }, task.map(Message::FirstRun))
        } else {
            let said = Settings::load();
            (App { screen: Screen::Main(Words::new(said.lang)), backdrop }, Task::none())
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
                    self.screen = Screen::Main(Words::new(said.lang));
                    return Task::none();
                }
                task.map(Message::FirstRun)
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let front: Element<'_, Message> = match &self.screen {
            Screen::FirstRun(flow) => flow.view().map(Message::FirstRun),
            Screen::Main(words) => container(text(words.t("app-name")).font(theme::SANS_SEMI).size(theme::TITLE))
                .center(Length::Fill)
                .into(),
        };
        stack![ui::backdrop(&self.backdrop), front]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        match &self.screen {
            Screen::FirstRun(flow) => flow.subscription().map(Message::FirstRun),
            Screen::Main(_) => Subscription::none(),
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

