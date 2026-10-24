#[test]
fn opening_a_video_shows_loading_and_ignores_cancelled_or_replaced_results() {
    use super::{Message, Overlay};
    let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().find(|(name, _)| name == "main-player").unwrap();
    main.overlay = Overlay::Videos;
    let at = main.open_video.unwrap();
    let path = main.store.videos[at].path.clone();
    let _ = main.update(Message::OpenVideo(at));
    assert!(main.watching() && main.stage_open.value());
    assert_eq!(main.video_request.as_ref(), Some(&path));
    assert!(main.player.as_ref().unwrap().borrow().frame.is_none());
    let serial = main.video_serial;
    let _ = main.update(Message::OpenVideo(at));
    let ready = || crate::player::Prepared(std::sync::Arc::new(std::sync::Mutex::new(Some(crate::player::Player::still(&path, 1000, 0)))));
    let _ = main.update(Message::VideoReady(serial, path.clone(), None, ready()));
    assert_eq!(main.video_request.as_ref(), Some(&path), "a replaced request cannot complete the new one");
    let serial = main.video_serial;
    let _ = main.update(Message::ClosePlayer);
    let _ = main.update(Message::VideoReady(serial, path.clone(), None, ready()));
    assert!(main.leaving_player && main.video_request.is_none());
    let _ = main.update(Message::OpenVideo(at));
    let _ = main.update(Message::VideoReady(main.video_serial, path.clone(), None, crate::player::Prepared(std::sync::Arc::new(std::sync::Mutex::new(None)))));
    assert!(main.player.is_none() && main.video_request.is_none());
    assert!(!main.cinema.value(), "failed loading returns to the catalogue");
}

#[test]
fn presence_keeps_running_in_the_tray_without_overlapping_requests() {
    let mut main = super::Main::staged(crate::lang::Words::new(crate::lang::Lang::En), crate::settings::Settings::default(), crate::library::Library::default(), None);
    main.gallery = false;
    main.set_minimized(true);
    let _ = main.update(super::Message::PresenceTick);
    assert!(!main.presence_pending);
    let without_account = main.subscription().units();
    main.settings.token = "token".into();
    assert_eq!(main.subscription().units(), without_account + 3);
    let _ = main.update(super::Message::PresenceTick);
    assert!(main.presence_pending);
    let _ = main.update(super::Message::PresenceTick);
    assert!(main.presence_pending);
    let _ = main.update(super::Message::PresenceAnswered(Err(super::Refused::Network("offline".into()))));
    assert!(!main.presence_pending);
    let _ = main.update(super::Message::PresenceTick);
    assert!(main.presence_pending);
    let _ = main.update(super::Message::PresenceAnswered(Ok(Some(7))));
    assert_eq!(main.online, Some(7), "the answer carries how many people are online");
    assert!(!main.presence_pending);
    main.gallery = true;
    let _ = main.update(super::Message::PresenceTick);
    assert!(!main.presence_pending);
}

#[test]
fn paste_shortcut_works_with_russian_layout_and_platform_modifiers() {
    use iced::keyboard::{key::{Code, Key, Physical}, Modifiers};
    for modifiers in [Modifiers::CTRL, Modifiers::COMMAND] {
        assert!(super::paste_shortcut(&Key::Character("м".into()), Physical::Code(Code::KeyV), modifiers));
        assert!(super::paste_shortcut(&Key::Character("v".into()), Physical::Code(Code::KeyV), modifiers));
        assert!(!super::paste_shortcut(&Key::Character("с".into()), Physical::Code(Code::KeyC), modifiers));
        assert!(!super::paste_shortcut(&Key::Character("м".into()), Physical::Code(Code::KeyV), modifiers | Modifiers::ALT));
    }
    assert!(!super::paste_shortcut(&Key::Character("м".into()), Physical::Code(Code::KeyV), Modifiers::empty()));
}

#[test]
fn tray_suspends_both_live_renderers_without_changing_manual_pause() {
    let mut main = super::Main::staged(crate::lang::Words::new(crate::lang::Lang::En), crate::settings::Settings::default(), crate::library::Library::default(), None);
    let make_live = || super::Live {
        control: std::sync::Arc::new(crate::live::Control::default()),
        for_path: "sample.osr".into(), frame: None, at_ms: 0.0,
        fade: iced::Animation::new(true), rest: iced::Animation::new(false),
    };
    main.live = Some(make_live());
    main.live.as_ref().unwrap().control.pause(true);
    main.reel.now = Some((0, make_live()));
    main.set_hidden(true);
    assert!(main.live.as_ref().unwrap().control.suspended());
    assert!(main.reel.now.as_ref().unwrap().1.control.suspended());
    assert!(!main.moving());
    main.set_hidden(false);
    assert!(!main.live.as_ref().unwrap().control.suspended());
    assert!(main.live.as_ref().unwrap().control.paused());
    assert!(!main.reel.now.as_ref().unwrap().1.control.suspended());
}

#[test]
fn minimizing_stops_periodic_work_and_restores_only_the_same_playing_video() {
    let mut main = super::Main::staged(crate::lang::Words::new(crate::lang::Lang::En), crate::settings::Settings::default(), crate::library::Library::default(), None);
    main.window_id = Some(iced::window::Id::unique());
    let player = std::rc::Rc::new(std::cell::RefCell::new(crate::player::Player::still(std::path::Path::new("sample.mp4"), 1_000, 0)));
    player.borrow_mut().toggle();
    main.player = Some(player.clone());
    assert!(!player.borrow().paused);
    main.set_minimized(true);
    assert!(player.borrow().paused);
    assert!(!main.moving());
    assert_eq!(main.subscription().units(), 2);
    let frozen_at = main.now;
    let _ = main.update(super::Message::Tick(frozen_at + std::time::Duration::from_secs(1)));
    assert_eq!(main.now, frozen_at);
    main.set_minimized(false);
    assert!(!player.borrow().paused);
    assert!(main.subscription().units() > 2);
    player.borrow_mut().toggle();
    main.set_minimized(true);
    main.set_minimized(false);
    assert!(player.borrow().paused);
    player.borrow_mut().toggle();
    main.set_minimized(true);
    let replacement = std::rc::Rc::new(std::cell::RefCell::new(crate::player::Player::still(std::path::Path::new("other.mp4"), 1_000, 0)));
    main.player = Some(replacement.clone());
    main.set_minimized(false);
    assert!(replacement.borrow().paused);
}

#[test]
fn compare_opens_with_you_and_them_and_holds_four_at_most() {
    use super::Message as M;
    use crate::community_screen::{Message as C, Section};
    let mut main = super::Main::staged(crate::lang::Words::new(crate::lang::Lang::En), crate::settings::Settings::default(), crate::library::Library::default(), None);
    main.community = Some(chat_catalogue());
    let _ = main.update(M::Community(C::CompareWith(1)));
    assert_eq!(main.community_section, Section::Compare);
    assert_eq!(main.compare, [7, 8], "the dossier's compare did not start with you and them");

    let _ = main.update(M::Community(C::CompareWith(0)));
    assert_eq!(main.compare, [7], "comparing yourself put you in twice");

    main.everyone = serde_json::from_str::<crate::community::wire::Everyone>(r#"{"people": [
        {"id": 901, "player": 91, "name": "a", "pp": 1}, {"id": 902, "player": 92, "name": "b", "pp": 2},
        {"id": 903, "player": 93, "name": "c", "pp": 3}, {"id": 904, "player": 94, "name": "d", "pp": 4}
    ]}"#).unwrap().people;
    main.pool_players();
    for id in [8, 901, 902, 903, 8] {
        let _ = main.update(M::Community(C::CompareAdd(id)));
    }
    assert_eq!(main.compare, [7, 8, 901, 902], "more than four were compared or one came twice");
    let _ = main.update(M::Community(C::CompareSearch("c".into())));
    let _ = main.update(M::Community(C::CompareRemove(901)));
    assert_eq!(main.compare, [7, 8, 902]);
    let _ = main.update(M::Community(C::CompareAdd(903)));
    assert_eq!(main.compare, [7, 8, 902, 903]);
    assert!(main.compare_query.is_empty(), "the search stayed after adding");

    main.compare.clear();
    let _ = main.update(M::Community(C::Section(Section::People)));
    let _ = main.update(M::Community(C::Section(Section::Compare)));
    assert_eq!(main.compare, [7], "an empty comparison did not start with you");
}

fn chat_catalogue() -> crate::community::Catalog {
    let said: crate::community::wire::Community = serde_json::from_str(r#"{"chat": -100, "group": "Osu Squad", "people": [
        {"id": 7, "player": 70, "name": "NaumRedlo", "pp": 9870, "you": true},
        {"id": 8, "player": 80, "name": "kotofey", "pp": 12480}
    ]}"#).unwrap();
    crate::community::Catalog::from_wire(said)
}

#[test]
fn every_player_comes_and_goes_with_the_setting_and_the_open_dossier_stays_open() {
    use super::Message as M;
    let mut main = super::Main::staged(crate::lang::Words::new(crate::lang::Lang::En), crate::settings::Settings::default(), crate::library::Library::default(), None);
    main.community = Some(chat_catalogue());
    main.community_person = Some(1);
    let everyone: crate::community::wire::Everyone = serde_json::from_str(r#"{"people": [
        {"id": 7, "player": 70, "name": "NaumRedlo", "pp": 9870},
        {"id": 900, "player": 90, "name": "Mirrorwave", "pp": 11215}
    ]}"#).unwrap();
    let _ = main.update(M::EveryoneArrived(Ok(everyone)));
    assert_eq!(main.community.as_ref().unwrap().people.len(), 2, "every player came in while the setting was off");

    main.settings.people_everyone = true;
    main.welcome_everyone();
    let people = &main.community.as_ref().unwrap().people;
    assert_eq!(people.iter().map(|person| person.name.as_str()).collect::<Vec<_>>(), ["NaumRedlo", "kotofey", "Mirrorwave"]);
    assert!(people[2].outside);
    assert_eq!(main.community_person, Some(1), "the open dossier moved to someone else");

    main.community_person = Some(2);
    main.settings.people_everyone = false;
    main.welcome_everyone();
    assert_eq!(main.community.as_ref().unwrap().people.len(), 2);
    assert_eq!(main.community_person, None, "a dossier of someone no longer listed stayed open");
}

#[test]
fn a_pin_refused_says_when_it_can_move_and_a_pin_moves_the_community() {
    use super::Message as M;
    use crate::community::wire::Pin;
    let mut settings = crate::settings::Settings::default();
    settings.chat_id = Some(-100);
    let mut main = super::Main::staged(crate::lang::Words::new(crate::lang::Lang::En), settings, crate::library::Library::default(), None);
    main.community = Some(chat_catalogue());
    assert_eq!(main.community_chat(), Some(-100));

    let _ = main.update(M::PinRead(Ok(Pin { chat: Some(-200), since: Some(1_790_000_000), free_at: Some(1_792_592_000), error: String::new() })));
    assert_eq!(main.community_chat(), Some(-200), "the community did not follow the pinned chat");

    main.hint = None;
    let _ = main.update(M::Pinned(Ok(Pin { chat: Some(-200), since: Some(1_790_000_000), free_at: Some(1_792_592_000), error: "too soon".into() })));
    let (said, _) = main.hint.clone().expect("nothing was said about the refusal");
    assert!(said.contains(&main.words.day(1_792_592_000, super::unix_now())), "the refusal did not say when: {said}");
    assert_eq!(main.pin.as_ref().and_then(|pin| pin.chat), Some(-200));

    let _ = main.update(M::Pinned(Ok(Pin { chat: Some(-300), since: Some(1_795_000_000), free_at: Some(1_797_592_000), error: String::new() })));
    assert_eq!(main.community_chat(), Some(-300));
    assert_eq!(main.hint.clone().map(|(said, _)| said), Some(main.words.t("pin-done")));
}

