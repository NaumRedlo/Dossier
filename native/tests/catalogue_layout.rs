use dossier_native::{gallery, lang::Lang};
use iced::{widget::{button, container, Space}, Event, Padding, Point, Size};
use iced_test::Simulator;

fn community_point(bounds: iced::Rectangle, width: f32) -> Point {
    let side = dossier_native::sidebar::NARROW;
    let origin_y = dossier_native::theme::CONTROL_HEIGHT + 26.0;
    Point::new(side + (bounds.center_x() - side) * 0.84, origin_y + (bounds.center_y() - origin_y) * 0.84)
}

fn content_middle(width: f32) -> f32 {
    let side = dossier_native::sidebar::NARROW;
    side + (width - side) / 2.0
}

#[test]
fn slider_values_cannot_paint_outside_their_control_on_the_first_frame() {
    for fraction in [0.0, 0.5, 1.0] {
        let control = container(dossier_native::ui::steps("Music".to_owned(), "100 %".to_owned(), fraction, fraction, 1.0, vec![], |_| ()))
            .width(240.0).id(iced::widget::Id::new("slider-boundary"));
        let root = container(control).padding(20).width(340.0).height(90.0).style(|_| container::Style {
            background: Some(iced::Background::Color(iced::Color::BLACK)), ..Default::default()
        });
        let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(340.0, 90.0), root);
        let bounds = ui.find(iced::widget::Id::new("slider-boundary")).unwrap().bounds();
        let dir = std::env::temp_dir().join(format!("dossier-slider-boundary-{}-{fraction}", std::process::id()));
        let file = gallery::write_snapshot(&ui.snapshot(&dossier_native::theme::theme()).unwrap(), &dir.join("slider")).unwrap();
        let pixels = image::open(&file).unwrap().to_rgba8();
        let scale = pixels.width() as f32 / 340.0;
        let background = *pixels.get_pixel(0, 0);
        for y in (bounds.y * scale) as u32..((bounds.y + bounds.height) * scale) as u32 {
            for x in ((bounds.x + bounds.width + 2.0) * scale) as u32..(330.0 * scale) as u32 {
                assert_eq!(*pixels.get_pixel(x, y), background, "slider value escaped on the first frame at {fraction}: {x}, {y}");
            }
        }
        std::fs::remove_file(file).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }
}

#[test]
fn resting_hides_main_controls_and_waking_cannot_click_through_them() {
    use dossier_native::main_screen::Message as MainMessage;
    let backdrop = dossier_native::ui::backdrop_handle();
    for lang in Lang::ALL {
        let (_, base) = gallery::main_states(lang).into_iter().find(|(name, _)| name == "main-rest").unwrap();
        for width in [980.0, 1440.0] {
            let mut main = base.clone();
            main.width = width;
            main.height = 720.0;
            let settings = main.words.t("settings");
            main.resting = iced::Animation::new(false).duration(std::time::Duration::from_millis(600)).go(true, main.now);
            main.now += std::time::Duration::from_millis(300);
            let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(width, main.height), gallery::main_frame(&main, &backdrop));
            assert!(ui.find(settings.as_str()).is_ok(), "controls remain drawn during the fade");
            if let Ok(dir) = std::env::var("DOSSIER_REST_REVIEW") {
                if lang == Lang::Ru && width == 980.0 {
                    std::fs::create_dir_all(&dir).unwrap();
                    ui.snapshot(&dossier_native::theme::theme()).unwrap().matches_image(std::path::Path::new(&dir).join("fading")).unwrap();
                }
            }
            ui.click(settings.as_str()).unwrap();
            let messages: Vec<_> = ui.into_messages().collect();
            assert!(messages.iter().any(|message| matches!(message, dossier_native::Message::Main(MainMessage::UserInput(None)))));
            assert!(!messages.iter().any(|message| matches!(message, dossier_native::Message::Main(MainMessage::Show(_)))), "the fading controls cannot be activated: {messages:?}");
            main.resting = iced::Animation::new(true);
            let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(width, main.height), gallery::main_frame(&main, &backdrop));
            assert!(ui.find(settings.as_str()).is_err());
            assert!(ui.find(main.words.t("replays")).is_err());
            let mark = ui.find(iced::widget::Id::new("rest-mark")).unwrap().bounds();
            assert!((mark.x - 40.0).abs() < 1.0 && (mark.y + mark.height - (main.height - 40.0)).abs() < 1.0);
            assert!((mark.height - dossier_native::ui::EMBLEM * 1.5).abs() < 1.0);
            assert!(ui.find("Dossier").is_ok(), "resting keeps the name beside the enlarged emblem");
            if let Ok(dir) = std::env::var("DOSSIER_REST_REVIEW") {
                if lang == Lang::Ru {
                    std::fs::create_dir_all(&dir).unwrap();
                    ui.snapshot(&dossier_native::theme::theme()).unwrap().matches_image(std::path::Path::new(&dir).join(format!("rest-{width}"))).unwrap();
                }
            }
            drop(ui);
            let _ = main.update(MainMessage::UserInput(None));
            let _ = main.update(MainMessage::Tick(std::time::Instant::now() + std::time::Duration::from_secs(1)));
            let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(width, main.height), gallery::main_frame(&main, &backdrop));
            assert!(ui.find(settings.as_str()).is_ok());
            assert!(ui.find(iced::widget::Id::new("rest-mark")).is_err());
            ui.click(settings.as_str()).unwrap();
            assert!(ui.into_messages().any(|message| matches!(message, dossier_native::Message::Main(MainMessage::Show(dossier_native::main_screen::Overlay::Settings)))));
        }
    }
}

