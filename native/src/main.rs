use dossier_native::{gallery, settings, App, WINDOW};
use iced::{window, Size};

fn main() -> iced::Result {
    let args: Vec<String> = std::env::args().collect();
    if let Some(at) = args.iter().position(|a| a == "--gallery") {
        let dir = args.get(at + 1).cloned().unwrap_or_else(|| "gallery".to_owned());
        match gallery::write(std::path::Path::new(&dir)) {
            Ok(n) => println!("{n} frames in {dir}"),
            Err(why) => {
                eprintln!("{why}");
                std::process::exit(1);
            }
        }
        return Ok(());
    }
    iced::application(App::boot, App::update, App::view)
        .title("Dossier")
        .settings(settings())
        .window(window::Settings {
            size: WINDOW,
            min_size: Some(Size::new(760.0, 560.0)),
            position: window::Position::Centered,
            ..window::Settings::default()
        })
        .theme(App::theme)
        .subscription(App::subscription)
        .run()
}