#[test]
fn renders_asked_for_while_one_runs_wait_their_turn_and_start_by_themselves() {
    use super::Message as M;
    use crate::render::Step as R;
    let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().find(|(name, _)| name == "main-rest").unwrap();
    main.ffmpeg = Some(std::path::PathBuf::from("ffmpeg"));
    let drawable: Vec<std::path::PathBuf> = main.entries().iter().filter(|entry| entry.map.is_some()).map(|entry| entry.path.clone()).collect();
    assert!(drawable.len() >= 3, "the staged library needs three replays with maps");
    let press = |main: &mut super::Main, path: &std::path::Path| {
        main.chosen = main.entries().iter().position(|entry| entry.path == path);
        let _ = main.update(M::Render);
    };
    let queue = |main: &super::Main| main.queued.iter().map(|queued| queued.path.clone()).collect::<Vec<_>>();

    press(&mut main, &drawable[0]);
    assert_eq!(main.rendering.as_ref().map(|r| r.path.clone()), Some(drawable[0].clone()));
    assert!(main.queued.is_empty());
    press(&mut main, &drawable[1]);
    press(&mut main, &drawable[1]);
    press(&mut main, &drawable[0]);
    press(&mut main, &drawable[2]);
    assert_eq!(queue(&main), vec![drawable[1].clone(), drawable[2].clone()], "a replay was queued twice or the running one again");
    assert!(main.busy());

    let _ = main.update(M::Unqueue(drawable[2].clone()));
    assert_eq!(queue(&main), vec![drawable[1].clone()]);
    let _ = main.update(M::Rendered(R::Drawing { frames: 10, of: 100, left_seconds: 5.0 }));
    assert_eq!(main.rendering.as_ref().map(|r| r.path.clone()), Some(drawable[0].clone()), "the queue jumped ahead of a running render");

    let _ = main.update(M::Rendered(R::Stopped));
    assert_eq!(main.rendering.as_ref().map(|r| r.path.clone()), Some(drawable[1].clone()), "the next render did not start after a stop");
    assert!(main.queued.is_empty());
    assert!(main.rendering.as_ref().unwrap().reached.is_empty());
    let _ = main.update(M::Rendered(R::Failed("ffmpeg".into())));
    assert!(!main.busy(), "an empty queue kept the app busy");
}

#[test]
fn maps_download_side_by_side_and_each_keeps_its_own_progress_and_stop() {
    use super::Message as M;
    use crate::maps::Step as S;
    use std::sync::atomic::Ordering;
    let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().find(|(name, _)| name == "main-rest").unwrap();
    let mut hashes: Vec<String> = Vec::new();
    for entry in main.entries() {
        if !hashes.contains(&entry.map_hash) {
            hashes.push(entry.map_hash.clone());
        }
    }
    assert!(hashes.len() >= 4, "the staged library needs four maps");
    let choose = |main: &mut super::Main, hash: &str| {
        main.chosen = main.entries().iter().position(|entry| entry.map_hash == hash);
    };
    for hash in &hashes[..4] {
        choose(&mut main, hash);
        let _ = main.update(M::GetMap);
    }
    assert_eq!(main.fetching.len(), super::FETCHES_AT_ONCE, "a download started past the limit");
    assert!(!main.fetch_room());
    let (first, second) = (main.fetching[0].id, main.fetching[1].id);

    let _ = main.update(M::Fetched(second, S::Downloading { from: "osu.direct", done: 50, total: Some(100) }));
    assert!(main.fetching[0].last().is_none(), "one download's step went to another");
    assert!(matches!(main.fetching[1].last(), Some(S::Downloading { done: 50, .. })));
    let _ = main.update(M::Tick(main.now + std::time::Duration::from_millis(500)));
    assert!(main.fetching[1].shown > main.fetching[0].shown, "the progress is not each download's own");

    let _ = main.update(M::StopFetch(first));
    assert!(main.fetching[0].stop.load(Ordering::SeqCst));
    assert!(!main.fetching[1].stop.load(Ordering::SeqCst), "stopping one download stopped another");
    let _ = main.update(M::Fetched(first, S::Stopped));
    assert!(main.fetch_room(), "a stopped download kept its place");

    let _ = main.update(M::Fetched(first + 100, S::Nowhere));
    assert_eq!(main.fetching.len(), super::FETCHES_AT_ONCE, "a step of no download changed the list");

    choose(&mut main, &hashes[0]);
    let _ = main.update(M::GetMap);
    let again: Vec<_> = main.fetching.iter().filter(|fetching| fetching.hash == hashes[0]).collect();
    assert_eq!(again.len(), 1, "the map is fetched twice");
    assert_ne!(again[0].id, first);
    let _ = main.update(M::Fetched(first, S::Downloading { from: "osu.direct", done: 1, total: None }));
    assert!(main.fetch_of(&hashes[0]).unwrap().last().is_none(), "a late step of the stopped download reached the new one");
}

#[test]
fn notification_hover_preserves_remaining_time_and_ignores_duplicate_enters() {
    use std::time::{Duration, Instant};
    let now = Instant::now();
    let mut toast = super::Toast { id: 1, shown: iced::Animation::new(true), born: now, hovered: false, paused_at: None, stays: false };
    toast.hover(true, now + Duration::from_secs(2));
    toast.hover(true, now + Duration::from_secs(8));
    assert_eq!(toast.age(now + Duration::from_secs(10)), Duration::from_secs(2));
    toast.hover(false, now + Duration::from_secs(12));
    assert_eq!(toast.age(now + Duration::from_secs(15)), Duration::from_secs(5));
    toast.hover(false, now + Duration::from_secs(16));
    assert_eq!(toast.age(now + Duration::from_secs(16)), Duration::from_secs(6));
}

#[test]
fn settled_toasts_do_not_keep_the_window_rendering_at_frame_rate() {
    use std::time::{Duration, Instant};
    let mut main = super::Main::staged(crate::lang::Words::new(crate::lang::Lang::En), crate::settings::Settings::default(), crate::library::Library::default(), None);
    let now = Instant::now() + Duration::from_secs(10);
    let _ = main.update(super::Message::Tick(now));
    assert!(!main.moving(), "the staged screen must be settled before checking toast work");
    main.toasts.push(super::Toast { id: 1, shown: iced::Animation::new(true), born: now, hovered: false, paused_at: None, stays: true });
    assert!(!main.moving(), "a shown error needs no continuous redraw");
    let _ = main.update(super::Message::Tick(now + Duration::from_secs(7)));
    assert!(main.toasts[0].shown.value(), "an error is given longer than an ordinary notification");
    let _ = main.update(super::Message::Tick(now + Duration::from_secs(13)));
    assert!(!main.toasts[0].shown.value(), "and then goes away as well");
    assert!(main.moving(), "the dismissal animation still needs frames");
    let _ = main.update(super::Message::Tick(now + Duration::from_secs(14)));
    assert!(main.toasts.is_empty());
    assert!(!main.moving());
}

#[test]
fn dismissing_an_arriving_notification_keeps_the_visible_opacity() {
    let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().find(|(name, _)| name == "main-menu-feed").unwrap();
    let id = main.notices.notices[0].id;
    let now = std::time::Instant::now();
    main.arrivals.insert(id, iced::Animation::new(false).duration(super::NOTICE_ARRIVE).easing(iced::animation::Easing::EaseOutCubic).go(true, now - std::time::Duration::from_millis(50)));
    let before = main.arrivals[&id].interpolate(0.0_f32, 1.0, now);
    let _ = main.update(super::Message::DismissNotice(id));
    let after = main.leaving[&id].interpolate(0.0_f32, 1.0, std::time::Instant::now());
    assert!((before - after).abs() < 0.02, "dismissal must reverse the current arrival instead of jumping to full opacity: {before} -> {after}");
}

#[test]
fn notification_errors_stay_longer_and_closing_a_popup_keeps_its_history() {
    let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().find(|(name, _)| name == "main-notifications").unwrap();
    let bad = main.toasts.iter().find(|toast| toast.stays).unwrap().id;
    let _ = main.update(super::Message::Tick(main.now + std::time::Duration::from_secs(8)));
    assert!(main.toasts.iter().find(|toast| toast.id == bad).unwrap().shown.value());
    assert!(main.toasts.iter().filter(|toast| !toast.stays).all(|toast| !toast.shown.value()));
    let _ = main.update(super::Message::ToastClose(bad));
    assert!(!main.toasts.iter().find(|toast| toast.id == bad).unwrap().shown.value());
    assert!(main.notices.get(bad).is_some());
    main.height = 400.0;
    for n in 0..8 {
        main.announce(crate::notices::Mark::Plain, format!("event {n}"), String::new(), String::new(), String::new(), crate::notices::Link::None);
    }
    assert_eq!(main.toasts.iter().filter(|toast| toast.shown.value()).count(), 2);
    assert_eq!(main.notices.notices.len(), 11);
    assert_eq!(super::notice_preview("Карта\nс очень длинным именем", 8), "Карта с…");
}

#[test]
fn repeated_pointer_positions_do_not_wake_the_resting_interface() {
    let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().find(|(name, _)| name == "main-rest").unwrap();
    main.input_pointer = Some(iced::Point::new(100.0, 100.0));
    main.resting = iced::Animation::new(true);
    let last_input = main.last_input;
    let _ = main.update(super::Message::PointerActivity(iced::Point::new(100.0, 100.0)));
    assert!(main.resting.value());
    assert_eq!(main.last_input, last_input);
    let _ = main.update(super::Message::PointerActivity(iced::Point::new(101.0, 100.0)));
    assert!(!main.resting.value());
}

#[test]
fn idle_reel_starts_from_both_timer_and_animation_ticks() {
    use super::{Message, REST_AFTER};
    let (_, base) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().find(|(name, _)| name == "main-rest").unwrap();
    for (animation, scenes_due) in [(false, false), (true, false), (true, true)] {
        let mut main = base.clone();
        main.gallery = false;
        main.settings.live_scene = true;
        main.scenes_due = scenes_due;
        let began = std::time::Instant::now();
        main.now = began;
        main.last_input = began;
        let message = |now| if animation { Message::Tick(now) } else { Message::RestCheck(now) };
        let task = main.update(message(began + REST_AFTER));
        assert!(main.rest_waiting && !main.resting.value(), "the screen stays visible until the first scene frame");
        assert!(main.reel.running(), "animation={animation}, scenes_due={scenes_due}");
        assert!(task.units() > 0, "the renderer task must reach the runtime");
        let serial = main.reel.serial;
        let _ = main.update(message(began + REST_AFTER + std::time::Duration::from_millis(16)));
        assert_eq!(main.reel.serial, serial, "later ticks must not restart the renderer");
        let _ = main.update(Message::UserInput(None));
        assert!(!main.resting.value());
        assert!(!main.reel.running());
    }
}

#[test]
fn idle_scene_waits_for_a_frame_and_keeps_it_until_waking_finishes() {
    use super::{Message, REST_AFTER, REST_FADE};
    let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().find(|(name, _)| name == "main-rest").unwrap();
    main.gallery = false;
    main.settings.live_scene = true;
    let now = std::time::Instant::now();
    main.last_input = now - REST_AFTER;
    let _ = main.update(Message::RestCheck(now));
    assert!(main.rest_waiting && !main.resting.value());
    let serial = main.reel.serial;
    let frame = crate::live::Frame::Picture { frame: crate::film::Frame::new(1, 1, 1, vec![255; 4]), at_ms: 1000.0, from_ms: 0.0, to_ms: 2000.0 };
    let _ = main.update(Message::Reel(serial, frame.clone()));
    assert!(!main.rest_waiting && main.resting.value());
    let awake_at = std::time::Instant::now() + REST_FADE;
    main.now = awake_at;
    main.wake(awake_at);
    assert!(!main.resting.value());
    assert!(main.reel.running(), "the scene and its caption survive the outgoing fade");
    let _ = main.check_rest(awake_at + REST_FADE / 2);
    assert!(main.reel.running());
    let _ = main.check_rest(awake_at + REST_FADE);
    assert!(!main.reel.running());
    assert!(main.reel.before.is_none(), "release scene memory when it is invisible");
    let _ = main.update(Message::Reel(serial, frame));
    assert!(!main.resting.value() && !main.reel.running(), "late frames cannot restart the idle scene");
}