#[test]
fn a_toast_background_cannot_paint_below_its_card() {
    let backdrop = dossier_native::ui::backdrop_handle();
    let (_, mut main) = gallery::main_states(Lang::En).into_iter().find(|(name, _)| name == "main-rendered").unwrap();
    let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(main.width, main.height), gallery::main_frame(&main, &backdrop));
    let bounds = ui.find(iced::widget::Id::new("toast-card")).unwrap().bounds();
    let dir = std::env::temp_dir().join(format!("dossier-toast-boundary-{}", std::process::id()));
    let with_path = gallery::write_snapshot(&ui.snapshot(&dossier_native::theme::theme()).unwrap(), &dir.join("with-toast")).unwrap();
    let with = image::open(&with_path).unwrap().to_rgba8();
    drop(ui);
    main.notices.notices.clear();
    let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(main.width, main.height), gallery::main_frame(&main, &backdrop));
    let without_path = gallery::write_snapshot(&ui.snapshot(&dossier_native::theme::theme()).unwrap(), &dir.join("without-toast")).unwrap();
    let without = image::open(&without_path).unwrap().to_rgba8();
    let scale = with.width() as f32 / main.width;
    for y in ((bounds.y + bounds.height + 36.0) * scale).ceil() as u32..((bounds.y + bounds.height + 60.0) * scale).floor() as u32 {
        for x in ((bounds.x + 10.0) * scale).ceil() as u32..((bounds.x + bounds.width - 10.0) * scale).floor() as u32 {
            assert_eq!(with.get_pixel(x, y), without.get_pixel(x, y), "toast background escaped at {x}, {y}");
        }
    }
    std::fs::remove_file(with_path).unwrap();
    std::fs::remove_file(without_path).unwrap();
    std::fs::remove_dir(dir).unwrap();
}

#[test]
fn notification_actions_fit_and_popup_close_keeps_the_event_in_the_feed() {
    use dossier_native::{main_screen::Message as MainMessage, Message};
    let backdrop = dossier_native::ui::backdrop_handle();
    for lang in Lang::ALL {
        let states = gallery::main_states(lang);
        let (_, base) = states.iter().find(|(name, _)| name == "main-notifications").unwrap();
        for (width, height) in [(980.0, 480.0), (980.0, 720.0), (1440.0, 900.0)] {
            let mut main = base.clone();
            main.width = width;
            main.height = height;
            let bad = main.toasts.iter().find(|toast| toast.stays).unwrap().id;
            let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(width, height), gallery::main_frame(&main, &backdrop));
            for key in ["notice-details", "once-more"] {
                let bounds = ui.find(main.words.t(key)).unwrap().bounds();
                assert!(bounds.x > width - 440.0 && bounds.x + bounds.width < width && bounds.y + bounds.height < height - 20.0, "{lang:?}: {bounds:?}");
            }
            let details = ui.find(main.words.t("notice-details")).unwrap().bounds();
            let retry = ui.find(main.words.t("once-more")).unwrap().bounds();
            assert!(details.x < retry.x && retry.x - (details.x + details.width) < 30.0, "notification actions must stay together on the right: {details:?}, {retry:?}");
            ui.click(main.words.t("notice-details")).unwrap();
            assert!(ui.into_messages().any(|message| matches!(message, Message::Main(MainMessage::ShowError(id)) if id == bad)));
            let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(width, height), gallery::main_frame(&main, &backdrop));
            ui.click(main.words.t("once-more")).unwrap();
            assert!(ui.into_messages().any(|message| matches!(message, Message::Main(MainMessage::ToastLink(id)) if id == bad)));
        }
        let (_, base) = states.iter().find(|(name, _)| name == "main-rendered").unwrap();
        let mut main = base.clone();
        let id = main.toasts[0].id;
        let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(main.width, main.height), gallery::main_frame(&main, &backdrop));
        let card = ui.find(iced::widget::Id::new("toast-card")).unwrap().bounds();
        ui.point_at(Point::new(card.x + card.width - 20.0, card.y + 20.0));
        ui.simulate(iced_test::simulator::click());
        let messages: Vec<_> = ui.into_messages().collect();
        assert!(messages.iter().any(|message| matches!(message, Message::Main(MainMessage::ToastClose(closed)) if *closed == id)), "{messages:?}");
        let _ = main.update(MainMessage::ToastClose(id));
        assert!(main.notices.get(id).is_some());
        let (_, base) = states.iter().find(|(name, _)| name == "main-menu-feed").unwrap();
        let id = base.notices.notices[0].id;
        let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(base.width, base.height), gallery::main_frame(base, &backdrop));
        let card = ui.find(iced::widget::Id::new("notice-card")).unwrap().bounds();
        ui.point_at(Point::new(card.x + card.width - 20.0, card.y + 20.0));
        ui.simulate(iced_test::simulator::click());
        assert!(ui.into_messages().any(|message| matches!(message, Message::Main(MainMessage::DismissNotice(closed)) if closed == id)));
    }
}

#[test]
fn notification_height_tracks_text_and_stays_bounded() {
    let backdrop = dossier_native::ui::backdrop_handle();
    let (_, base) = gallery::main_states(Lang::En).into_iter()
        .find(|(name, _)| name == "main-notifications").unwrap();
    let mut main = base.clone();
    let id = main.toasts[0].id;
    main.toasts.retain(|toast| toast.id == id);
    let notice = main.notices.notices.iter_mut().find(|notice| notice.id == id).unwrap();
    notice.mark = dossier_native::notices::Mark::Plain;
    notice.link = dossier_native::notices::Link::None;
    notice.detail.clear();
    let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(main.width, main.height), gallery::main_frame(&main, &backdrop));
    let short = ui.find(iced::widget::Id::new("toast-card")).unwrap().bounds();
    assert!(short.height < 70.0, "short notification must stay compact: {short:?}");
    drop(ui);
    main.notices.notices.iter_mut().find(|notice| notice.id == id).unwrap().detail = "A longer detail wraps across several lines and still stays within a fixed height in the popup notification card".into();
    let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(main.width, main.height), gallery::main_frame(&main, &backdrop));
    let long = ui.find(iced::widget::Id::new("toast-card")).unwrap().bounds();
    assert!(long.height > short.height + 15.0 && long.height <= 112.0, "notification must grow with text up to its cap: {short:?} -> {long:?}");
}

