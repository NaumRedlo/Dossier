use dossier_native::first_run::Message;
use dossier_native::gallery;
use dossier_native::lang::Lang;
use iced::Point;
use iced_test::Simulator;

fn flow(name: &str) -> dossier_native::first_run::FirstRun {
    gallery::states(Lang::En)
        .into_iter()
        .find(|(n, _)| n == name)
        .map(|(_, f)| f)
        .expect("a staged state")
}

#[test]
fn a_click_on_the_edge_of_a_button_presses_it() {
    let staged = flow("folder-stable");
    let backdrop = dossier_native::ui::backdrop_handle();
    let mut ui = Simulator::with_size(dossier_native::settings(), iced::Size::new(980.0, 720.0), gallery::frame(&staged, &backdrop));
    let target = ui.find("Use this").expect("the button's label");
    let bounds = target.bounds();
    ui.point_at(Point::new(bounds.x - 8.0, bounds.y + bounds.height / 2.0));
    let _ = ui.simulate(iced_test::simulator::click());
    let messages: Vec<_> = ui.into_messages().collect();
    assert!(
        messages.iter().any(|m| matches!(m, dossier_native::Message::FirstRun(Message::Continue))),
        "a click beside the label, inside the button, produced {messages:?}"
    );
}

#[test]
fn a_click_on_the_label_presses_it() {
    let staged = flow("folder-stable");
    let backdrop = dossier_native::ui::backdrop_handle();
    let mut ui = Simulator::with_size(dossier_native::settings(), iced::Size::new(980.0, 720.0), gallery::frame(&staged, &backdrop));
    let _ = ui.click("Use this").expect("clicked");
    let messages: Vec<_> = ui.into_messages().collect();
    assert!(messages.iter().any(|m| matches!(m, dossier_native::Message::FirstRun(Message::Continue))), "{messages:?}");
}

#[test]
fn a_missing_ffmpeg_offers_a_download_and_a_way_past() {
    let staged = flow("checks-ffmpeg-missing");
    let backdrop = dossier_native::ui::backdrop_handle();
    let mut ui = Simulator::with_size(dossier_native::settings(), iced::Size::new(980.0, 720.0), gallery::frame(&staged, &backdrop));
    assert!(ui.find("Where to get it").is_err(), "the link belongs to a failed download, not to a missing ffmpeg");
    let _ = ui.click("Download").expect("clicked");
    let messages: Vec<_> = ui.into_messages().collect();
    assert!(messages.iter().any(|m| matches!(m, dossier_native::Message::FirstRun(Message::Download))), "{messages:?}");

    let mut ui = Simulator::with_size(dossier_native::settings(), iced::Size::new(980.0, 720.0), gallery::frame(&staged, &backdrop));
    let _ = ui.click("Continue anyway").expect("clicked");
    let messages: Vec<_> = ui.into_messages().collect();
    assert!(messages.iter().any(|m| matches!(m, dossier_native::Message::FirstRun(Message::Finish))), "{messages:?}");
}

#[test]
fn while_ffmpeg_downloads_there_is_no_second_download_button() {
    let staged = flow("checks-ffmpeg-downloading");
    let backdrop = dossier_native::ui::backdrop_handle();
    let mut ui = Simulator::with_size(dossier_native::settings(), iced::Size::new(980.0, 720.0), gallery::frame(&staged, &backdrop));
    assert!(ui.find("Download").is_err());
    assert!(ui.find("Continue anyway").is_ok());
}

fn main_state(name: &str) -> dossier_native::main_screen::Main {
    gallery::main_states(Lang::En)
        .into_iter()
        .find(|(n, _)| n == name)
        .map(|(_, m)| m)
        .expect("a staged main screen")
}

#[test]
fn the_three_words_and_the_frames_answer_to_clicks() {
    use dossier_native::main_screen::{Message as M, Overlay};
    let staged = main_state("main-rest");
    let backdrop = dossier_native::ui::backdrop_handle();
    let mut ui = Simulator::with_size(dossier_native::settings(), iced::Size::new(980.0, 720.0), gallery::main_frame(&staged, &backdrop));
    let _ = ui.click("Settings").expect("clicked");
    let messages: Vec<_> = ui.into_messages().collect();
    assert!(messages.iter().any(|m| matches!(m, dossier_native::Message::Main(M::Show(Overlay::Settings)))), "{messages:?}");

    let mut ui = Simulator::with_size(dossier_native::settings(), iced::Size::new(980.0, 720.0), gallery::main_frame(&staged, &backdrop));
    ui.point_at(Point::new(40.0 + 88.0 + 6.0 + 44.0 + 22.0 + 8.0, 720.0 - 10.0 - 18.0 - 4.0 - 25.0));
    let _ = ui.simulate(iced_test::simulator::click());
    let messages: Vec<_> = ui.into_messages().collect();
    assert!(
        messages.iter().any(|m| matches!(m, dossier_native::Message::Main(M::Choose(_)) | dossier_native::Message::Main(M::Hover(_)))),
        "a click on the strip produced {messages:?}"
    );
}

#[test]
fn the_worker_is_a_part_of_settings_and_the_bar_turns_between_them() {
    use dossier_native::main_screen::Message as M;
    use dossier_native::settings_screen::{Message as P, Side};
    let staged = main_state("main-worker");
    let backdrop = dossier_native::ui::backdrop_handle();
    let mut ui = Simulator::with_size(dossier_native::settings(), iced::Size::new(980.0, 720.0), gallery::main_frame(&staged, &backdrop));
    assert!(ui.find("Take work from the bot").is_ok());
    let _ = ui.click("Application").expect("clicked");
    let messages: Vec<_> = ui.into_messages().collect();
    assert!(messages.iter().any(|m| matches!(m, dossier_native::Message::Main(M::Prefs(P::Side(Side::App))))), "{messages:?}");
}