#[test]
fn horizontal_arrows_open_the_focused_carousel_group() {
    use super::{Message, Overlay, select::Focus};
    use iced::keyboard::key::Named;
    let (_, base) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().find(|(name, _)| name == "main-rest").unwrap();
    for key in [Named::ArrowLeft, Named::ArrowRight] {
        let mut main = base.clone();
        let at = main.laid().rows.iter().find_map(|row| match row {
            crate::carousel::Row::Head { first, open: false, .. } => Some(*first),
            _ => None,
        }).unwrap();
        main.focus = Some(Focus::Head(at));
        main.overlay = Overlay::Settings;
        let before = main.chosen;
        let _ = main.update(Message::Key(key.clone()));
        assert_eq!(main.chosen, before, "panels must not change carousel selection");
        main.overlay = Overlay::None;
        let _ = main.update(Message::Key(key.clone()));
        assert_eq!(main.chosen, Some(at));
        assert!(main.laid().rows.iter().any(|row| matches!(row, crate::carousel::Row::Head { first, open: true, .. } if *first == at)));
        let expected = main.jump_group(if key == Named::ArrowLeft { -1 } else { 1 });
        let _ = main.update(Message::Key(key));
        assert_eq!(main.chosen, expected.or(Some(at)), "an open group switches in the arrow's direction");
    }
}

#[test]
fn the_main_menu_rests_only_after_two_minutes_without_input() {
    use super::{Message, REST_AFTER};
    let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().find(|(name, _)| name == "main-rest").unwrap();
    let began = std::time::Instant::now();
    main.last_input = began;
    main.now = began;
    main.hover = Some(0);
    let _ = main.update(Message::RestCheck(began + REST_AFTER - std::time::Duration::from_millis(1)));
    assert!(!main.resting.value());
    let _ = main.update(Message::RestCheck(began + REST_AFTER));
    assert!(main.resting.value());
    assert!(main.hover.is_none());
    let _ = main.update(Message::UserInput(Some(Box::new(Message::Typed('x')))));
    assert!(!main.resting.value());
    assert!(main.search.is_empty(), "the first key wakes the interface without triggering a shortcut");
    main.resting = iced::Animation::new(false);
    let _ = main.update(Message::UserInput(Some(Box::new(Message::Typed('x')))));
    assert_eq!(main.search, "x", "keyboard actions work again after waking");
    let touched = main.last_input;
    let _ = main.update(Message::RestCheck(touched + REST_AFTER - std::time::Duration::from_millis(1)));
    assert!(!main.resting.value(), "input resets the deadline");
}

#[test]
fn open_panels_and_active_work_keep_the_interface_visible() {
    use super::{Message, REST_AFTER};
    let states = crate::gallery::main_states(crate::lang::Lang::En);
    for name in ["main-prefs", "main-community-feed", "main-videos", "main-menu-account", "main-player", "main-rendering", "main-fetching"] {
        let (_, base) = states.iter().find(|(state, _)| state == name).unwrap_or_else(|| panic!("missing gallery state {name}"));
        let mut main = base.clone();
        let began = std::time::Instant::now();
        main.last_input = began;
        main.now = began;
        let _ = main.update(Message::RestCheck(began + REST_AFTER * 2));
        assert!(!main.resting.value(), "{name}");
        assert_eq!(main.last_input, began + REST_AFTER * 2, "time in a panel does not consume the next idle interval");
    }
}

#[test]
fn closing_title_holders_keeps_the_original_dossier_return_point() {
    use crate::community_screen::Message as C;
    let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().find(|(name, _)| name == "main-community-person").unwrap();
    let original = iced::Rectangle { x: 120.0, y: 200.0, width: 320.0, height: 100.0 };
    main.community_tap = Some(original);
    let at = main.community_person.unwrap();
    let _ = main.update(super::Message::Community(C::Person(Some(at))));
    main.community_tap = Some(iced::Rectangle { x: 600.0, ..original });
    let _ = main.update(super::Message::Community(C::TitleOf("supporter".into(), None)));
    assert!(main.panel_from.is_none());
    let _ = main.update(super::Message::Community(C::Unread));
    let _ = main.update(super::Message::Community(C::Person(None)));
    assert_eq!(main.panel_from, Some(original));
    assert!(!main.person_fade.value());
}

#[test]
fn long_replay_labels_leave_room_for_the_whole_accuracy() {
    let (_, main) = crate::gallery::main_states(crate::lang::Lang::Ru).into_iter().find(|(name, _)| name == "main-rest").unwrap();
    let entry = main.chosen_entry().unwrap();
    let mut shown = main.shown(entry);
    shown.player = "A player with a very long name".repeat(3);
    shown.map = "Mega Ultra Cool Collab Extra".repeat(10);
    shown.accuracy = "100,00%".into();
    for width in [480.0, 720.0, 1000.0] {
        let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(width, 400.0), main.block(entry, &shown, &shown, 1.0, true));
        let accuracy = screen.find("100,00%").unwrap().bounds();
        let map = screen.find(shown.map.clone()).unwrap().bounds();
        assert!(accuracy.x >= 0.0 && accuracy.x + accuracy.width <= width, "{accuracy:?}");
        assert!(map.x + map.width <= accuracy.x, "{map:?} {accuracy:?}");
    }
}

#[test]
fn shared_replay_avatars_use_cached_profiles_without_a_community_catalogue() {
    let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().find(|(name, _)| name == "main-community-person").unwrap();
    main.community = None;
    main.everyone.clear();
    main.people_cards.insert("guest".into(), crate::community::wire::Card { username: "Guest".into(), avatar_url: "https://a.ppy.sh/123".into(), ..Default::default() });
    assert_eq!(main.shared_avatar("GUEST"), Some("https://a.ppy.sh/123"));
    assert_eq!(main.shared_avatar("Unknown"), None);
}

#[test]
fn a_cached_profile_keeps_its_content_and_close_control_while_updating() {
    let backdrop = crate::ui::backdrop_handle();
    for lang in crate::lang::Lang::ALL {
        let (_, mut main) = crate::gallery::main_states(lang).into_iter().find(|(name, _)| name == "main-community-person").unwrap();
        main.settings.token = "test-account".into();
        main.settings.chat_id = Some(-42);
        main.community.as_mut().unwrap().staged = false;
        let at = main.community_person.unwrap();
        let person = main.community.as_ref().unwrap().people[at].clone();
        main.sync_dossiers();
        let mut dossier = main.community.as_ref().unwrap().me.clone().unwrap();
        dossier.person = person.clone();
        main.people_dossiers.insert(person.id, dossier);
        main.dossier_cache.request(person.id, std::time::Instant::now(), super::unix_now()).unwrap();
        main.person_fade = iced::Animation::new(true);
        main.person_at -= std::time::Duration::from_secs(2);
        for width in [980.0, 1440.0] {
            main.width = width;
            main.height = 1100.0;
            let mut ui = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(width, main.height), crate::gallery::main_frame(&main, &backdrop));
            assert!(ui.find(person.name.as_str()).is_ok());
            assert!(ui.find("✕").is_ok(), "refresh keeps the panel interactive");
            if let Ok(dir) = std::env::var("DOSSIER_DOSSIER_REVIEW") {
                std::fs::create_dir_all(&dir).unwrap();
                ui.snapshot(&crate::theme::theme()).unwrap().matches_image(std::path::Path::new(&dir).join(format!("cached-profile-{}-{}", lang.tag(), width as u32))).unwrap();
            }
        }
    }
}

#[test]
fn a_play_witness_saw_is_told_only_by_a_paired_device_and_counted_when_it_arrives() {
    let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().find(|(name, _)| name == "main-community-person").unwrap();
    let queue_path = std::env::temp_dir().join(format!("dossier-witness-main-{}.json", std::process::id()));
    main.witness_delivery = crate::witness_delivery::Queue::load(queue_path.clone());
    let replay = include_bytes!("../../tests/fixtures/witness.osr");
    let player = dossier_replay::Replay::heading(replay).unwrap().player;
    let hex: String = replay.iter().map(|byte| format!("{byte:02x}")).collect();
    let kept = crate::witness::Kept { osr: hex, passed: true, watched: Some(false), ..Default::default() };
    main.settings.token.clear();
    assert!(main.tell_task(&kept).is_none(), "an unpaired device told a play");
    main.settings.token = "test-account".into();
    for person in main.community.as_mut().unwrap().people.iter_mut().filter(|person| person.you) {
        person.name = player.clone();
    }
    assert!(main.tell_task(&kept).is_some());
    assert!(main.tell_task(&crate::witness::Kept { watched: Some(true), ..kept.clone() }).is_none());
    let owner = crate::witness_delivery::owner(&main.settings.server, &main.settings.token);
    let delivery = main.witness_delivery.ready(&owner, std::time::Instant::now()).unwrap();
    let _ = main.update(super::Message::WitnessDelivered(delivery.clone(), Err(crate::bot::Refused::Network("offline".into()))));
    assert_eq!((main.witness.told, main.witness.pending, main.witness.untold), (0, 1, false));
    let later = std::time::Instant::now() + std::time::Duration::from_secs(60);
    assert_eq!(main.witness_delivery.ready(&owner, later), Some(delivery.clone()));
    let _ = main.update(super::Message::WitnessDelivered(delivery, Ok(())));
    assert_eq!((main.witness.told, main.witness.pending, main.witness.untold), (1, 0, false));
    main.witness.take(&crate::witness::Event::Gone);
    assert_eq!((main.witness.told, main.witness.status.clone()), (1, crate::witness::Status::Absent));
    let _ = std::fs::remove_file(queue_path);
}

#[test]
fn the_overlay_switch_reaches_the_running_witness_with_the_language_and_the_journal() {
    use crate::settings_screen::Message as P;
    use crate::witness::Overlay;
    let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().find(|(name, _)| name == "main-prefs-witness").unwrap();
    let control = std::sync::Arc::new(crate::witness::Control::default());
    main.witness_control = Some(control.clone());
    main.settings.lang = crate::lang::Lang::En;
    main.settings.witness_overlay = false;
    assert_eq!(control.overlay(), None);
    let _ = main.update(super::Message::Prefs(P::WitnessOverlay(true)));
    assert!(main.settings.witness_overlay);
    assert_eq!(control.overlay(), Some(Overlay { english: true, keeps: true }));
    let _ = main.update(super::Message::Prefs(P::Witness(false)));
    assert_eq!(control.overlay(), Some(Overlay { english: true, keeps: false }), "the tile may not say a play was saved");
    let _ = main.update(super::Message::Prefs(P::PickLang(crate::lang::Lang::Ru)));
    assert_eq!(control.overlay(), Some(Overlay { english: false, keeps: false }));
    let _ = main.update(super::Message::Prefs(P::WitnessOverlay(false)));
    assert_eq!(control.overlay(), None);
}