#[test]
fn people_search_keeps_original_member_ids_and_filters_game_friends() {
    use dossier_native::{community_screen::{Message as CommunityMessage, PeopleFrom}, main_screen::Message as MainMessage, Message};
    let backdrop = dossier_native::ui::backdrop_handle();
    for lang in Lang::ALL {
        let states = gallery::main_states(lang);
        let (_, base) = states.iter().find(|(name, _)| name == "main-community-people").unwrap();
        for width in [980.0, 1440.0] {
            let mut main = base.clone();
            main.width = width;
            main.height = 900.0;
            let at = main.community.as_ref().unwrap().people.iter().position(|person| person.name == "d1ce").unwrap();
            let order = main.community.as_ref().unwrap().ranked(dossier_native::community::Board::Pp);
            let place = order.iter().position(|index| *index == at).unwrap() + 1;
            let other_place = order.iter().position(|index| *index != at).unwrap() + 1;
            let _ = main.update(MainMessage::Community(CommunityMessage::PeopleSearch("  D1CE  ".into())));
            {
                let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(main.width, main.height), gallery::main_frame(&main, &backdrop));
                let target = ui.find(format!("#{place}")).unwrap().bounds();
                assert!(ui.find(format!("#{other_place}")).is_err());
                assert!(ui.find(main.words.of(1, main.community.as_ref().unwrap().people.len() as u64)).is_ok());
                ui.point_at(community_point(target, main.width));
                ui.simulate(iced_test::simulator::click());
                assert!(ui.into_messages().any(|message| matches!(message, Message::Main(MainMessage::Community(CommunityMessage::Person(Some(index)))) if index == at)), "filtered position must not replace the player's catalogue index");
            }
            let mut friend = main.community.as_ref().unwrap().friends[0].clone();
            friend.name = "D1CE".into();
            friend.rank = 1234;
            main.community.as_mut().unwrap().friends.push(friend);
            let _ = main.update(MainMessage::Community(CommunityMessage::PeopleFrom(PeopleFrom::Game)));
            assert_eq!(main.people_query, "  D1CE  ");
            main.now = std::time::Instant::now() + std::time::Duration::from_secs(2);
            {
                let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(main.width, main.height), gallery::main_frame(&main, &backdrop));
                assert!(ui.find("quietstorm").is_err());
                assert!(ui.find(format!("#{}", main.words.lang().group(1234))).is_ok());
                assert!(ui.find(main.words.t("metric-world")).is_ok());
                assert!(ui.find(main.words.t("people-open-dossier")).is_ok());
                let target = ui.find("D1CE").unwrap().bounds();
                ui.point_at(community_point(target, main.width));
                ui.simulate(iced_test::simulator::click());
                assert!(ui.into_messages().any(|message| matches!(message, Message::Main(MainMessage::Community(CommunityMessage::Person(Some(index)))) if index == at)));
            }
            let _ = main.update(MainMessage::Community(CommunityMessage::PeopleSearch("not-a-player-xyz".into())));
            {
                let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(main.width, main.height), gallery::main_frame(&main, &backdrop));
                assert!(ui.find(main.words.t("people-no-results")).is_ok());
            }
            let _ = main.update(MainMessage::Community(CommunityMessage::PeopleSearch(String::new())));
            let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(main.width, main.height), gallery::main_frame(&main, &backdrop));
            assert!(ui.find("quietstorm").is_ok());
            assert!(ui.find(main.words.t("open-osu")).is_ok());
            let target = ui.find("quietstorm").unwrap().bounds();
            ui.point_at(community_point(target, main.width));
            ui.simulate(iced_test::simulator::click());
            assert!(ui.into_messages().any(|message| matches!(message, Message::Main(MainMessage::Community(CommunityMessage::Open(url))) if url.ends_with("/quietstorm"))));
            if let Ok(dir) = std::env::var("DOSSIER_PEOPLE_REVIEW") {
                if (lang == Lang::Ru && width == 980.0) || (lang == Lang::En && width == 1440.0) {
                    std::fs::create_dir_all(&dir).unwrap();
                    let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(main.width, main.height), gallery::main_frame(&main, &backdrop));
                    ui.snapshot(&dossier_native::theme::theme()).unwrap().matches_image(std::path::Path::new(&dir).join(format!("people-{}-{}", lang.tag(), width as u32))).unwrap();
                }
            }
        }
    }
}

#[test]
fn people_search_input_and_clear_have_their_own_messages() {
    use dossier_native::{community_screen::Message as CommunityMessage, main_screen::Message as MainMessage, Message};
    let backdrop = dossier_native::ui::backdrop_handle();
    let states = gallery::main_states(Lang::En);
    let (_, base) = states.iter().find(|(name, _)| name == "main-community-people").unwrap();
    let mut main = base.clone();
    main.feed_query = "keep-feed-query".into();
    {
        let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(main.width, main.height), gallery::main_frame(&main, &backdrop));
        let input = ui.find(iced::widget::Id::new("community-people-search")).unwrap().bounds();
        let search = ui.find(iced::widget::Id::new("people-search-box")).unwrap().bounds();
        assert_eq!(search.width, 300.0);
        assert!((community_point(search, main.width).x - content_middle(main.width)).abs() < 1.0, "search must stay centered: {search:?}");
        ui.point_at(community_point(input, main.width));
        ui.simulate(iced_test::simulator::click());
        ui.typewrite("k");
        let messages: Vec<_> = ui.into_messages().collect();
        assert!(messages.iter().any(|message| matches!(message, Message::Main(MainMessage::Community(CommunityMessage::PeopleSearch(query))) if query == "k")), "{messages:?}");
        for message in messages { if let Message::Main(inner) = message { let _ = main.update(inner); } }
    }
    assert_eq!(main.people_query, "k");
    assert_eq!(main.feed_query, "keep-feed-query");
    let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(main.width, main.height), gallery::main_frame(&main, &backdrop));
    let clear = ui.find(main.words.t("clear")).unwrap().bounds();
    ui.point_at(community_point(clear, main.width));
    ui.simulate(iced_test::simulator::click());
    assert!(ui.into_messages().any(|message| matches!(message, Message::Main(MainMessage::Community(CommunityMessage::PeopleSearch(query))) if query.is_empty())));
}

