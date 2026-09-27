use dossier_native::{gallery, lang::Lang, main_screen::{Main, Message as M, Overlay}, settings_screen::{Message as P, Side}, community_screen::{Message as C, Section, Standing}};
use iced::{Size, Point};
use iced_test::Simulator;
use iced_test::Selector;

fn top_label(mut label: &str) -> impl Selector<Output = iced_test::selector::Text> + '_ {
    move |candidate: iced_test::selector::Candidate<'_>| {
        (candidate.bounds().y < 70.0).then(|| label.select(candidate)).flatten()
    }
}

fn staged(lang: Lang, name: &str, width: f32) -> Main {
    let (_, mut main) = gallery::main_states(lang).into_iter().find(|(name_here, _)| name_here == name).unwrap();
    main.width = width;
    main.height = 720.0;
    main
}

#[test]
fn navigation_and_section_tabs_fit_and_route_in_both_languages() {
    let backdrop = dossier_native::ui::backdrop_handle();
    for lang in Lang::ALL {
        for width in [980.0, 1060.0, 1200.0, 1440.0] {
            for name in ["main-community-profile", "main-prefs"] {
                let main = staged(lang, name, width);
                let parts: Vec<_> = if name == "main-prefs" {
                    vec![("app-side", M::Prefs(P::Side(Side::App))), ("bot-side", M::Prefs(P::Side(Side::Bot)))]
                } else {
                    vec![("community-profile", M::Community(C::Section(Section::Profile))), ("community-feed", M::Community(C::Section(Section::Feed))), ("community-people", M::Community(C::Section(Section::People))), ("community-boards", M::Community(C::Section(Section::Boards))), ("community-titles", M::Community(C::Section(Section::Titles)))]
                };
                let targets = parts.into_iter().chain([
                    ("replays", M::Show(Overlay::None)), ("videos", M::Show(Overlay::Videos)),
                    ("community", M::Show(Overlay::Community)), ("settings", M::Show(Overlay::Settings)),
                ]);
                let mut previous_right = 0.0;
                for (key, expected) in targets {
                    let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(width, 720.0), gallery::main_frame(&main, &backdrop));
                    let label = main.words.t(key);
                    let bounds = ui.find(top_label(label.as_str())).unwrap().bounds();
                    assert!(bounds.x >= previous_right && bounds.x + bounds.width <= width - 40.0, "{lang:?} {name} {width}: {key} {bounds:?}, previous right {previous_right}");
                    assert!(bounds.height < 30.0, "a tab wrapped: {lang:?} {width} {key} {bounds:?}");
                    previous_right = bounds.x + bounds.width;
                    ui.click(top_label(label.as_str())).unwrap();
                    let messages: Vec<_> = ui.into_messages().collect();
                    assert!(messages.iter().any(|message| matches!(message, dossier_native::Message::Main(actual) if format!("{actual:?}") == format!("{expected:?}"))), "{lang:?} {width} {key}: {messages:?}");
                }
            }
        }
    }
}

#[test]
fn settings_switches_request_the_opposite_value_after_tiles_are_reordered() {
    let backdrop = dossier_native::ui::backdrop_handle();
    for lang in Lang::ALL {
        for on in [false, true] {
            for (tile, controls) in [
                ("scene", vec![("live-replay", P::Scene(!on)), ("pause-unfocused", P::PauseUnfocused(!on)), ("auto-flip", P::AutoFlip(!on))]),
                ("play", vec![("hud", P::Hud(!on)), ("cursor-grows", P::CursorGrows(!on)), ("map-sounds", P::MapSounds(!on)), ("skin-sounds", P::SkinSounds(!on))]),
                ("look", vec![("scale-to-monitor", P::AutoScale(!on))]),
                ("sources", vec![("exported-only", P::ExportedOnly(!on))]),
                ("builds", vec![("quiet-updates", P::QuietUpdates(!on))]),
            ] {
                let mut main = staged(lang, "main-prefs", 980.0);
                main.settings.tiles_app = vec![tile.to_owned()];
                main.settings.live_scene = on;
                main.settings.pause_unfocused = on;
                main.settings.auto_flip = on;
                main.settings.hud = on;
                main.settings.cursor_grows = on;
                main.settings.map_sounds = on;
                main.settings.skin_sounds = on;
                main.settings.ui_scale = if on { 0 } else { 100 };
                main.settings.exported_only = on;
                main.settings.quiet_updates = on;
                for (key, expected) in controls {
                    let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(980.0, 720.0), gallery::main_frame(&main, &backdrop));
                    ui.click(main.words.t(key)).unwrap();
                    let messages: Vec<_> = ui.into_messages().collect();
                    assert!(messages.iter().any(|message| matches!(message, dossier_native::Message::Main(M::Prefs(actual)) if format!("{actual:?}") == format!("{expected:?}"))), "{lang:?} {key} {on}: {messages:?}");
                    assert!(!messages.iter().any(|message| matches!(message, dossier_native::Message::Main(M::Prefs(P::Moved(..))))), "clicking a switch must not drag its tile");
                }
            }
        }
    }
}