#[test]
fn local_scores_go_to_the_bot_only_when_asked_for_once_at_a_time_and_from_where_the_last_left_off() {
    let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().find(|(name, _)| name == "main-community-person").unwrap();
    main.settings.token = "test-account".into();
    main.settings.sources = vec![crate::sources::Source { kind: crate::sources::Kind::Stable, root: std::path::PathBuf::from("/no/such/client"), on: true, ..crate::sources::shared(true) }];
    main.gallery = false;
    let _ = main.history_task(true);
    assert!(!main.history_running, "nothing is told unless the person asked for it");
    let _ = main.update(super::Message::Prefs(crate::settings_screen::Message::WitnessHistory(true)));
    assert!(main.settings.history_share && main.history_running && main.settings.history_sent == 0);
    assert!(main.history_at.is_some());
    let _ = main.history_task(true);
    assert!(main.history_running, "a second sync does not start while one is going");
    let _ = main.update(super::Message::HistoryTold(crate::history::Sent { kept: 40, through: 1_700_000_000, failed: false }));
    assert!(!main.history_running);
    assert_eq!((main.witness.history_told, main.settings.history_sent, main.witness.history_failed), (40, 1_700_000_000, false));
    let _ = main.history_task(false);
    assert!(!main.history_running, "a new sync waits for its turn");
    let _ = main.history_task(true);
    let _ = main.update(super::Message::HistoryTold(crate::history::Sent { kept: 0, through: 0, failed: true }));
    assert_eq!((main.witness.history_failed, main.settings.history_sent), (true, 1_700_000_000), "a failure leaves the mark where it was");
    let _ = main.update(super::Message::Prefs(crate::settings_screen::Message::WitnessHistory(false)));
    main.history_at = None;
    let _ = main.history_task(true);
    assert!(!main.history_running && !main.settings.history_share);
    main.settings.history_share = true;
    main.settings.token.clear();
    let _ = main.history_task(true);
    assert!(!main.history_running, "an unpaired device tells nothing");
}

#[test]
fn a_cached_dossier_stays_visible_during_refresh_and_catalogue_reordering() {
    use crate::community::wire;
    use crate::dossier_cache::Entry;
    let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().find(|(name, _)| name == "main-community-person").unwrap();
    main.settings.token = "test-account".into();
    main.settings.chat_id = Some(-42);
    main.community.as_mut().unwrap().staged = false;
    let at = main.community.as_ref().unwrap().people.iter().position(|person| !person.you).unwrap();
    let person = main.community.as_ref().unwrap().people[at].clone();
    main.community_person = Some(at);
    main.sync_dossiers();
    let time = super::unix_now();
    let request = main.dossier_cache.request(person.id, std::time::Instant::now(), time).unwrap();
    let dossier = wire::Me {
        person: wire::Person { id: person.id, name: person.name.clone(), ..Default::default() },
        points: 123,
        recent: vec![wire::Recent { play: wire::Play { map: wire::Map { title: "Cached map".into(), ..Default::default() }, ..Default::default() }, ..Default::default() }],
        ..Default::default()
    };
    let entered = main.person_at;
    let fade = main.person_fade.value();
    let _ = main.update(super::Message::PersonDossierCached(request.clone(), Some(Entry { saved_at: time - 120, dossier })));
    assert_eq!(main.people_dossiers[&person.id].points, 123);
    assert!(main.dossier_cache.loading(person.id));
    assert_eq!(main.person_at, entered);
    assert_eq!(main.person_fade.value(), fade);
    let _ = main.update(super::Message::PersonDossier(request.clone(), Err("offline".into())));
    assert_eq!(main.people_dossiers[&person.id].points, 123);
    assert!(!main.dossier_cache.loading(person.id));
    let retry = main.dossier_cache.request(person.id, std::time::Instant::now() + std::time::Duration::from_secs(20), time + 20).unwrap();
    let refreshed = wire::Me {
        points: 124,
        ..main.dossier_cache.get(person.id).unwrap().dossier.clone()
    };
    let _ = main.update(super::Message::PersonDossier(retry, Ok(refreshed)));
    assert_eq!(main.people_dossiers[&person.id].points, 124);
    assert_eq!(main.person_at, entered);
    assert_eq!(main.person_fade.value(), fade);
    let fresh = wire::Community {
        chat: Some(-42),
        people: vec![wire::Person { id: person.id, name: person.name.clone(), ..Default::default() }],
        ..Default::default()
    };
    let _ = main.update(super::Message::CommunityArrived(Ok(fresh)));
    assert_eq!(main.community_person, Some(0), "selection follows identity, not the former row position");
    let shown = &main.people_dossiers[&person.id];
    assert_eq!(shown.points, 124);
    assert!(main.community.as_ref().unwrap().maps[shown.recent[0].map].line.contains("Cached map"));
    assert_eq!(main.person_at, entered, "refresh must not restart the opening animation");
    main.settings.chat_id = Some(-43);
    main.sync_dossiers();
    assert!(main.people_dossiers.is_empty());
    let late = wire::Me { person: wire::Person { id: person.id, ..Default::default() }, points: 999, ..Default::default() };
    let _ = main.update(super::Message::PersonDossier(request, Ok(late)));
    assert!(main.people_dossiers.is_empty(), "an old group's response must not enter the new cache");
}

#[test]
fn selected_community_switches_do_not_restart_panel_animations() {
    use crate::community_screen::{Message as C, PeopleFrom, Section, Standing};
    let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().find(|(name, _)| name == "main-community-boards").unwrap();
    for standing in [Standing::Adaptive, Standing::General] {
        let _ = main.update(super::Message::Community(C::Standing(standing)));
        let started = main.shift_at;
        let _ = main.update(super::Message::Community(C::Standing(standing)));
        assert_eq!(main.shift_at, started);
    }
    for board in crate::community::Board::ALL {
        let _ = main.update(super::Message::Community(C::Board(board)));
        let started = main.shift_at;
        let _ = main.update(super::Message::Community(C::Board(board)));
        assert_eq!(main.shift_at, started);
    }
    for section in [Section::People, Section::Profile, Section::Feed, Section::Boards, Section::Titles] {
        let _ = main.update(super::Message::Community(C::Section(section)));
        let started = main.section_at;
        let _ = main.update(super::Message::Community(C::Section(section)));
        assert_eq!(main.section_at, started);
    }
    for from in [PeopleFrom::Game, PeopleFrom::Chat] {
        let _ = main.update(super::Message::Community(C::PeopleFrom(from)));
        let started = main.shift_at;
        let _ = main.update(super::Message::Community(C::PeopleFrom(from)));
        assert_eq!(main.shift_at, started);
    }
    for filter in crate::chronicle::Filter::ALL {
        let _ = main.update(super::Message::Community(C::Filter(filter)));
        let started = main.stream_at;
        let _ = main.update(super::Message::Community(C::Filter(filter)));
        assert_eq!(main.stream_at, started);
        assert_eq!(main.feed_filter, filter);
    }
}

fn seen(main: &super::Main) -> ::image::RgbaImage {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let shot = crate::gallery::snapshot_main(main, iced::Size::new(1180.0, 760.0)).expect("a frame");
    let stem = std::env::temp_dir().join(format!("dossier-feed-motion-{}-{}", std::process::id(), NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)));
    let file = crate::gallery::write_snapshot(&shot, &stem).expect("written");
    let pixels = ::image::open(&file).expect("read back").to_rgba8();
    let _ = std::fs::remove_file(file);
    pixels
}

fn moved(a: &::image::RgbaImage, b: &::image::RgbaImage) -> (f64, u32) {
    assert_eq!(a.dimensions(), b.dimensions());
    let mut sum = 0u64;
    let (mut top, mut bottom) = (u32::MAX, 0u32);
    for (y, (one, other)) in a.rows().zip(b.rows()).enumerate() {
        let row: u64 = one.zip(other).map(|(p, q)| p.0.iter().zip(q.0.iter()).map(|(x, y)| u64::from(x.abs_diff(*y))).sum::<u64>()).sum();
        if row > 0 {
            top = top.min(y as u32);
            bottom = bottom.max(y as u32);
        }
        sum += row;
    }
    (sum as f64 / (f64::from(a.width()) * f64::from(a.height()) * 4.0), if top == u32::MAX { 0 } else { bottom - top + 1 })
}

#[test]
fn a_failed_refresh_keeps_the_data_shown_remembers_how_old_it_is_and_a_new_try_does_not_blink_the_notice_away() {
    use crate::community_screen::Fetch;
    let mut main = feed_at_rest();
    main.settings.token = "token".into();
    let shown = main.community.as_ref().unwrap().people.len();
    main.community_fetch = Fetch::Fresh(1_000);
    let _ = main.update(super::Message::CommunityArrived(Err(crate::bot::Refused::Network("offline".into()))));
    assert_eq!(main.community_fetch, Fetch::Failed(Some(1_000), crate::community_screen::Fault::Offline));
    assert_eq!(main.community.as_ref().unwrap().people.len(), shown);
    let _ = main.update(super::Message::CommunityArrived(Err(crate::bot::Refused::Network("offline".into()))));
    assert_eq!(main.community_fetch, Fetch::Failed(Some(1_000), crate::community_screen::Fault::Offline), "a second failure keeps the time of the last good answer");
    let _ = main.community_task(true);
    assert_eq!(main.community_fetch, Fetch::Failed(Some(1_000), crate::community_screen::Fault::Offline), "a new try does not hide the notice");
    let _ = main.update(super::Message::CommunityArrived(Ok(crate::community::wire::Community { chat: Some(-42), ..Default::default() })));
    assert!(matches!(main.community_fetch, Fetch::Fresh(_)));
}

fn feed_at_rest() -> super::Main {
    let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().find(|(name, _)| name == "main-community-feed").unwrap();
    main.now_unix = super::unix_now();
    let _ = main.update(super::Message::Tick(std::time::Instant::now()));
    main
}

fn happen(main: &mut super::Main, message: super::Message) -> std::time::Instant {
    let at = std::time::Instant::now();
    let _ = main.update(super::Message::Tick(at));
    let _ = main.update(message);
    at
}

fn at_once(main: &mut super::Main, at: std::time::Instant) -> ::image::RgbaImage {
    let _ = main.update(super::Message::Tick(at));
    seen(main)
}

fn after(main: &mut super::Main, wait: std::time::Duration) -> ::image::RgbaImage {
    std::thread::sleep(wait);
    let _ = main.update(super::Message::Tick(std::time::Instant::now()));
    seen(main)
}

#[test]
fn a_refused_device_is_told_apart_from_a_lost_connection_and_an_unwell_server() {
    use crate::bot::Refused;
    use crate::community_screen::Fault;
    assert_eq!(Fault::of(&Refused::Network("timed out".into())), Fault::Offline);
    assert_eq!(Fault::of(&Refused::Said("401".into())), Fault::Denied);
    assert_eq!(Fault::of(&Refused::Said("500".into())), Fault::Server);
    assert_eq!(Fault::of(&Refused::NotThere), Fault::Server);
}

#[test]
fn the_refresh_button_is_there_for_a_guest_too_and_does_not_stay_running() {
    use crate::community_screen::{pulse, Pulse};
    let mut main = feed_at_rest();
    assert!(main.settings.token.is_empty());
    let state = |main: &super::Main| pulse(&main.community_ground(main.community.as_ref().unwrap(), false));
    assert_eq!(state(&main), Some(Pulse::Idle));
    let _ = main.update(super::Message::Community(crate::community_screen::Message::Again));
    assert!(!main.refresh_asked, "a guest has no answer to wait for");
    main.now = std::time::Instant::now();
    assert_eq!(state(&main), Some(Pulse::Done), "the word says it is done at once");
    main.refresh_answered = Some(std::time::Instant::now() - std::time::Duration::from_secs(3));
    main.now = std::time::Instant::now();
    assert_eq!(state(&main), Some(Pulse::Idle), "and goes back to the resting one");
}