#[test]
fn feed_stream_switches_preserve_highlight_pixels() {
    use dossier_native::{chronicle::Stream, community_screen::Message as CommunityMessage, main_screen::Message as MainMessage};
    let backdrop = dossier_native::ui::backdrop_handle();
    let states = gallery::main_states(Lang::Ru);
    let (_, base) = states.iter().find(|(name, _)| name == "main-community-feed").unwrap();
    let mut main = base.clone();
    main.width = 980.0;
    main.height = 440.0;
    let mut highlights = Vec::new();
    for (index, stream) in [Stream::All, Stream::Group, Stream::News].into_iter().enumerate() {
        let _ = main.update(MainMessage::Community(CommunityMessage::Stream(stream)));
        main.now = std::time::Instant::now() + std::time::Duration::from_millis(20);
        let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(main.width, main.height), gallery::main_frame(&main, &backdrop));
        let heading = ui.find(main.words.t("highlights").to_uppercase()).unwrap().bounds();
        let stem = std::env::temp_dir().join(format!("dossier-feed-highlights-{}-{index}", std::process::id()));
        ui.snapshot(&dossier_native::theme::theme()).unwrap().matches_image(&stem).unwrap();
        let path = gallery::written_as(&stem);
        let image = image::open(&path).unwrap().to_rgba8();
        let origin_y = dossier_native::theme::CONTROL_HEIGHT + 26.0;
        let side = dossier_native::sidebar::NARROW;
        let x = ((side + (heading.x - side) * 0.84) * 2.0).ceil() as u32;
        let y = ((origin_y + (heading.y - origin_y) * 0.84) * 2.0).ceil() as u32;
        let crop = image::imageops::crop_imm(&image, x, y, (300.0 * 0.84 * 2.0) as u32, ((heading.height + 8.0 + 146.0) * 0.84 * 2.0) as u32).to_image();
        highlights.push(crop);
        std::fs::remove_file(path).unwrap();
    }
    assert!(highlights[0] == highlights[1], "switching to games must not fade or move highlights");
    assert!(highlights[0] == highlights[2], "switching to news must not fade or move highlights");
}

#[test]
fn journal_anchor_follows_scroll_without_consumed_mouse_events() {
    use iced::widget::{row, scrollable};
    #[derive(Debug, Clone)]
    enum Message { Enter(iced::Rectangle), Exit, Click }
    let id = iced::widget::Id::unique();
    let frame = dossier_native::ui::sensed(
        button(Space::new().width(108).height(61)).padding(0).on_press(Message::Click),
        Message::Enter, Message::Exit,
    ).horizontal_viewport(40.0).risen(2.0, 1.03);
    let strip = scrollable(row![Space::new().width(70), frame, Space::new().width(500)])
        .id(id.clone()).width(200).direction(scrollable::Direction::Horizontal(scrollable::Scrollbar::new().width(0).scroller_width(0).margin(0)));
    let root = container(dossier_native::glide::glide(strip, id).grabbed()).padding(Padding::ZERO.left(40).top(20));
    let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(280.0, 120.0), root);
    let redraw = || Event::Window(iced::window::Event::RedrawRequested(std::time::Instant::now()));
    ui.point_at(Point::new(150.0, 45.0));
    ui.simulate([Event::Mouse(iced::mouse::Event::CursorMoved { position: Point::new(150.0, 45.0) })]);
    ui.simulate([Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left))]);
    for x in [140.0, 130.0, 120.0] {
        let point = Point::new(x, 45.0);
        ui.point_at(point);
        ui.simulate([Event::Mouse(iced::mouse::Event::CursorMoved { position: point }), redraw()]);
    }
    for x in [125.0, 135.0, 115.0, 130.0] {
        ui.point_at(Point::new(x, 45.0));
        ui.simulate([redraw()]);
    }
    let messages: Vec<_> = ui.into_messages().collect();
    let anchors: Vec<_> = messages.iter().filter_map(|message| match message { Message::Enter(bounds) => Some(bounds.x), _ => None }).collect();
    assert_eq!(anchors, vec![110.0, 100.0, 90.0], "{messages:?}");
    assert!(!messages.iter().any(|message| matches!(message, Message::Exit)));
}

#[test]
fn journal_bubble_does_not_interrupt_hover_across_the_frame() {
    use dossier_native::{main_screen::Message as MainMessage, Message};
    let backdrop = dossier_native::ui::backdrop_handle();
    let states = gallery::main_states(Lang::Ru);
    let (_, main) = states.iter().find(|(name, _)| name == "main-hover").unwrap();
    let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(main.width, main.height), gallery::main_frame(main, &backdrop));
    let bounds = main.hover_bounds.unwrap();
    for (x, y) in [(0.5, 0.5), (0.1, 0.1), (0.9, 0.8), (0.3, 0.2), (0.7, 0.6), (0.5, -0.03), (0.5, 0.5)] {
        let point = Point::new(bounds.x + bounds.width * x, bounds.y + bounds.height * y);
        ui.point_at(point);
        ui.simulate([Event::Mouse(iced::mouse::Event::CursorMoved { position: point }), Event::Window(iced::window::Event::RedrawRequested(std::time::Instant::now()))]);
    }
    let messages: Vec<_> = ui.into_messages().collect();
    let anchors: Vec<_> = messages.iter().filter_map(|message| match message { Message::Main(MainMessage::Over(1, bounds)) => Some(*bounds), _ => None }).collect();
    assert_eq!(anchors.len(), 1, "{messages:?}");
    assert!(!messages.iter().any(|message| matches!(message, Message::Main(MainMessage::HoverLeft(1)))), "{messages:?}");
}