#[test]
fn ranking_switches_select_the_metric_and_ranking_mode() {
    let backdrop = dossier_native::ui::backdrop_handle();
    for lang in Lang::ALL {
        for width in [980.0, 1440.0] {
            let mut main = staged(lang, "main-community-boards", width);
            for standing in [Standing::Adaptive, Standing::General] {
                let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(width, 720.0), gallery::main_frame(&main, &backdrop));
                let label = main.words.t(if standing == Standing::Adaptive { "board-adaptive" } else { "board-general" });
                let bounds = ui.find(label.as_str()).unwrap().bounds();
                // Community content is displayed at 84% of its logical layout.
                let origin = dossier_native::theme::CONTROL_HEIGHT + 26.0;
                ui.point_at(Point::new(bounds.center_x() * 0.84, origin + (bounds.center_y() - origin) * 0.84));
                ui.simulate(iced_test::simulator::click());
                let messages: Vec<_> = ui.into_messages().collect();
                assert!(messages.iter().any(|message| matches!(message, dossier_native::Message::Main(M::Community(C::Standing(actual))) if *actual == standing)), "{lang:?} {width}: {messages:?}");
                drop(messages);
                let _ = main.update(M::Community(C::Standing(standing)));
                assert_eq!(main.community_standing, standing);
            }
            for board in dossier_native::community::Board::ALL {
                let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(width, 720.0), gallery::main_frame(&main, &backdrop));
                let bounds = ui.find(main.words.t(board.key())).unwrap().bounds();
                let origin = dossier_native::theme::CONTROL_HEIGHT + 26.0;
                ui.point_at(Point::new(bounds.center_x() * 0.84, origin + (bounds.center_y() - origin) * 0.84));
                ui.simulate(iced_test::simulator::click());
                assert!(ui.into_messages().any(|message| matches!(message, dossier_native::Message::Main(M::Community(C::Board(actual))) if actual == board)), "{lang:?} {width} {board:?}");
                let _ = main.update(M::Community(C::Board(board)));
                assert_eq!(main.community_board, board);
                assert_eq!(main.community_section, Section::Boards);
            }
        }
    }
}

#[test]
fn language_source_and_chat_choices_route_to_the_exact_item() {
    let backdrop = dossier_native::ui::backdrop_handle();
    for lang in Lang::ALL {
        let mut main = staged(lang, "main-prefs", 980.0);
        main.settings.tiles_app = vec!["language".to_owned(), "sources".to_owned()];
        main.settings.sources = [dossier_native::sources::Kind::Stable, dossier_native::sources::Kind::Lazer].into_iter().enumerate().map(|(at, kind)| dossier_native::sources::Source {
            kind, root: std::path::PathBuf::from(format!("/switch-test/game-{at}")), songs: None, skins: None, replays: None,
            maps: None, skin_count: 0, replay_count: 0, scores: 0, on: true,
        }).collect();
        for (label, selected) in [("Русский", Lang::Ru), ("English", Lang::En)] {
            let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(980.0, 720.0), gallery::main_frame(&main, &backdrop));
            ui.click(label).unwrap();
            assert!(ui.into_messages().any(|message| matches!(message, dossier_native::Message::Main(M::Prefs(P::PickLang(actual))) if actual == selected)));
        }
        for on in [false, true] {
            for at in 0..main.settings.sources.len() {
                main.settings.sources[at].on = on;
                let label = main.settings.sources[at].shown();
                let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(980.0, 720.0), gallery::main_frame(&main, &backdrop));
                ui.click(label).unwrap();
                assert!(ui.into_messages().any(|message| matches!(message, dossier_native::Message::Main(M::Prefs(P::Source(index, value))) if index == at && value == !on)), "{lang:?} source {at} {on}");
            }
        }
        let mut main = staged(lang, "main-prefs-bot", 980.0);
        main.settings.tiles_bot = vec!["chats".to_owned()];
        assert!(!main.chats.is_empty(), "the fixture must exercise real chat choices");
        for chat in &main.chats {
            let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(980.0, 720.0), gallery::main_frame(&main, &backdrop));
            ui.click(chat.title.as_str()).unwrap();
            assert!(ui.into_messages().any(|message| matches!(message, dossier_native::Message::Main(M::Prefs(P::Chat(id))) if id == chat.id)), "{lang:?} chat {}", chat.id);
        }
    }
}
