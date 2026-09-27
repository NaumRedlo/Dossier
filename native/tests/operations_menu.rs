use dossier_native::{gallery, lang::Lang, main_screen::{Message, Overlay, Tab}};
use iced::{Animation, Size};
use iced_test::Simulator;

#[test]
fn every_catalogue_keeps_the_operations_tabs_aligned_and_clickable() {
    let backdrop = dossier_native::ui::backdrop_handle();
    let (_, base) = gallery::main_states(Lang::Ru).into_iter().find(|(name, _)| name == "main-menu-stats").unwrap();
    for (label, overlay) in [("journal", Overlay::None), ("videos", Overlay::Videos), ("community", Overlay::Community), ("settings", Overlay::Settings)] {
        let mut main = base.clone();
        main.overlay = overlay;
        main.overlay_drawn = overlay;
        main.overlay_fade = Animation::new(true);
        main.menu = Some(Tab::Stats);
        main.menu_open = Animation::new(true);
        let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(980.0, 720.0), gallery::main_frame(&main, &backdrop));
        let account = ui.find("Аккаунт").unwrap().bounds();
        let feed = ui.find("Лента").unwrap().bounds();
        let stats = ui.find("Статистика").unwrap().bounds();
        assert!((account.center_y() - stats.center_y()).abs() < 0.5, "{label}");
        assert!((feed.center_x() - account.center_x() - (stats.center_x() - feed.center_x())).abs() < 0.5, "{label}");
        assert!(ui.find("Карт в библиотеке").is_ok(), "{label}");
        if let Ok(dir) = std::env::var("DOSSIER_MENU_REVIEW") {
            let stem = std::path::Path::new(&dir).join(label);
            std::fs::create_dir_all(&dir).unwrap();
            ui.snapshot(&dossier_native::theme::theme()).unwrap().matches_image(&stem).unwrap();
        }
        ui.click("Аккаунт").unwrap();
        assert!(ui.into_messages().any(|message| matches!(message, dossier_native::Message::Main(Message::MenuTab(Tab::Account)))), "{label}");
    }
    if let Ok(dir) = std::env::var("DOSSIER_MENU_REVIEW") {
        for name in ["main-menu-account", "main-worker", "main-prefs"] {
            let (_, mut main) = gallery::main_states(Lang::Ru).into_iter().find(|(state, _)| state == name).unwrap();
            if name == "main-prefs" {
                main.settings.ui_scale = 120;
            }
            let stem = std::path::Path::new(&dir).join(name);
            gallery::snapshot_main(&main, Size::new(980.0, 720.0)).unwrap().matches_image(&stem).unwrap();
        }
    }
}