#[test]
fn cover_image_cannot_paint_above_or_below_its_card() {
    let handle = iced::widget::image::Handle::from_rgba(100, 100, [255, 0, 0, 255].repeat(10_000));
    let picture: iced::Element<'_, ()> = dossier_native::ui::clipped(container(
        iced::widget::image(handle).content_fit(iced::ContentFit::Cover).width(80).height(20),
    ).width(80).height(20));
    let root = container(picture).padding(Padding::ZERO.top(40).left(40)).width(160).height(100).style(|_| container::Style {
        background: Some(iced::Background::Color(iced::Color::BLACK)), ..container::Style::default()
    });
    let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(160.0, 100.0), root);
    let stem = std::env::temp_dir().join(format!("dossier-image-clip-{}", std::process::id()));
    ui.snapshot(&dossier_native::theme::theme()).unwrap().matches_image(&stem).unwrap();
    let path = gallery::written_as(&stem);
    let image = image::open(&path).unwrap().to_rgba8();
    let scale = image.width() / 160;
    assert!(image.get_pixel(80 * scale, 50 * scale)[0] > 240);
    for y in [20, 70] {
        assert!(image.get_pixel(80 * scale, y * scale)[0] < 20, "cover must stay inside the card");
    }
    std::fs::remove_file(path).unwrap();
}

#[test]
fn profile_background_keeps_the_layout_for_own_and_other_dossiers() {
    use dossier_native::{community_screen::Section, dossier};
    let backdrop = dossier_native::ui::backdrop_handle();
    let bright = iced::widget::image::Handle::from_rgba(100, 500, [255, 255, 255, 255].repeat(50_000));
    for lang in Lang::ALL {
        let states = gallery::main_states(lang);
        for mode in ["profile", "own-dossier", "other-dossier"] {
            let stage = if mode == "other-dossier" { "main-community-person" } else { "main-community-profile" };
            let (_, base) = states.iter().find(|(name, _)| name == stage).unwrap();
            for width in [980.0, 1600.0] {
                let mut main = base.clone();
                main.width = width;
                main.height = 1100.0;
                main.now += std::time::Duration::from_secs(2);
                let at = if mode == "other-dossier" { main.community_person.unwrap() } else {
                    main.community.as_ref().unwrap().people.iter().position(|person| person.you).unwrap()
                };
                let key = format!("test-cover-{at}");
                let own_card = main.shown_card.clone().or_else(|| main.community.as_ref()?.card_of());
                let person = &mut main.community.as_mut().unwrap().people[at];
                person.cover = key.clone();
                if mode == "other-dossier" {
                    main.people_cards.insert(person.name.to_lowercase(), dossier::card_from(person));
                } else {
                    let mut card = own_card.unwrap();
                    card.cover_url = key.clone();
                    main.shown_card = Some(card);
                }
                if mode != "profile" {
                    main.community_section = Section::People;
                    main.community_person = Some(at);
                    main.person_fade = iced::Animation::new(true);
                }
                let before = {
                    let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(width, main.height), gallery::main_frame(&main, &backdrop));
                    [ui.find(iced::widget::Id::new("profile-cover")).unwrap().bounds(), ui.find(iced::widget::Id::new("profile-identity")).unwrap().bounds()]
                };
                main.news_pictures.insert(key.clone(), bright.clone());
                let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(width, main.height), gallery::main_frame(&main, &backdrop));
                let after = [ui.find(iced::widget::Id::new("profile-cover")).unwrap().bounds(), ui.find(iced::widget::Id::new("profile-identity")).unwrap().bounds()];
                assert_eq!(before, after, "loading a cover must not move profile controls");
                assert!((after[0].width - after[1].width).abs() <= 2.0);
                assert!(ui.find(main.words.t("info-country")).is_ok());
                let rank = ui.find(iced::widget::Id::from("dossier-metric-1")).unwrap().bounds();
                let country = ui.find(iced::widget::Id::from("dossier-metric-2")).unwrap().bounds();
                assert!((rank.y - country.y).abs() < 1.0);
                assert!(rank.width >= 110.0, "long ranks need sufficient space: {rank:?}");
                if let Ok(dir) = std::env::var("DOSSIER_COVER_REVIEW") {
                    if lang == Lang::Ru && (width == 980.0 || mode == "profile") {
                        drop(ui);
                        let photo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/mockups/main/bg-astral.jpg");
                        if let Ok(photo) = image::open(photo) {
                            let photo = photo.to_rgba8();
                            main.news_pictures.insert(key, iced::widget::image::Handle::from_rgba(photo.width(), photo.height(), photo.into_raw()));
                        }
                        let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(width, main.height), gallery::main_frame(&main, &backdrop));
                        std::fs::create_dir_all(&dir).unwrap();
                        ui.snapshot(&dossier_native::theme::theme()).unwrap().matches_image(std::path::Path::new(&dir).join(format!("cover-{mode}-{width}"))).unwrap();
                    }
                }
            }
        }
    }
}