#[test]
fn the_refresh_button_goes_from_ready_to_running_to_done_and_back_and_a_refused_device_offers_to_connect() {
    use crate::community_screen::{pulse, Fault, Fetch, Pulse};
    let mut main = feed_at_rest();
    main.settings.token = "token".into();
    main.community.as_mut().unwrap().staged = false;
    main.community_fetch = Fetch::Fresh(super::unix_now());
    let state = |main: &super::Main| pulse(&main.community_ground(main.community.as_ref().unwrap(), false));
    assert_eq!(state(&main), Some(Pulse::Idle));

    let _ = main.update(super::Message::Community(crate::community_screen::Message::Again));
    assert_eq!(state(&main), Some(Pulse::Running));

    let _ = main.update(super::Message::CommunityArrived(Ok(crate::community::wire::Community { chat: Some(-42), ..Default::default() })));
    assert_eq!(state(&main), Some(Pulse::Done));
    main.refresh_answered = Some(std::time::Instant::now() - std::time::Duration::from_secs(3));
    main.now = std::time::Instant::now();
    assert_eq!(state(&main), Some(Pulse::Idle), "the word goes back to the resting one after two seconds");

    let _ = main.update(super::Message::CommunityArrived(Ok(crate::community::wire::Community { chat: Some(-42), ..Default::default() })));
    assert_eq!(state(&main), Some(Pulse::Idle), "a quiet poll does not flash the button");

    let _ = main.update(super::Message::CommunityArrived(Err(crate::bot::Refused::Said("401".into()))));
    assert_eq!(state(&main), Some(Pulse::Failed(Fault::Denied)));
    assert_eq!(main.notice_kept.map(|(_, fault)| fault), Some(Fault::Denied));

    let _ = main.update(super::Message::Community(crate::community_screen::Message::Reconnect));
    assert_eq!(main.overlay, super::Overlay::Settings);
    assert_eq!(main.side, super::Side::Bot);
}

#[test]
fn pasting_a_map_link_outside_the_constructor_opens_a_free_pool() {
    let mut main = feed_at_rest();
    main.pools.loaded = true;
    main.pools.dir = std::env::temp_dir().join(format!("dossier-pool-paste-main-{}", std::process::id()));
    main.overlay = super::Overlay::None;
    let _ = main.update(super::Message::PoolClipboard("ordinary text".into()));
    assert_eq!(main.overlay, super::Overlay::None);
    let _ = main.update(super::Message::PoolClipboard("https://osu.ppy.sh/beatmaps/123".into()));
    assert_eq!(main.overlay, super::Overlay::Community);
    assert_eq!(main.community_section, crate::community_screen::Section::Pools);
    assert_eq!(main.pools.editing().unwrap().frame, crate::pools::Frame::Free);
    assert!(matches!(main.pools.finding, Some(crate::pools_screen::Finding::Asking)));
    let _ = std::fs::remove_dir_all(&main.pools.dir);
}

#[test]
fn dropping_a_chart_or_pool_opens_the_constructor() {
    use crate::community_screen::{Message as C, Section};
    let mut main = feed_at_rest();
    main.pools.loaded = true;
    main.pools.dir = std::env::temp_dir().join(format!("dossier-pool-drop-main-{}", std::process::id()));
    main.overlay = super::Overlay::None;
    let _ = main.update(super::Message::Dropped(std::path::PathBuf::from("example.OSU")));
    assert_eq!(main.overlay, super::Overlay::Community);
    assert_eq!(main.community_section, Section::Pools);
    assert!(main.pools.importing.is_some());
    let _ = main.update(super::Message::Dropped(std::path::PathBuf::from("example.POOL")));
    assert_eq!(main.overlay, super::Overlay::Community);
    assert_eq!(main.community_section, Section::Pools);
    let _ = main.update(super::Message::Community(C::Pools(crate::pools_screen::Message::Back)));
    main.overlay = super::Overlay::None;
    let _ = main.update(super::Message::Dropped(std::path::PathBuf::from("example.OSZ")));
    assert_eq!(main.overlay, super::Overlay::Community);
    assert!(main.pools.importing.is_some());
    let _ = main.update(super::Message::Community(C::Pools(crate::pools_screen::Message::Back)));
    assert!(main.pools.importing.is_none());
    let _ = std::fs::remove_dir_all(&main.pools.dir);
}

#[test]
fn a_late_journal_thumbnail_does_not_replace_a_sharp_pool_cover() {
    let mut main = feed_at_rest();
    let small = iced::widget::image::Handle::from_rgba(176, 100, vec![0; 176 * 100 * 4]);
    let large = iced::widget::image::Handle::from_rgba(768, 432, vec![0; 768 * 432 * 4]);
    let hash = "pool-cover".to_owned();
    let _ = main.update(super::Message::Thumb(hash.clone(), small.clone()));
    let _ = main.update(super::Message::Thumb(hash.clone(), large.clone()));
    let _ = main.update(super::Message::Thumb(hash.clone(), small));
    assert_eq!(main.thumbs[&hash].id(), large.id());
}

#[test]
fn pool_best_plays_follow_the_own_profile_and_clear_when_it_is_removed() {
    let mut main = feed_at_rest();
    main.osu_card = None;
    main.community_card = Some(crate::community::wire::Card {
        top_scores: vec![crate::community::wire::Score { beatmap_id: 42.0, pp: 100.0, ..Default::default() }],
        ..Default::default()
    });
    main.remerge();
    assert_eq!(main.pools.best.as_ref().unwrap()[0].beatmap_id, 42.0);
    main.community_card = None;
    main.remerge();
    assert!(main.pools.best.is_none());
}

#[test]
fn the_sidebar_keeps_pools_and_duels_in_the_play_group_and_opening_pools_loads_the_shelf() {
    use crate::community_screen::{Message as C, Section};
    use crate::sidebar::Entry;
    let mut main = feed_at_rest();
    main.pools.dir = std::env::temp_dir().join(format!("dossier-main-pools-{}", std::process::id()));
    let entries = main.community_entries();
    let at = entries.iter().position(|entry| matches!(entry, Entry::Item { key: "pools", .. })).expect("a pools entry");
    assert!(matches!(&entries[at - 1], Entry::Caption(words) if *words == main.words.t("community-play")));
    assert!(matches!(&entries[at + 1], Entry::Item { key: "duels", .. }));
    assert!(matches!(&entries[at + 2], Entry::Caption(words) if *words == main.words.t("community-standing")));
    assert!(!main.pools.loaded);
    let _ = main.update(super::Message::Community(C::Go(Section::Pools, None)));
    assert_eq!(main.community_section, Section::Pools);
    assert!(main.pools.loaded);
    assert_eq!(main.pools.screen, crate::pools_screen::Screen::Shelf);
    let _ = std::fs::remove_dir_all(&main.pools.dir);
}

#[test]
fn a_play_arriving_in_the_feed_opens_its_row_instead_of_pushing_the_rest_down_at_once() {
    let mut main = feed_at_rest();
    let before = seen(&main);
    let at = happen(&mut main, super::Message::LiveArrive);
    let just = at_once(&mut main, at);
    let settled = after(&mut main, std::time::Duration::from_secs_f32(crate::ui::APPEAR + 0.15));
    let (jump, _) = moved(&before, &just);
    let (whole, _) = moved(&before, &settled);
    assert!(whole > 0.2, "the arrival is on screen: {whole}");
    assert!(jump < whole * 0.25, "the first frame after the arrival moved {jump} of the {whole} it ends up moving");
}

#[test]
fn rows_that_leave_the_feed_close_gradually_instead_of_vanishing_at_once() {
    use crate::community_screen::Message as C;
    let mut main = feed_at_rest();
    main.feed_filter = crate::chronicle::Filter::News;
    let before = seen(&main);
    let at = happen(&mut main, super::Message::Community(C::ChannelRemove("osunewsru".into())));
    assert!(main.feed_leaving.busy(), "the removed channel's rows are kept to leave");
    let just = at_once(&mut main, at);
    let settled = after(&mut main, std::time::Duration::from_secs_f32(crate::ui::APPEAR + 0.15));
    let (jump, _) = moved(&before, &just);
    let (whole, _) = moved(&before, &settled);
    assert!(whole > 0.1, "the rows are gone in the end: {whole}");
    assert!(jump < whole * 0.25, "the first frame after the removal moved {jump} of the {whole} it ends up moving");
    assert!(!main.feed_leaving.busy(), "the kept rows are let go once they have left");
}

#[test]
fn a_play_that_drops_out_of_the_answer_closes_with_its_own_player_and_map_even_when_the_people_are_reordered() {
    let mut main = feed_at_rest();
    let before_frame = seen(&main);
    let before = main.feed_before();
    let catalog = main.community.as_mut().unwrap();
    catalog.live.remove(0);
    catalog.people.reverse();
    let count = catalog.people.len();
    for play in &mut catalog.live {
        play.who = count - 1 - play.who;
    }
    for happened in &mut catalog.feed {
        happened.who = count - 1 - happened.who;
    }
    main.fresh_from(before);
    assert!(main.feed_leaving.busy(), "the dropped play is kept to leave");
    let at = std::time::Instant::now();
    let just = at_once(&mut main, at);
    let settled = after(&mut main, std::time::Duration::from_secs_f32(crate::ui::APPEAR + 0.15));
    let (jump, _) = moved(&before_frame, &just);
    let (whole, _) = moved(&before_frame, &settled);
    assert!(whole > 0.05, "the row is gone in the end: {whole}");
    assert!(jump < whole * 0.25, "the first frame moved {jump} of the {whole}");
}

#[test]
fn a_feed_that_only_gains_rows_keeps_none_to_leave() {
    let mut main = feed_at_rest();
    let _ = happen(&mut main, super::Message::LiveArrive);
    assert!(!main.feed_leaving.busy());
}

#[test]
fn a_news_picture_has_its_room_before_it_arrives_and_fades_into_it() {
    let mut main = feed_at_rest();
    main.feed_filter = crate::chronicle::Filter::News;
    main.news.stories[0].at = main.now_unix + 60;
    let url = "test://story-picture".to_owned();
    main.news.stories[0].image = Some(url.clone());
    let bare = seen(&main);
    main.news_asked.insert(url.clone());
    let waiting = seen(&main);
    let lower = (bare.height() as f32 * 0.5) as u32;
    assert!((lower..bare.height()).all(|y| (0..bare.width()).all(|x| bare.get_pixel(x, y) == waiting.get_pixel(x, y))), "asking for a picture moves nothing: its slot is there from the first frame");

    let picture = iced::widget::image::Handle::from_rgba(192, 108, vec![200u8; 192 * 108 * 4]);
    let at = happen(&mut main, super::Message::NewsPicture(url, Some(picture)));
    let just = at_once(&mut main, at);
    let settled = after(&mut main, std::time::Duration::from_secs_f32(crate::chronicle::PICTURE_FADE + 0.15));
    let (jump, _) = moved(&waiting, &just);
    let (whole, _) = moved(&waiting, &settled);
    assert!(whole > 0.003, "the picture is on screen: {whole}");
    assert!(jump < whole * 0.25, "the first frame after the picture came changed {jump} of {whole}");
    let below = (settled.height() as f32 * 0.58) as u32;
    let unmoved = (below..settled.height()).all(|y| (0..settled.width()).all(|x| settled.get_pixel(x, y) == waiting.get_pixel(x, y)));
    assert!(unmoved, "only the picture's own box and the highlight changed; nothing below the picture moved");
}

#[test]
fn a_news_picture_that_never_comes_leaves_its_slot_where_it_was() {
    let mut main = feed_at_rest();
    main.feed_filter = crate::chronicle::Filter::News;
    main.news.stories[0].at = main.now_unix + 60;
    let url = "test://lost-picture".to_owned();
    main.news.stories[0].image = Some(url.clone());
    main.news_asked.insert(url.clone());
    let waiting = seen(&main);
    let at = happen(&mut main, super::Message::NewsPicture(url, None));
    let just = at_once(&mut main, at);
    let settled = after(&mut main, std::time::Duration::from_secs_f32(crate::chronicle::PICTURE_FADE + 0.15));
    let lower = (waiting.height() as f32 * 0.5) as u32;
    for frame in [&just, &settled] {
        assert!((lower..waiting.height()).all(|y| (0..waiting.width()).all(|x| waiting.get_pixel(x, y) == frame.get_pixel(x, y))), "the rows below did not move");
    }
}

