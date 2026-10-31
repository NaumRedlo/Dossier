use dossier_native::{gallery, lang::Lang, main_screen::{Main, Message as M, Overlay}, settings_screen::Message as P, Message};
use iced::{mouse, Point, Size};
use iced_test::Simulator;
use std::{path::PathBuf, time::{Duration, Instant}};

fn staged(lang: Lang, room: bool, width: f32) -> Main {
    let states = gallery::main_states(lang);
    let (_, base) = states.iter().find(|(name, _)| name == "main-prefs").unwrap();
    let mut main = base.clone();
    main.width = width;
    main.height = 900.0;
    main.settings.tiles_app = vec!["skins".to_owned()];
    main.skins = ["/skins/Alpha", "/skins/Beta", "/skins/Gamma"].map(PathBuf::from).to_vec();
    main.skin_room = room;
    main.room_fade = iced::Animation::new(room);
    if room {
        main.overlay = Overlay::None;
        main.overlay_drawn = Overlay::None;
        main.overlay_fade = iced::Animation::new(false);
    }
    main
}

fn drag(ui: &mut Simulator<Message>, from: Point, to: Point) {
    let now = Instant::now();
    ui.point_at(from);
    ui.simulate([
        iced::Event::Window(iced::window::Event::RedrawRequested(now)),
        iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
    ]);
    ui.point_at(to);
    ui.simulate([
        iced::Event::Mouse(mouse::Event::CursorMoved { position: to }),
        iced::Event::Window(iced::window::Event::RedrawRequested(now + Duration::from_millis(100))),
        iced::Event::Window(iced::window::Event::RedrawRequested(now + Duration::from_millis(220))),
        iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
    ]);
}

#[test]
fn skin_cards_select_reorder_and_ask_before_deletion_in_both_panels() {
    let backdrop = dossier_native::ui::backdrop_handle();
    for lang in Lang::ALL {
        for room in [false, true] {
            for width in [980.0, 1440.0] {
                let main = staged(lang, room, width);
                let size = Size::new(main.width, main.height);
                let mut ui = Simulator::with_size(dossier_native::settings(), size, gallery::main_frame(&main, &backdrop));
                let alpha = ui.find("Alpha").unwrap().bounds();
                ui.point_at(alpha.center());
                ui.simulate(iced_test::simulator::click());
                let messages: Vec<_> = ui.into_messages().collect();
                assert!(messages.iter().any(|m| matches!(m, Message::Main(M::Prefs(P::Skin(Some(path)))) if path == &main.skins[0])), "{room}: {messages:?}");

                let mut ui = Simulator::with_size(dossier_native::settings(), size, gallery::main_frame(&main, &backdrop));
                let alpha = ui.find("Alpha").unwrap().bounds();
                let gamma = ui.find("Gamma").unwrap().bounds();
                drag(&mut ui, gamma.center(), Point::new(alpha.x + 1.0, alpha.center_y()));
                let messages: Vec<_> = ui.into_messages().collect();
                assert!(messages.iter().any(|m| matches!(m, Message::Main(M::Prefs(P::MoveSkin(path, Some(before)))) if path == &main.skins[2] && before == &main.skins[0])), "{lang:?} {width} {room}: {messages:?}");
                assert!(!messages.iter().any(|m| matches!(m, Message::Main(M::Prefs(P::Skin(_) | P::Moved(_, _))))), "a skin drag must not select a skin or drag the settings tile: {messages:?}");

                let mut ui = Simulator::with_size(dossier_native::settings(), size, gallery::main_frame(&main, &backdrop));
                let default = ui.find(main.words.t("own-skin-short")).unwrap().bounds();
                let alpha = ui.find("Alpha").unwrap().bounds();
                drag(&mut ui, default.center(), alpha.center());
                assert!(!ui.into_messages().any(|m| matches!(m, Message::Main(M::Prefs(P::MoveSkin(_, _))))), "the built-in skin stays first");

                let mut ui = Simulator::with_size(dossier_native::settings(), size, gallery::main_frame(&main, &backdrop));
                let delete = ui.find(iced::widget::Id::from(format!("skin-remove-{}", main.skins[0].display()))).unwrap().bounds();
                ui.point_at(Point::new(delete.x + delete.width - 5.0, delete.y + if room { delete.height / 2.0 } else { 8.0 }));
                ui.simulate(iced_test::simulator::click());
                let messages: Vec<_> = ui.into_messages().collect();
                assert!(messages.iter().any(|m| matches!(m, Message::Main(M::Prefs(P::AskDeleteSkin(path))) if path == &main.skins[0])), "{room}: {messages:?}");
                assert!(!messages.iter().any(|m| matches!(m, Message::Main(M::Prefs(P::DeleteSkin | P::Skin(_))))), "delete control must only ask: {messages:?}");
                if let Ok(dir) = std::env::var("DOSSIER_SKINS_REVIEW") {
                    if lang == Lang::Ru && width == 980.0 {
                        std::fs::create_dir_all(&dir).unwrap();
                        let mut ui = Simulator::with_size(dossier_native::settings(), size, gallery::main_frame(&main, &backdrop));
                        ui.snapshot(&dossier_native::theme::theme()).unwrap().matches_image(std::path::Path::new(&dir).join(if room { "skins-room" } else { "skins-settings" })).unwrap();
                    }
                }
            }
        }
    }
}