#[test]
fn profile_country_rank_selects_the_chart_and_title_has_no_holder_fraction() {
    let backdrop = dossier_native::ui::backdrop_handle();
    for lang in Lang::ALL {
        let states = gallery::main_states(lang);
        let (_, main) = states.iter().find(|(name, _)| name == "main-community-profile").unwrap();
        for width in [980.0, 1600.0] {
            let mut main = main.clone();
            main.width = width;
            main.height = 900.0;
            let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(width, 900.0), gallery::main_frame(&main, &backdrop));
            let country = ui.find(main.words.t("info-country")).unwrap().bounds();
            let ranking = ui.find(main.words.t("metric-country").to_uppercase()).unwrap().bounds();
            let world = ui.find(main.words.t("metric-world").to_uppercase()).unwrap().bounds();
            assert!((ranking.y - world.y).abs() < 1.0);
            assert!(ranking.y < country.y);
            assert!(ui.find("#412").is_ok());
            assert!(ui.find(main.words.t("held-by")).is_err());
            ui.point_at(community_point(ranking, width));
            ui.simulate(iced_test::simulator::click());
            let messages: Vec<_> = ui.into_messages().collect();
            assert!(messages.iter().any(|message| matches!(message, dossier_native::Message::Main(dossier_native::main_screen::Message::Community(dossier_native::community_screen::Message::Metric(dossier_native::dossier::Metric::CountryRank))))), "{ranking:?}: {messages:?}");
            let _ = main.update(dossier_native::main_screen::Message::Community(dossier_native::community_screen::Message::Metric(dossier_native::dossier::Metric::CountryRank)));
            assert_eq!(main.dossier_metric, dossier_native::dossier::Metric::CountryRank);
            let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(width, 900.0), gallery::main_frame(&main, &backdrop));
            let heading = format!("{}  {}", main.words.t("metric-country"), main.words.n("days-long", 90)).to_uppercase();
            assert!(ui.find(heading).is_ok());
            if let Ok(dir) = std::env::var("DOSSIER_PROFILE_REVIEW") {
                std::fs::create_dir_all(&dir).unwrap();
                let stem = std::path::Path::new(&dir).join(format!("profile-{}-{}", lang.tag(), width as u32));
                ui.snapshot(&dossier_native::theme::theme()).unwrap().matches_image(&stem).unwrap();
            }
        }
    }
}

#[test]
fn profile_and_own_dossier_leave_title_actions_to_the_titles_tab_and_show_only_the_active_streak() {
    let backdrop = dossier_native::ui::backdrop_handle();
    for lang in Lang::ALL {
        let states = gallery::main_states(lang);
        let (_, base) = states.iter().find(|(name, _)| name == "main-community-profile").unwrap();
        let mut main = base.clone();
        main.width = 1600.0;
        main.height = 1100.0;
        let catalog = main.community.as_mut().unwrap();
        let own = catalog.people.iter().position(|person| person.you).unwrap();
        let person = &mut catalog.people[own];
        person.streak = 7;
        person.streak_best = 27;
        let title = person.title.clone().unwrap();
        let own_handle = catalog.card_of().unwrap().handle;
        main.title_pick = Some(title);
        {
            let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(main.width, main.height), gallery::main_frame(&main, &backdrop));
            assert!(ui.find(main.words.t("take-off-title")).is_err(), "main profile leaves title management to the titles tab");
            assert!(ui.find(main.words.n("streak-card", 7)).is_ok());
            assert!(ui.find(main.words.n("streak-best-n", 27)).is_err());
        }
        main.community_section = dossier_native::community_screen::Section::People;
        main.community_person = Some(own);
        main.person_fade = iced::Animation::new(true);
        let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(main.width, main.height), gallery::main_frame(&main, &backdrop));
        assert!(ui.find(main.words.t("take-off-title")).is_err(), "own dossier must not offer title removal");
        assert!(ui.find(own_handle).is_ok(), "own dossier reuses the full profile, including its Telegram handle");
        assert!(ui.find("#412").is_ok(), "own dossier keeps the cached country rank");
        assert!(ui.find(main.words.n("streak-card", 7)).is_ok());
        assert!(ui.find(main.words.n("streak-best-n", 27)).is_err());
        if let Ok(dir) = std::env::var("DOSSIER_PROFILE_REVIEW") {
            std::fs::create_dir_all(&dir).unwrap();
            ui.snapshot(&dossier_native::theme::theme()).unwrap().matches_image(std::path::Path::new(&dir).join(format!("own-dossier-{}", lang.tag()))).unwrap();
        }
        drop(ui);
        main.community.as_mut().unwrap().people[own].streak = 0;
        let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(main.width, main.height), gallery::main_frame(&main, &backdrop));
        assert!(ui.find(main.words.t("streak")).is_err(), "a past best streak must not create an active streak row");
    }
}

