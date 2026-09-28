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
