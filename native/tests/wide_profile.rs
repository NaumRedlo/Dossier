use dossier_native::{community_screen::Section, gallery, lang::Lang};
use iced::Size;
use iced_test::Simulator;

#[test]
fn profile_columns_keep_a_readable_width_on_large_displays() {
    std::env::set_var("ICED_TEST_BACKEND", "tiny-skia");
    let (_, mut main) = gallery::main_states(Lang::Ru).into_iter().find(|(name, _)| name == "main-community-profile").unwrap();
    main.width = 3024.0;
    main.height = 1600.0;
    let backdrop = dossier_native::ui::backdrop_handle();
    let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(main.width, main.height), gallery::main_frame(&main, &backdrop));
    let best = ui.find("ЛУЧШИЕ ИГРЫ").unwrap().bounds();
    let activity = ui.find("АКТИВНОСТЬ").unwrap().bounds();
    assert!(best.x > 500.0, "the profile must stay centered: {}", best.x);
    assert!(activity.x - best.x < 1200.0, "the middle column must not stretch across the screen: {}", activity.x - best.x);
}

#[test]
fn title_actions_only_appear_in_the_titles_tab() {
    std::env::set_var("ICED_TEST_BACKEND", "tiny-skia");
    let backdrop = dossier_native::ui::backdrop_handle();
    for section in [Section::People, Section::Boards] {
        let state = if section == Section::People { "main-community-people" } else { "main-community-boards" };
        let (_, mut main) = gallery::main_states(Lang::Ru).into_iter().find(|(name, _)| name == state).unwrap();
        main.community_person = main.community.as_ref().and_then(|catalog| catalog.people.iter().position(|person| person.you));
        main.person_fade = iced::Animation::new(true);
        let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(980.0, 720.0), gallery::main_frame(&main, &backdrop));
        assert!(ui.find("Надеть").is_err() && ui.find("Снять").is_err(), "{section:?} dossier must not change titles");
    }
    let (_, main) = gallery::main_states(Lang::Ru).into_iter().find(|(name, _)| name == "main-community-titles").unwrap();
    let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(980.0, 720.0), gallery::main_frame(&main, &backdrop));
    assert!(ui.find("Надеть").is_ok() || ui.find("Снять").is_ok());
}

#[test]
fn a_top_play_and_a_title_in_the_profile_open_their_sheets_where_they_are() {
    use dossier_native::community_screen::{Message as C, Reading};
    use dossier_native::main_screen::Message as M;
    std::env::set_var("ICED_TEST_BACKEND", "tiny-skia");
    let (_, mut main) = gallery::main_states(Lang::Ru).into_iter().find(|(name, _)| name == "main-community-profile").unwrap();
    main.width = 1280.0;
    main.height = 1500.0;
    let backdrop = dossier_native::ui::backdrop_handle();
    let side = dossier_native::sidebar::NARROW;
    let origin = dossier_native::theme::CONTROL_HEIGHT + 26.0;
    let pressed = |label: &str| {
        let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(main.width, main.height), gallery::main_frame(&main, &backdrop));
        let bounds = ui.find(label).unwrap_or_else(|_| panic!("no {label} in the profile")).bounds();
        ui.point_at(iced::Point::new(side + (bounds.center_x() - side) * 0.84, origin + (bounds.center_y() - origin) * 0.84));
        ui.simulate(iced_test::simulator::click());
        ui.into_messages().collect::<Vec<_>>()
    };
    let result = pressed("Результат");
    assert!(result.iter().any(|message| matches!(message, dossier_native::Message::Main(M::Community(C::Read(Reading::Score(scored)))) if scored.name == "NaumRedlo" && scored.play.pp > 400.0)), "{result:?}");
    let holders = pressed("Обладатели");
    assert!(holders.iter().any(|message| matches!(message, dossier_native::Message::Main(M::Community(C::Read(Reading::Title { code, who: Some(_) }))) if code == "wysi")), "{holders:?}");

    let mut opened = main.clone();
    let _ = opened.update(M::Community(C::Read(Reading::Title { code: "wysi".into(), who: None })));
    assert_eq!(opened.community_section, Section::Profile);
    opened.read_fade = iced::Animation::new(true);
    let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(main.width, main.height), gallery::main_frame(&opened, &backdrop));
    assert!(ui.find("ОБЛАДАТЕЛИ В БЕСЕДЕ").is_ok() && ui.find("Снять").is_ok(), "the title sheet opens over the profile with its wear button");
}

#[test]
fn a_sheet_opened_from_a_dossier_lies_over_it_and_another_dossier_closes_it() {
    use dossier_native::community_screen::{Message as C, Reading};
    use dossier_native::main_screen::Message as M;
    std::env::set_var("ICED_TEST_BACKEND", "tiny-skia");
    let backdrop = dossier_native::ui::backdrop_handle();
    let (_, mut main) = gallery::main_states(Lang::Ru).into_iter().find(|(name, _)| name == "main-community-person").unwrap();
    assert!(main.community_person.is_some() && !main.read_over_person);
    let _ = main.update(M::Community(C::Read(Reading::Title { code: "wysi".into(), who: None })));
    assert!(main.read_over_person && main.read_fade.value());
    main.read_fade = iced::Animation::new(true);
    {
        let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(980.0, 720.0), gallery::main_frame(&main, &backdrop));
        assert!(ui.find("ОБЛАДАТЕЛИ В БЕСЕДЕ").is_ok(), "the sheet is not drawn over the dossier");
    }
    let _ = main.update(M::Community(C::Person(Some(0))));
    assert!(!main.read_fade.value(), "the sheet stayed over the dossier it opened");

    let (_, mut feed) = gallery::main_states(Lang::Ru).into_iter().find(|(name, _)| name == "main-community-title").unwrap();
    let _ = feed.update(M::Community(C::Read(Reading::Title { code: "wysi".into(), who: None })));
    assert!(!feed.read_over_person);
    let _ = feed.update(M::Community(C::Person(Some(0))));
    assert!(feed.read_fade.value(), "a dossier opened from a sheet lies over it, the sheet stays");
}