#[test]
fn skin_preview_respects_slider_head_size_and_retina_density() {
    let folder = std::env::temp_dir().join(format!("dossier-preview-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    image::RgbaImage::from_pixel(128, 128, image::Rgba([255, 255, 255, 255])).save(folder.join("hitcircle.png")).unwrap();
    image::RgbaImage::from_pixel(64, 64, image::Rgba([255, 0, 0, 255])).save(folder.join("sliderstartcircle.png")).unwrap();
    let standard = dossier_native::settings::skin_pattern(Some(&folder), 512, 320);
    let image = image::RgbaImage::from_raw(512, 320, standard.clone()).unwrap();
    let centre = image.get_pixel(132, 264);
    let body = image.get_pixel(162, 264);
    assert!(centre[0] > 200 && centre[1] < 50);
    assert!(body[0] < 100, "head sprite must not be stretched to slider body width");
    std::fs::remove_file(folder.join("sliderstartcircle.png")).unwrap();
    image::RgbaImage::from_pixel(128, 128, image::Rgba([255, 0, 0, 255])).save(folder.join("sliderstartcircle@2x.png")).unwrap();
    let retina = dossier_native::settings::skin_pattern(Some(&folder), 512, 320);
    assert_eq!(standard, retina, "@2x must preserve the same logical size");
    std::fs::remove_dir_all(folder).unwrap();
}

#[test]
fn weekly_leaders_keep_the_period_without_explanatory_metric_lines() {
    use dossier_native::community::Board;
    let backdrop = dossier_native::ui::backdrop_handle();
    for lang in Lang::ALL {
        let states = gallery::main_states(lang);
        let (_, base) = states.iter().find(|(name, _)| name == "main-community-feed").unwrap();
        for (index, board) in Board::ALL.into_iter().enumerate() {
            let mut main = base.clone();
            main.rank = index;
            main.width = 1440.0;
            main.height = 900.0;
            let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(1440.0, 900.0), gallery::main_frame(&main, &backdrop));
            let label = if board == Board::HitsPerPlay { main.words.t("week-average") } else { main.words.with("week-gain", &[("metric", main.words.t(board.key()))]) };
            assert!(ui.find(label).is_err());
            let heading = ui.find(main.words.t("week-leaders").to_uppercase()).unwrap().bounds();
            let period = ui.find(main.words.week_span(main.community.as_ref().unwrap().week_began)).unwrap().bounds();
            assert!(period.y >= heading.y + heading.height);
            assert!(!main.words.t("all-boards").contains('→'));
            if index == 0 {
                if let Ok(dir) = std::env::var("DOSSIER_PROFILE_REVIEW") {
                    std::fs::create_dir_all(&dir).unwrap();
                    let stem = std::path::Path::new(&dir).join(format!("weekly-{}", lang.tag()));
                    ui.snapshot(&dossier_native::theme::theme()).unwrap().matches_image(&stem).unwrap();
                }
            }
        }
    }
}

#[test]
fn community_channels_and_future_have_separate_panels_without_event_headers() {
    let backdrop = dossier_native::ui::backdrop_handle();
    for lang in Lang::ALL {
        let states = gallery::main_states(lang);
        let (_, base) = states.iter().find(|(name, _)| name == "main-community-feed").unwrap();
        for width in [980.0, 1440.0] {
            let mut main = base.clone();
            main.width = width;
            main.height = 1100.0;
            main.now += std::time::Duration::from_secs(2);
            let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(main.width, main.height), gallery::main_frame(&main, &backdrop));
            let channels = ui.find(iced::widget::Id::new("community-channels-card")).unwrap().bounds();
            let future = ui.find(iced::widget::Id::new("community-future-card")).unwrap().bounds();
            assert!(future.y >= channels.y + channels.height + 10.0, "{channels:?}, {future:?}");
            assert!((future.x - channels.x).abs() < 1.0);
            assert!((future.width - channels.width).abs() < 1.0);
            assert!(future.height >= 90.0);
            for key in ["journal-time", "journal-player", "journal-map"] {
                assert!(ui.find(main.words.t(key).to_uppercase()).is_err());
            }
            if let Ok(dir) = std::env::var("DOSSIER_COMMUNITY_REVIEW") {
                std::fs::create_dir_all(&dir).unwrap();
                ui.snapshot(&dossier_native::theme::theme()).unwrap().matches_image(std::path::Path::new(&dir).join(format!("feed-{}-{}", lang.tag(), width as u32))).unwrap();
                drop(ui);
                main.community_section = dossier_native::community_screen::Section::People;
                let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(main.width, main.height), gallery::main_frame(&main, &backdrop));
                let search = ui.find(iced::widget::Id::new("people-search-box")).unwrap().bounds();
                assert_eq!(search.width, 300.0);
                assert!((community_point(search, main.width).x - content_middle(main.width)).abs() < 1.0);
                ui.snapshot(&dossier_native::theme::theme()).unwrap().matches_image(std::path::Path::new(&dir).join(format!("people-{}-{}", lang.tag(), width as u32))).unwrap();
            }
        }
    }
}

#[test]
fn moving_across_the_raised_frame_edge_keeps_hover_and_clicks_stable() {
    #[derive(Debug, Clone, PartialEq)]
    enum Message { Enter, Exit, Click }
    let frame = dossier_native::ui::sensed(
        button(Space::new().width(108.0).height(61.0)).padding(0).on_press(Message::Click),
        |_| Message::Enter,
        Message::Exit,
    )
    .hit_padding(Padding { top: 4.0, right: 2.0, bottom: 0.0, left: 2.0 })
    .risen(2.0, 1.03);
    let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(160.0, 100.0), container(frame).padding(10));
    for y in [12.0, 9.0, 11.0, 8.0, 10.0, 7.5, 12.0, 9.0] {
        let point = Point::new(60.0, y);
        ui.point_at(point);
        ui.simulate([Event::Mouse(iced::mouse::Event::CursorMoved { position: point })]);
        ui.simulate([Event::Window(iced::window::Event::RedrawRequested(std::time::Instant::now()))]);
    }
    ui.simulate(iced_test::simulator::click());
    let outside = Point::new(60.0, 4.0);
    ui.point_at(outside);
    ui.simulate([Event::Mouse(iced::mouse::Event::CursorMoved { position: outside })]);
    let messages: Vec<_> = ui.into_messages().collect();
    assert_eq!(messages, [Message::Enter, Message::Click, Message::Exit]);
}