#[test]
fn filter_switches_only_restart_the_timeline_transition() {
    use crate::chronicle::Filter;
    use crate::community_screen::Message as CommunityMessage;
    let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().find(|(name, _)| name == "main-community-feed").unwrap();
    let section = main.section_at;
    let shift = main.shift_at;
    for filter in [Filter::Plays, Filter::News, Filter::All] {
        let previous = main.stream_at;
        let _ = main.update(super::Message::Community(CommunityMessage::Filter(filter)));
        assert!(main.stream_at > previous);
        assert_eq!((main.section_at, main.shift_at), (section, shift));
        let same = main.stream_at;
        let _ = main.update(super::Message::Community(CommunityMessage::Filter(filter)));
        assert_eq!(main.stream_at, same, "clicking the selected filter must not replay its animation");
    }
}

#[test]
fn arrows_light_heads_and_replays_without_opening_anything_and_enter_opens() {
    use super::select::Focus;
    let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().find(|(name, _)| name == "main-hover").unwrap();
    main.gallery = false;
    main.group = crate::carousel::Group::Maps;
    let rows = main.laid().rows.clone();
    let chosen = main.chosen;
    let heads: Vec<usize> = rows.iter().filter_map(|row| match row { crate::carousel::Row::Head { first, open: false, .. } => Some(*first), _ => None }).collect();
    for _ in 0..rows.len() {
        let _ = main.update(super::Message::Step(1));
    }
    assert_eq!(main.laid().rows, rows, "no group opened on the way");
    assert_eq!(main.chosen, chosen);
    assert_eq!(main.focus, Some(Focus::Head(*heads.last().unwrap())), "the light rests on the last row");
    let _ = main.update(super::Message::PoolHot(crate::pools_screen::Hot::Confirm));
    assert_eq!(main.chosen, heads.last().copied(), "enter opens the lit group");
    assert_eq!(main.focus, None);
}

#[test]
fn wheel_moves_the_carousel_immediately_and_can_reverse_without_inertia() {
    let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().find(|(name, _)| name == "main-hover").unwrap();
    main.gallery = false;
    main.height = 360.0;
    main.group = crate::carousel::Group::Flat;
    let start = main.carousel_target();
    main.scroll = Some(start);
    let _ = main.update(super::Message::Wheel(-1.0));
    assert_eq!(main.scroll, Some(start - super::select::WHEEL_STEP));
    assert_eq!(main.scroll_speed, 0.0);
    let _ = main.update(super::Message::Wheel(1.0));
    assert_eq!(main.scroll, Some(start));
    assert_eq!(main.scroll_speed, 0.0);
}

#[test]
fn dragging_the_carousel_moves_it_with_the_pointer_and_a_fling_carries_on() {
    let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().find(|(name, _)| name == "main-hover").unwrap();
    main.gallery = false;
    main.height = 360.0;
    main.group = crate::carousel::Group::Flat;
    let start = main.carousel_target();
    let _ = main.update(super::Message::CarouselDrag(-40.0));
    assert_eq!(main.scroll, Some(start - 40.0));
    let _ = main.update(super::Message::CarouselDrop(-400.0));
    assert!(main.carousel_target() < start - 40.0, "a fling goes further than the hand");
}

#[test]
fn stepping_keeps_the_chosen_row_in_place_and_then_glides() {
    use super::{select, Main};
    use std::time::{Duration, Instant};
    let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().find(|(name, _)| name == "main-hover").unwrap();
    main.gallery = false;
    main.height = 360.0;
    main.group = crate::carousel::Group::Flat;
    let start = Instant::now();
    main.now = start;
    main.spring_carousel(start);
    main.spring_carousel(start + Duration::from_secs(2));
    let place = |main: &Main| {
        let laid = main.laid();
        let shown = main.display(&laid);
        main.scroll.unwrap() + select::row_top(&shown.rows, &shown.factors, main.chosen).unwrap()
    };
    let before_next = {
        let laid = main.laid();
        let shown = main.display(&laid);
        let next = crate::carousel::step(&main.laid().order, main.chosen, 1).unwrap();
        main.scroll.unwrap() + select::row_top(&shown.rows, &shown.factors, Some(next)).unwrap()
    };
    let _ = main.update(super::Message::Step(1));
    assert_eq!(main.focus, Some(select::Focus::Replay(crate::carousel::step(&main.laid().order, main.chosen, 1).unwrap())), "the arrow only moves the light");
    let _ = main.update(super::Message::PoolHot(crate::pools_screen::Hot::Confirm));
    assert!((place(&main) - before_next).abs() < 0.5, "the newly chosen row jumped from {before_next} to {}", place(&main));
    let mut at = start + Duration::from_secs(2);
    let mut seen = Vec::new();
    for _ in 0..30 {
        at += Duration::from_millis(16);
        main.now = at;
        main.spring_carousel(at);
        seen.push(place(&main));
    }
    let biggest = seen.windows(2).map(|pair| (pair[1] - pair[0]).abs()).fold(0.0f32, f32::max);
    assert!(biggest < 24.0, "a single frame moved {biggest} px: {seen:?}");
}

#[test]
fn jumping_to_another_group_moves_every_head_without_a_leap() {
    use super::{select, Main};
    use std::collections::HashMap;
    use std::time::{Duration, Instant};
    let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().find(|(name, _)| name == "main-hover").unwrap();
    main.gallery = false;
    main.height = 520.0;
    main.group = crate::carousel::Group::Maps;
    let start = Instant::now();
    main.now = start;
    main.spring_carousel(start);
    main.spring_carousel(start + Duration::from_secs(2));
    let heads = |main: &Main| -> HashMap<String, f32> {
        let laid = main.laid();
        let shown = main.display(&laid);
        let mut out = HashMap::new();
        for (index, row) in shown.rows.iter().enumerate() {
            if let crate::carousel::Row::Head { title, first, .. } = row {
                out.insert(format!("{title}{first}"), main.scroll.unwrap() + select::shown_top(&shown.rows, &shown.factors, index));
            }
        }
        out
    };
    for by in [1, 1, -1] {
        let mut at = main.now + Duration::from_secs(2);
        main.now = at;
        main.spring_carousel(at);
        let mut last = heads(&main);
        let _ = main.update(super::Message::Jump(by));
        for frame in 0..40 {
            at += Duration::from_millis(16);
            main.now = at;
            main.spring_carousel(at);
            let now = heads(&main);
            for (name, y) in &now {
                if let Some(was) = last.get(name) {
                    assert!((y - was).abs() < 28.0, "jump {by}, frame {frame}: {name} moved from {was} to {y}; before {:?} after {:?}", { let mut v: Vec<_> = last.iter().collect(); v.sort_by(|a, b| a.1.total_cmp(b.1)); v }, { let mut v: Vec<_> = now.iter().collect(); v.sort_by(|a, b| a.1.total_cmp(b.1)); v });
                }
            }
            last = now;
        }
    }
}

#[test]
fn typing_opens_the_search_line_and_escape_closes_the_panel_before_the_search() {
    let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().find(|(name, _)| name == "main-hover").unwrap();
    main.gallery = false;
    assert!(!main.search_shown());
    let _ = main.update(super::Message::Typed('a'));
    assert!(main.search_shown() && main.search == "a", "a letter opens the line and goes into it");
    assert_eq!(main.scroll, Some(main.carousel_target()), "typing focuses a matching row immediately");
    assert_eq!(main.scroll_speed, 0.0);
    let _ = main.update(super::Message::FiltersPanel(true));
    assert!(main.filters_open);
    let _ = main.update(super::Message::Escape);
    assert!(!main.filters_open && main.search == "a", "the first escape shuts only the panel");
    let _ = main.update(super::Message::Escape);
    assert!(!main.search_shown() && main.search.is_empty(), "the second one clears and hides the line");
    let _ = main.update(super::Message::SearchLine(true));
    assert!(main.search_shown() && main.search.is_empty(), "the line can be opened empty");
}

#[test]
fn search_and_filters_finish_opening_and_closing() {
    use super::Message;
    use std::time::Duration;
    let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().find(|(name, _)| name == "main-hover").unwrap();
    main.gallery = false;
    let start = main.now;
    for (open, offset) in [(true, 0), (false, 300)] {
        let _ = main.update(Message::SearchLine(open));
        let _ = main.update(Message::FiltersPanel(open));
        for elapsed in [0, 75, 160] {
            let now = start + Duration::from_millis(offset + elapsed);
            let _ = main.update(Message::Tick(now));
            if elapsed == 75 {
                let k = main.search_fade.interpolate(0.0_f32, 1.0, now);
                assert!(k > 0.0 && k < 1.0);
            }
        }
        assert_eq!(main.search_fade.interpolate(0.0, 1.0, main.now), if open { 1.0 } else { 0.0 });
        assert_eq!(main.filters_fade.interpolate(0.0, 1.0, main.now), if open { 1.0 } else { 0.0 });
        assert!(!main.search_fade.is_animating(main.now));
        assert!(!main.filters_fade.is_animating(main.now));
    }
}

#[test]
fn rapid_group_jumps_never_leap() {
    use super::{select, Main};
    use std::collections::HashMap;
    use std::time::{Duration, Instant};
    let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().find(|(name, _)| name == "main-hover").unwrap();
    main.gallery = false;
    main.height = 520.0;
    main.group = crate::carousel::Group::Maps;
    let start = Instant::now();
    main.now = start;
    main.spring_carousel(start);
    main.spring_carousel(start + Duration::from_secs(2));
    let heads = |main: &Main| -> HashMap<String, f32> {
        let laid = main.laid();
        let shown = main.display(&laid);
        let mut out = HashMap::new();
        for (index, row) in shown.rows.iter().enumerate() {
            if let crate::carousel::Row::Head { title, first, .. } = row {
                out.insert(format!("{title}{first}"), main.scroll.unwrap() + select::shown_top(&shown.rows, &shown.factors, index));
            }
        }
        out
    };
    let mut at = start + Duration::from_secs(2);
    main.now = at;
    let mut last = heads(&main);
    for (turn, by) in [1, 1, 1, 1, -1, -1, 1, -1, -1, -1, 1, 1].into_iter().enumerate() {
        let _ = main.update(super::Message::Jump(by));
        for frame in 0..3 + 30 * usize::from(turn == 11) {
            at += Duration::from_millis(16);
            main.now = at;
            main.spring_carousel(at);
            let now = heads(&main);
            for (name, y) in &now {
                if let Some(was) = last.get(name) {
                    assert!((y - was).abs() < 28.0, "turn {turn}, frame {frame}: {name} moved from {was} to {y}");
                }
            }
            last = now;
        }
    }
}

#[test]
fn a_saved_video_uses_encoded_metadata_instead_of_replay_time_or_current_settings() {
    let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().next().unwrap();
    let replay = main.entries()[0].path.clone();
    main.lengths.insert(replay.clone(), 300_000);
    main.settings.render_fps = 30;
    main.settings.render_height = 720;
    main.rendering = Some(super::Rendering { path: replay, reached: vec![], out: None });
    let media = crate::videos::Probe { length_ms: 202_500, width: 1920, height: 1080, fps: 120.0 };
    let _ = main.update(super::Message::Rendered(crate::render::Step::Saved("finished.mp4".into(), media)));
    let saved = &main.store.videos[0];
    assert_eq!((saved.length_ms, saved.width, saved.height, saved.fps), (202_500, 1920, 1080, 120.0));
}

#[test]
fn a_borrowed_replay_is_drawn_with_the_look_it_came_with_and_the_video_keeps_it() {
    let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().next().unwrap();
    main.ffmpeg = Some("ffmpeg".into());
    main.settings.render_fps = 60;
    let entry = main.entries().iter().find(|entry| entry.map.is_some()).cloned().unwrap();
    let own = main.render_ask(&entry).unwrap();
    let look = crate::inbox::Look { width: 1280, height: 720, fps: 30, music: 0.4, hitsounds: 0.9, play: Some(crate::render::Play { hud: false, ..crate::render::Play::default() }) };
    main.sharing.looks.insert(entry.path.clone(), look.clone());
    let borrowed = main.render_ask(&entry).unwrap();
    assert_eq!((borrowed.size, borrowed.fps, borrowed.play.hud), ((1280, 720), 30, false));
    assert_eq!((borrowed.skin, borrowed.crf, borrowed.out), (own.skin, own.crf, own.out));
    main.sharing.render_look = Some(look.clone());
    main.rendering = Some(super::Rendering { path: entry.path.clone(), reached: vec![], out: None });
    let media = crate::videos::Probe { length_ms: 1000, width: 1280, height: 720, fps: 30.0 };
    let _ = main.update(super::Message::Rendered(crate::render::Step::Saved("borrowed.mp4".into(), media)));
    assert_eq!(main.store.videos[0].look, Some(look.clone()));
    let meta = main.send_meta(&main.store.videos[0], None);
    assert_eq!(meta["settings"]["fps"], 30);
    assert_eq!(meta["settings"]["play"]["hud"], false);
    assert_eq!(meta["player"], entry.player.as_str());
    assert!(meta["chat"].is_null());
}

