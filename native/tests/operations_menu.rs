use dossier_native::{billing, gallery, lang::Lang, main_screen::{Message, Overlay, Tab}};
use iced::{Animation, Size};
use iced_test::Simulator;

#[test]
fn every_catalogue_keeps_the_account_menu_whole_and_clickable() {
    let backdrop = dossier_native::ui::backdrop_handle();
    let (_, base) = gallery::main_states(Lang::Ru).into_iter().find(|(name, _)| name == "main-menu-account").unwrap();
    for (label, overlay) in [("journal", Overlay::None), ("videos", Overlay::Videos), ("community", Overlay::Community), ("settings", Overlay::Settings)] {
        let mut main = base.clone();
        main.overlay = overlay;
        main.overlay_drawn = overlay;
        main.overlay_fade = Animation::new(true);
        main.menu = Some(Tab::Head);
        main.menu_open = Animation::new(true);
        let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(980.0, 720.0), gallery::main_frame(&main, &backdrop));
        let name = ui.find("Naum Redlo").unwrap().bounds();
        let out = ui.find("Выйти").unwrap().bounds();
        let manage = ui.find("Управлять").unwrap().bounds();
        assert!(name.x < out.x && (out.x + out.width - manage.x - manage.width).abs() < 24.0, "{label}: the way out and the subscription button share the right edge");
        assert!(manage.y > out.y, "{label}: the subscription sits under the account in the same panel");
        for gone in ["Аккаунт", "Лента", "Статистика", "Карт в библиотеке", "Уведомления"] {
            assert!(ui.find(gone).is_err(), "{label}: {gone}");
        }
        if let Ok(dir) = std::env::var("DOSSIER_MENU_REVIEW") {
            let stem = std::path::Path::new(&dir).join(label);
            std::fs::create_dir_all(&dir).unwrap();
            ui.snapshot(&dossier_native::theme::theme()).unwrap().matches_image(&stem).unwrap();
        }
        ui.click("Управлять").unwrap();
        assert!(ui.into_messages().any(|message| matches!(message, dossier_native::Message::Main(Message::Billing(billing::Message::Open)))), "{label}");
    }
}