#[test]
fn video_dates_and_times_have_separate_rows_and_accounts_have_no_build() {
    let backdrop = dossier_native::ui::backdrop_handle();
    for lang in Lang::ALL {
        let states = gallery::main_states(lang);
        let (_, base) = states.iter().find(|(name, _)| name == "main-videos").unwrap();
        let mut videos = base.clone();
        videos.store.videos[0].made_at = 1_735_689_600;
        let stamp = videos.words.compact_date(videos.store.videos[0].made_at, videos.now_unix);
        let (day, time) = stamp.rsplit_once(' ').unwrap();
        let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(980.0, 720.0), gallery::main_frame(&videos, &backdrop));
        let date_bounds = ui.find(day).unwrap().bounds();
        let time_bounds = ui.find(time).unwrap().bounds();
        assert!(date_bounds.y + date_bounds.height <= time_bounds.y);
        assert!((date_bounds.center_x() - time_bounds.center_x()).abs() < 0.5);
        assert!(date_bounds.width <= 80.0);
        if let Ok(dir) = std::env::var("DOSSIER_CATALOGUE_REVIEW") {
            std::fs::create_dir_all(&dir).unwrap();
            let stem = std::path::Path::new(&dir).join(format!("videos-{}", lang.tag()));
            ui.snapshot(&dossier_native::theme::theme()).unwrap().matches_image(&stem).unwrap();
        }
        for name in ["main-menu-account", "main-menu-guest"] {
            let (_, main) = states.iter().find(|(state, _)| state == name).unwrap();
            let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(980.0, 720.0), gallery::main_frame(main, &backdrop));
            assert!(ui.find(main.words.t("build")).is_err());
            assert!(ui.find(dossier_native::bot::BUILD).is_err());
        }
    }
}

#[test]
fn a_collapsing_notification_releases_its_spacing_without_a_final_jump() {
    use iced::widget::{column, text};
    let mut previous = f32::INFINITY;
    for fraction in [1.0, 0.75, 0.5, 0.25, 0.0] {
        let card = container(text("closing")).height(100.0);
        let row = dossier_native::ui::collapsing(container(card).padding(Padding::ZERO.bottom(8.0)), fraction);
        let root = column![row, container(button("next").on_press(())).id("next-row")];
        let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(400.0, 400.0), root);
        let y = ui.find(iced::widget::Id::new("next-row")).unwrap().bounds().y;
        assert!((y - 108.0 * fraction).abs() < 0.001);
        assert!(y < previous);
        previous = y;
    }
    let mut ui = Simulator::<()>::with_size(dossier_native::settings(), Size::new(400.0, 400.0), column![container(button("next").on_press(())).id("next-row")]);
    assert_eq!(ui.find(iced::widget::Id::new("next-row")).unwrap().bounds().y, previous);
}

#[test]
fn player_dossiers_dim_and_block_the_navigation_above_the_community() {
    use dossier_native::{main_screen::{Message as MainMessage, Overlay}, community_screen::Message as CommunityMessage, Message};
    let backdrop = dossier_native::ui::backdrop_handle();
    for lang in Lang::ALL {
        let (_, base) = gallery::main_states(lang).into_iter().find(|(name, _)| name == "main-community-person").unwrap();
        let mut plain = base.clone();
        plain.person_fade = iced::Animation::new(false);
        let dir = std::env::temp_dir().join(format!("dossier-modal-nav-{}-{}", std::process::id(), lang.tag()));
        let mut images = Vec::new();
        for (name, main) in [("plain", &plain), ("dossier", &base)] {
            let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(main.width, main.height), gallery::main_frame(main, &backdrop));
            let file = gallery::write_snapshot(&ui.snapshot(&dossier_native::theme::theme()).unwrap(), &dir.join(name)).unwrap();
            images.push(image::open(&file).unwrap().to_rgba8());
            std::fs::remove_file(file).unwrap();
        }
        let scale = images[0].width() as f32 / base.width;
        let mut before = 0_u64;
        let mut after = 0_u64;
        for (x, y, pixel) in images[0].enumerate_pixels().filter(|(_, y, _)| *y < (60.0 * scale) as u32) {
            let bright = pixel[0] as u64 + pixel[1] as u64 + pixel[2] as u64;
            if bright > 450 {
                before += bright;
                let p = images[1].get_pixel(x, y);
                after += p[0] as u64 + p[1] as u64 + p[2] as u64;
            }
        }
        assert!(before > 0 && after * 3 < before, "logo and both navigation rows must be dimmed: {before} -> {after}");
        std::fs::remove_dir(dir).unwrap();
        let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(base.width, base.height), gallery::main_frame(&base, &backdrop));
        ui.click(base.words.t("settings")).unwrap();
        let messages: Vec<_> = ui.into_messages().collect();
        assert!(messages.iter().any(|m| matches!(m, Message::Main(MainMessage::Community(CommunityMessage::Person(None))))));
        assert!(!messages.iter().any(|m| matches!(m, Message::Main(MainMessage::Show(Overlay::Settings)))), "navigation behind the dossier cannot activate");
    }
}

#[test]
fn mini_player_keeps_the_same_video_across_sections_and_restores_its_home() {
    use dossier_native::main_screen::{Message, Overlay};
    for (state, home) in [("main-player", Overlay::Videos), ("main-community-clip", Overlay::Community)] {
        let (_, mut main) = gallery::main_states(Lang::En).into_iter().find(|(name, _)| name == state).unwrap();
        let player = main.player.as_ref().unwrap().clone();
        let paused = player.borrow().paused;
        let _ = main.update(Message::PlayerMinimize);
        assert!(main.mini_player);
        assert!(std::rc::Rc::ptr_eq(main.player.as_ref().unwrap(), &player));
        assert_eq!(player.borrow().paused, paused, "minimizing must not restart or pause playback");
        for section in [Overlay::Settings, Overlay::None, Overlay::Videos, Overlay::Community] {
            let _ = main.update(Message::Show(section));
            assert!(std::rc::Rc::ptr_eq(main.player.as_ref().unwrap(), &player), "section {section:?} closed the mini player");
        }
        let _ = main.update(Message::PlayerRestore);
        assert!(!main.mini_player);
        assert_eq!(main.overlay, home);
        assert!(main.stage_open.value());
        assert!(std::rc::Rc::ptr_eq(main.player.as_ref().unwrap(), &player));
        let _ = main.update(Message::PlayerMinimize);
        let _ = main.update(Message::ClosePlayer);
        let _ = main.update(Message::Tick(std::time::Instant::now() + std::time::Duration::from_secs(1)));
        assert!(main.player.is_none(), "closing the mini player must release it");
    }
}