#[test]
fn drawing_a_received_replay_brings_it_into_the_journal_and_picks_it() {
    let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().find(|(name, _)| name == "main-videos-received-open").unwrap();
    let library = main.library.clone().unwrap();
    let at = library.entries.iter().position(|entry| entry.map.is_some()).unwrap();
    let path = library.entries[at].path.clone();
    main.sharing.videos[0].settings = Some(crate::inbox::Look { width: 1280, height: 720, fps: 30, ..crate::inbox::Look::default() });
    main.sharing.drawing = Some(3);
    let _ = main.update(super::Message::Sharing(super::sharing::Message::Drawn(3, Ok((path.clone(), Box::new(library))))));
    assert_eq!(main.overlay, super::Overlay::None);
    assert_eq!(main.sharing.open, None);
    assert_eq!(main.sharing.drawing, None);
    assert_eq!(main.chosen_entry().map(|entry| entry.path.clone()), Some(path.clone()));
    assert_eq!(main.sharing.looks.get(&path).map(|look| look.fps), Some(30));
}

#[test]
fn replays_brought_from_other_players_do_not_say_a_new_play_happened() {
    let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().next().unwrap();
    let mut library = main.library.clone().unwrap();
    let mut brought = library.entries[0].clone();
    brought.path = crate::sources::shared_root().join(format!("kotofey ({}).osr", "f".repeat(32)));
    brought.replay_hash = "f".repeat(32);
    library.entries.insert(0, brought.clone());
    let before = main.notices.notices.len();
    let _ = main.update(super::Message::Refreshed(library.clone()));
    assert_eq!(main.notices.notices.len(), before);
    assert!(main.entries().iter().any(|entry| entry.replay_hash == brought.replay_hash));
    assert_eq!(main.shown(&brought).from, "another player's replay", "a replay from the server is not told apart in the journal");
    assert_eq!(main.shown(&library.entries[1]).from, "");
    let mut own = brought;
    own.path = "/replays/own.osr".into();
    own.replay_hash = "e".repeat(32);
    library.entries.insert(0, own);
    let _ = main.update(super::Message::Refreshed(library));
    assert_eq!(main.notices.notices.len(), before + 1);
}

#[test]
fn leaving_the_previous_frame_does_not_clear_the_new_hover() {
    let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().next().unwrap();
    let bounds = iced::Rectangle::new(iced::Point::ORIGIN, iced::Size::new(108.0, 70.0));
    let _ = main.update(super::Message::Over(2, bounds));
    let _ = main.update(super::Message::Over(1, bounds));
    let since = main.hover_since;
    let _ = main.update(super::Message::HoverLeft(2));
    assert_eq!(main.hover, Some(1));
    let _ = main.update(super::Message::Over(1, bounds));
    assert_eq!(main.hover_since, since, "layout updates must not restart the hover animation");
    let _ = main.update(super::Message::HoverLeft(1));
    assert_eq!(main.hover, None);
}

#[test]
fn bubble_anchor_ignores_subpixel_noise_and_follows_real_layout_moves() {
    let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().next().unwrap();
    let bounds = iced::Rectangle::new(iced::Point::new(100.0, 650.0), iced::Size::new(108.0, 70.0));
    let _ = main.update(super::Message::Over(1, bounds));
    let since = main.hover_since;
    let noisy = iced::Rectangle { x: bounds.x + 0.1, y: bounds.y - 0.1, ..bounds };
    let _ = main.update(super::Message::Over(1, noisy));
    assert_eq!(main.hover_bounds, Some(bounds));
    let scrolled = iced::Rectangle { x: bounds.x - 24.0, ..bounds };
    let _ = main.update(super::Message::Over(1, scrolled));
    assert_eq!(main.hover_bounds, Some(scrolled));
    assert_eq!(main.hover_since, since);
}

#[test]
fn operations_menu_can_open_in_every_catalogue() {
    let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().next().unwrap();
    for overlay in [super::Overlay::None, super::Overlay::Videos, super::Overlay::Community, super::Overlay::Settings] {
        main.menu = None;
        let _ = main.update(super::Message::Show(overlay));
        let _ = main.update(super::Message::Circle);
        assert_eq!(main.menu, Some(super::Tab::Head));
    }
}

fn signed(name: &str) -> super::Main {
    let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().find(|(state, _)| state == name).expect("a staged screen");
    main.settings.token = "staged".into();
    main
}

#[test]
fn opening_a_menu_puts_away_the_toasts_and_clearing_lets_every_notification_fade_before_it_goes() {
    let mut main = signed("main-menu-feed");
    main.menu = None;
    main.menu_open = iced::Animation::new(false);
    main.announce(crate::notices::Mark::Plain, "One more".into(), String::new(), String::new(), String::new(), crate::notices::Link::None);
    assert!(main.toasts.iter().any(|toast| toast.shown.value()));
    assert!(main.bell_flash.is_animating(std::time::Instant::now()), "a new notification flashes the bell");
    let _ = main.update(super::Message::Circle);
    assert!(main.toasts.iter().all(|toast| !toast.shown.value()), "the account menu takes the place of the toasts");
    let count = main.notices.notices.len();
    assert!(count > 1);
    let _ = main.update(super::Message::ClearNotices);
    assert_eq!(main.notices.notices.len(), count, "nothing vanishes at once");
    assert_eq!(main.leaving.len(), count);
    assert!(main.leaving.values().all(|going| !going.value()));
}

#[test]
fn the_bell_and_the_avatar_open_two_separate_panels_and_only_the_bell_reads_the_notifications() {
    let mut main = signed("main-menu-account");
    main.menu = None;
    main.menu_open = iced::Animation::new(false);
    assert!(main.notices.unseen() > 0);
    let _ = main.update(super::Message::Circle);
    assert_eq!(main.menu, Some(super::Tab::Head));
    assert!(main.notices.unseen() > 0, "the account menu does not read the notifications");
    let _ = main.update(super::Message::MenuTab(super::Tab::Bell));
    assert_eq!(main.menu, Some(super::Tab::Bell));
    assert!(main.menu_open.value());
    assert_eq!(main.notices.unseen(), 0);
    let _ = main.update(super::Message::MenuTab(super::Tab::Bell));
    assert!(!main.menu_open.value(), "a second press on the bell closes its list");
    let _ = main.update(super::Message::Circle);
    assert_eq!(main.menu, Some(super::Tab::Head));
    let _ = main.update(super::Message::Circle);
    assert!(!main.menu_open.value());
}

#[test]
fn the_bell_stands_left_of_the_avatar_and_the_menu_holds_the_account_and_the_subscription_in_one_panel() {
    let words = crate::lang::Words::new(crate::lang::Lang::En);
    let size = iced::Size::new(980.0, 720.0);
    let backdrop = crate::ui::backdrop_handle();
    let main = signed("main-menu-account");
    let mut ui = iced_test::Simulator::with_size(crate::settings(), size, crate::gallery::main_frame(&main, &backdrop));
    for gone in ["Account", "Feed", "Stats", "Maps in the library"] {
        assert!(ui.find(gone).is_err(), "{gone}");
    }
    for key in ["sign-out", "billing-manage", "billing-badge-active"] {
        assert!(ui.find(words.t(key)).is_ok(), "{key}");
    }
    assert!(ui.find(words.t("bell-title")).is_err(), "the notifications are not part of the account menu");
    let quiet = signed("main-menu-quiet");
    let mut ui = iced_test::Simulator::with_size(crate::settings(), size, crate::gallery::main_frame(&quiet, &backdrop));
    ui.click(words.t("billing-subscribe").as_str()).unwrap();
    assert!(ui.into_messages().any(|message| matches!(message, crate::Message::Main(super::Message::Billing(crate::billing::Message::Open)))));
    let mut bell = signed("main-menu-feed");
    {
        let mut ui = iced_test::Simulator::with_size(crate::settings(), size, crate::gallery::main_frame(&bell, &backdrop));
        assert!(ui.find(words.t("bell-title")).is_ok() && ui.find(words.t("bell-clear")).is_ok());
        assert!(ui.find(words.t("sign-out")).is_err());
    }
    bell.notices.clear();
    bell.rendering = None;
    let mut ui = iced_test::Simulator::with_size(crate::settings(), size, crate::gallery::main_frame(&bell, &backdrop));
    assert!(ui.find(words.t("bell-empty")).is_ok() && ui.find(words.t("bell-clear")).is_err());
}

#[test]
fn regression_billing_rejects_replies_from_a_previous_account() {
    let mut main = signed("main-menu-waiting");
    let old = main.billing.subscription().cloned().unwrap();
    let epoch = main.billing_epoch;
    main.billing.email = "previous@example.org".into();
    main.billing.paying = true;
    main.billing_status_pending = true;
    main.reset_billing();
    assert!(main.billing.subscription().is_none());
    assert!(main.billing.email.is_empty() && !main.billing.paying && !main.billing.shown);
    assert!(!main.billing_status_pending);
    main.settings.token = "next-account".into();
    let notices = main.notices.notices.len();
    for reply in [crate::billing::Message::Status(Ok(Some(old.clone()))), crate::billing::Message::Paid(Ok(old.clone())), crate::billing::Message::Cancelled(Ok(old))] {
        assert_eq!(main.update(super::Message::BillingReply(epoch, reply)).units(), 0);
        assert!(main.billing.subscription().is_none());
    }
    assert_eq!(main.notices.notices.len(), notices);
}

#[test]
fn regression_billing_fetches_each_resource_once_until_it_answers() {
    let mut main = signed("main-menu-waiting");
    main.menu = None;
    main.billing.plans = None;
    assert_eq!(main.billing_step(crate::billing::Message::Open).units(), 2);
    assert_eq!(main.billing_step(crate::billing::Message::Open).units(), 0);
    assert_eq!(main.billing_step(crate::billing::Message::Poll).units(), 0);
    let epoch = main.billing_epoch;
    let _ = main.update(super::Message::BillingReply(epoch, crate::billing::Message::Status(Err("offline".into()))));
    assert_eq!(main.billing_step(crate::billing::Message::Poll).units(), 1);
    assert_eq!(main.billing_asks(true).units(), 0);
}

#[test]
fn regression_an_old_status_cannot_undo_cancellation() {
    let mut main = signed("main-menu-waiting");
    let old = main.billing.subscription().cloned().unwrap();
    let epoch = main.billing_epoch;
    let _ = main.billing_asks(false);
    main.billing.asking = true;
    assert_eq!(main.billing_step(crate::billing::Message::Cancel).units(), 1);
    assert_ne!(main.billing_epoch, epoch);
    assert_eq!(main.billing_asks(false).units(), 0);
    let mut cancelled = old.clone();
    cancelled.state = "cancelled".into();
    let _ = main.update(super::Message::BillingReply(main.billing_epoch, crate::billing::Message::Cancelled(Ok(cancelled))));
    let _ = main.update(super::Message::BillingReply(epoch, crate::billing::Message::Status(Ok(Some(old)))));
    assert_eq!(main.billing.subscription().unwrap().state, "cancelled");
}