#[test]
fn skin_confirmation_keeps_the_path_and_blocks_clicks_while_deleting() {
    let backdrop = dossier_native::ui::backdrop_handle();
    for lang in Lang::ALL {
        let mut main = staged(lang, true, 980.0);
        let path = PathBuf::from("/Users/player/Library/Application Support/osu-wine/drive_c/users/player/AppData/Local/osu!/Skins/Alpha");
        main.skins[0] = path.clone();
        let _ = main.update(M::Prefs(P::AskDeleteSkin(path.clone())));
        assert_eq!(main.skin_delete.as_ref(), Some(&path));
        let size = Size::new(main.width, main.height);
        {
            let mut ui = Simulator::with_size(dossier_native::settings(), size, gallery::main_frame(&main, &backdrop));
            let label = main.words.with("delete-skin", &[("name", "Alpha".to_owned())]);
            assert!(ui.find(path.to_string_lossy().into_owned()).is_ok());
            let title = ui.find(label).unwrap().bounds();
            if lang == Lang::Ru {
                if let Ok(dir) = std::env::var("DOSSIER_SKINS_REVIEW") {
                    std::fs::create_dir_all(&dir).unwrap();
                    ui.snapshot(&dossier_native::theme::theme()).unwrap().matches_image(std::path::Path::new(&dir).join("skins-confirmation")).unwrap();
                }
            }
            ui.point_at(title.center());
            ui.simulate(iced_test::simulator::click());
            assert!(ui.into_messages().any(|m| matches!(m, Message::Main(M::Prefs(P::SkinDeleteTap)))));
        }
        {
            let mut ui = Simulator::with_size(dossier_native::settings(), size, gallery::main_frame(&main, &backdrop));
            let label = ui.find(iced::widget::Id::new("skin-delete-confirm")).unwrap().bounds();
            ui.point_at(label.center());
            ui.simulate(iced_test::simulator::click());
            assert!(ui.into_messages().any(|m| matches!(m, Message::Main(M::Prefs(P::DeleteSkin)))));
        }
        main.skin_deleting = true;
        let _ = main.update(M::Prefs(P::KeepSkin));
        assert_eq!(main.skin_delete.as_ref(), Some(&path));
        let mut ui = Simulator::with_size(dossier_native::settings(), size, gallery::main_frame(&main, &backdrop));
        let label = ui.find(main.words.t("skin-deleting")).unwrap().bounds();
        ui.point_at(label.center());
        ui.simulate(iced_test::simulator::click());
        let messages: Vec<_> = ui.into_messages().collect();
        assert!(!messages.iter().any(|m| matches!(m, Message::Main(M::Prefs(P::DeleteSkin | P::Skin(_) | P::AskDeleteSkin(_))))), "{messages:?}");
        main.skin_deleting = false;
        let _ = main.update(M::Prefs(P::KeepSkin));
        assert!(main.skin_delete.is_none());
    }
}