#[test]
fn regression_failed_pictures_retry_after_a_delay_without_duplicates() {
    let mut main = signed("main-menu-waiting");
    let url = "test://retry-avatar".to_owned();
    let now = std::time::Instant::now();
    assert!(main.ask_picture(&url, now));
    assert!(!main.ask_picture(&url, now));
    let _ = main.update(super::Message::NewsPicture(url.clone(), None));
    assert!(!main.ask_picture(&url, now + std::time::Duration::from_secs(10)));
    assert!(main.ask_picture(&url, now + std::time::Duration::from_secs(31)));
    assert!(!main.ask_picture(&url, now + std::time::Duration::from_secs(32)));
    let image = iced::widget::image::Handle::from_rgba(1, 1, vec![255; 4]);
    let _ = main.update(super::Message::NewsPicture(url.clone(), Some(image)));
    assert!(main.news_picture_failures.is_empty());
    assert!(!main.ask_picture(&url, now + std::time::Duration::from_secs(60)));
    let dormant = "test://closed-picture".to_owned();
    main.news_asked.insert(dormant.clone());
    main.news_picture_failures.insert(dormant.clone(), now - std::time::Duration::from_secs(31));
    let _ = main.update(super::Message::RetryPictures);
    assert!(!main.news_picture_failures.contains_key(&dormant));
    assert!(!main.news_asked.contains(&dormant));
}

#[test]
fn the_subscription_line_opens_the_sheet_closes_the_menu_and_asks_the_server_only_when_signed_in() {
    let mut main = signed("main-menu-account");
    assert!(main.menu_open.value());
    let _ = main.update(super::Message::Billing(crate::billing::Message::Open));
    assert!(main.billing.shown && main.billing.fade.value());
    assert!(!main.menu_open.value(), "the menu steps aside for the sheet");
    let _ = main.update(super::Message::Escape);
    assert!(!main.billing.fade.value(), "escape closes the sheet first");
    let mut guest = signed("main-billing-guest");
    guest.settings.token.clear();
    let _ = guest.update(super::Message::Billing(crate::billing::Message::SignIn));
    assert!(!guest.billing.fade.value());
}

#[test]
fn a_payment_that_goes_through_while_waiting_says_so_once() {
    let mut main = signed("main-menu-waiting");
    let before = main.notices.notices.len();
    let mut paid = main.billing.subscription().cloned().unwrap();
    paid.state = "active".into();
    paid.access = true;
    paid.paid_until = Some("2999-01-01T00:00:00Z".into());
    paid.payment_url = None;
    let _ = main.update(super::Message::Billing(crate::billing::Message::Status(Ok(Some(paid.clone())))));
    assert_eq!(main.notices.notices.len(), before + 1);
    assert_eq!(main.notices.notices[0].words, crate::lang::Words::new(crate::lang::Lang::En).t("billing-thanks"));
    let _ = main.update(super::Message::Billing(crate::billing::Message::Status(Ok(Some(paid)))));
    assert_eq!(main.notices.notices.len(), before + 1);
}

#[test]
fn the_email_used_to_pay_is_remembered_for_next_time() {
    let mut main = signed("main-billing-plans");
    main.billing.email = "me@example.org".into();
    let _ = main.update(super::Message::Billing(crate::billing::Message::Pay));
    assert_eq!(main.settings.billing_email, "me@example.org");
    assert!(main.billing.paying);
}

#[test]
fn the_menu_names_the_chat_the_videos_go_to() {
    let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().next().expect("a staged screen");
    main.account = Some(crate::bot::Me { telegram_id: 7, name: "Naum Redlo".into(), username: "naumredlo".into(), avatar: false, telegram: true, player: None });
    main.chats = vec![crate::bot::Chat { id: -100, title: "Osu Squad".into(), private: false, photo: false }];
    assert_eq!(main.chat_name(), "@naumredlo");
    main.settings.chat_id = Some(-100);
    assert_eq!(main.chat_name(), "Osu Squad");
    main.chats.clear();
    main.settings.chat_title = "Osu Squad".into();
    assert_eq!(main.chat_name(), "Osu Squad", "the title is remembered before the chats arrive");
    main.settings.chat_id = Some(7);
    assert_eq!(main.chat_name(), "@naumredlo");
}

#[test]
fn events_that_arrive_while_the_feed_is_read_below_wait_and_come_in_at_the_top() {
    use crate::community_screen::Message as C;
    let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().find(|(name, _)| name == "main-community-feed").unwrap();
    let keys = main.feed_keys();
    assert!(keys.len() > 3);
    let _ = main.update(super::Message::Community(C::FeedScrolled(400.0)));
    main.fresh_from(crate::chronicle::Before { keys: keys[2..].to_vec(), frozen: None });
    assert_eq!(main.feed_held.len(), 2, "the two newest are held back while the reader is below");
    assert!(main.feed_held.contains(&keys[0]) && main.feed_held.contains(&keys[1]));
    assert!(main.feed_arrivals.ages(main.now).is_empty(), "nothing is animated while it is hidden");
    let _ = main.update(super::Message::Community(C::FeedScrolled(40.0)));
    assert!(main.feed_held.is_empty(), "back at the top they come in");
    let ages = main.feed_arrivals.ages(main.now);
    assert!(ages.contains_key(&keys[0]) && ages.contains_key(&keys[1]));
}

#[test]
fn the_new_events_pill_brings_them_in_and_a_reader_at_the_top_holds_nothing_back() {
    use crate::community_screen::Message as C;
    let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().find(|(name, _)| name == "main-community-feed").unwrap();
    let keys = main.feed_keys();
    main.fresh_from(crate::chronicle::Before { keys: keys[1..].to_vec(), frozen: None });
    assert!(main.feed_held.is_empty(), "at the top new rows open at once");
    let _ = main.update(super::Message::Community(C::FeedScrolled(500.0)));
    main.fresh_from(crate::chronicle::Before { keys: keys[3..].to_vec(), frozen: None });
    assert_eq!(main.feed_held.len(), 3);
    let _ = main.update(super::Message::Community(C::ShowNew));
    assert!(main.feed_held.is_empty());
    assert_eq!(main.feed_offset, 0.0);
    let _ = main.update(super::Message::Community(C::FeedScrolled(500.0)));
    main.fresh_from(crate::chronicle::Before { keys: keys[1..].to_vec(), frozen: None });
    let _ = main.update(super::Message::Community(C::Section(crate::community_screen::Section::People)));
    assert!(main.feed_held.is_empty(), "leaving the feed lets the held rows go");
}


fn bright_in_the_page(image: &::image::RgbaImage) -> Vec<(u32, u32, [u8; 4])> {
    let (width, height) = image.dimensions();
    let (left, top) = ((width as f32 * 0.09) as u32, (height as f32 * 0.11) as u32);
    let mut found = Vec::new();
    for y in top..height {
        for x in left..width {
            let pixel = image.get_pixel(x, y).0;
            if pixel[0] > 36 || pixel[1] > 36 || pixel[2] > 36 {
                found.push((x, y, pixel));
                if found.len() > 5 {
                    return found;
                }
            }
        }
    }
    found
}

#[test]
fn nothing_of_the_pool_editor_is_shown_before_its_entrance_has_begun() {
    let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().find(|(name, _)| name == "main-pools-editor").unwrap();
    if let crate::pools_screen::Screen::Editor(editor) = &mut main.pools.screen {
        editor.panel = crate::pools_screen::Panel::Closed;
        editor.selected = None;
    }
    main.now = std::time::Instant::now();
    main.section_at = main.now;
    let hidden = seen(&main);
    let lit = bright_in_the_page(&hidden);
    assert!(lit.is_empty(), "something is drawn at full strength at the first instant: {lit:?}");

    main.section_at = main.now - std::time::Duration::from_secs(5);
    let settled = seen(&main);
    assert!(!bright_in_the_page(&settled).is_empty(), "and the settled page is not empty");
}

#[test]
fn the_week_s_own_tile_comes_in_with_the_others_instead_of_standing_there_already() {
    let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().find(|(name, _)| name == "main-community-adaptive").unwrap();
    for person in main.community.as_mut().unwrap().people.iter_mut().filter(|person| person.you) {
        person.gained = Default::default();
    }
    main.now = std::time::Instant::now();
    main.section_at = main.now;
    main.shift_at = main.now;
    let hidden = seen(&main);
    let lit = bright_in_the_page(&hidden);
    assert!(lit.is_empty(), "the board is empty at the first instant, the own tile included: {lit:?}");
    main.section_at = main.now - std::time::Duration::from_secs(5);
    main.shift_at = main.section_at;
    let settled = seen(&main);
    let (width, height) = settled.dimensions();
    let lower = ((height as f32 * 0.55) as u32, height);
    let own_tile_lit = (lower.0..lower.1).any(|y| ((width as f32 * 0.2) as u32..(width as f32 * 0.9) as u32).any(|x| settled.get_pixel(x, y).0[0] > 60));
    assert!(own_tile_lit, "the own tile is on the page when it has settled");
}


#[test]
fn the_pool_screens_ask_for_frames_while_they_come_in_and_stop_asking_when_they_are_in() {
    use crate::community_screen::Message as C;
    let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().find(|(name, _)| name == "main-pools-shelf").unwrap();
    main.pools.dir = std::env::temp_dir().join(format!("dossier-main-pools-motion-{}", std::process::id()));
    let _ = main.update(super::Message::Tick(std::time::Instant::now()));
    assert!(!main.pools_marks.animating(main.now), "what is on screen when the section is looked at is not moving");
    let _ = main.update(super::Message::Community(C::Pools(crate::pools_screen::Message::New)));
    assert!(matches!(main.pools.screen, crate::pools_screen::Screen::Editor(_)));
    assert!(main.moving(), "a new screen must keep the ticks coming, or its clock stands still and it stays invisible");
    let clocks = main.pools_marks.clocks(main.now);
    assert!(clocks.screen < 0.5, "the clock of the new screen is young: {}", clocks.screen);
    main.now = std::time::Instant::now() + std::time::Duration::from_secs(4);
    assert!(!main.pools_marks.animating(main.now), "and it stops asking once the screen is in");
    let _ = std::fs::remove_dir_all(&main.pools.dir);
}

#[test]
fn a_closed_pool_panel_keeps_the_frames_coming_until_it_has_faded_and_then_it_is_gone() {
    use crate::community_screen::Message as C;
    use crate::pools_screen::Message as P;
    let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().find(|(name, _)| name == "main-pools-shelf").unwrap();
    main.pools.dir = std::env::temp_dir().join(format!("dossier-main-pools-leaving-{}", std::process::id()));
    let _ = main.update(super::Message::Tick(std::time::Instant::now()));
    let _ = main.update(super::Message::Community(C::Pools(P::New)));
    let _ = main.update(super::Message::Community(C::Pools(P::AddPanel(true))));
    assert_eq!(main.pools.panel_key(), "panel");
    assert_eq!(main.pools_marks.clocks(std::time::Instant::now()).gone, crate::pools_screen::SETTLED);
    let before = std::time::Instant::now();
    let _ = main.update(super::Message::Community(C::Pools(P::Select(None))));
    let after = std::time::Instant::now();
    assert_eq!(main.pools.panel_key(), "none");
    assert!(main.moving(), "the fade needs frames, or the panel stays on screen as it was");
    let fading = main.pools_marks.clocks(after + std::time::Duration::from_millis(150)).gone;
    assert!(fading >= 0.15 && fading <= 0.15 + after.duration_since(before).as_secs_f32() + 0.001, "the clock of the fade started when the panel was closed: {fading}");
    assert!(fading < crate::ui::APPEAR);
    assert!(main.pools_marks.clocks(after + std::time::Duration::from_millis(600)).gone >= crate::ui::APPEAR, "and after its time nothing of it is drawn");
    assert!(main.pools_marks.animating(after + std::time::Duration::from_millis(600)));
    assert!(!main.pools_marks.animating(after + std::time::Duration::from_secs(4)), "and the frames stop");
    let _ = std::fs::remove_dir_all(&main.pools.dir);
}
