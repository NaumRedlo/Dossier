use dossier_native::inbox::{Accept, Face, Inbox, Received, Refusal, Shared};
use dossier_native::main_screen::sharing::{Message as S, Tab};
use dossier_native::main_screen::{Main, Message as M, Overlay};
use dossier_native::settings_screen::Message as P;
use dossier_native::{bot, gallery, lang::Lang, notices};
use iced::{Point, Size};
use iced_test::Simulator;

fn staged(lang: Lang, name: &str) -> Main {
    let (_, mut main) = gallery::main_states(lang).into_iter().find(|(name_here, _)| name_here == name).unwrap();
    main.width = 980.0;
    main.height = 720.0;
    main
}

fn clicked(main: &Main, label: &str) -> Vec<String> {
    let backdrop = dossier_native::ui::backdrop_handle();
    let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(main.width, main.height), gallery::main_frame(main, &backdrop));
    let bounds = ui.find(label).unwrap_or_else(|_| panic!("no {label:?} on the screen")).bounds();
    ui.point_at(Point::new(bounds.x + bounds.width / 2.0, bounds.y + bounds.height / 2.0));
    let _ = ui.simulate(iced_test::simulator::click());
    ui.into_messages().map(|message| format!("{message:?}")).collect()
}

fn shows(main: &Main, label: &str) -> bool {
    let backdrop = dossier_native::ui::backdrop_handle();
    let mut ui = Simulator::with_size(dossier_native::settings(), Size::new(main.width, main.height), gallery::main_frame(main, &backdrop));
    ui.find(label).is_ok()
}

fn sent(id: u64, at: i64, seen: bool) -> Received {
    Received {
        id,
        from: Some(Face { player: 2, name: "kotofey".into(), ..Face::default() }),
        player: "kotofey".into(),
        song: "xi — Blue Zenith".into(),
        version: "FOUR DIMENSIONS".into(),
        duration: 134,
        size: 51_000_000,
        replay: true,
        sent_at: at,
        seen,
        ..Received::default()
    }
}

#[test]
fn a_new_video_is_told_once_and_shown_in_received() {
    let mut main = staged(Lang::En, "main-videos-received");
    let before = main.notices.notices.len();
    let inbox = Inbox { registered: true, accept: "everyone".into(), videos: vec![sent(9, 500, false), sent(8, 400, true)] };
    let _ = main.update(M::Sharing(S::Arrived(Ok(inbox.clone()))));
    assert_eq!(main.sharing.accept, Accept::Everyone);
    assert_eq!(main.notices.notices.len(), before + 1);
    assert_eq!(main.notices.notices[0].link, notices::Link::Received(9));
    assert_eq!(main.settings.inbox_heard, 500);
    let _ = main.update(M::Sharing(S::Arrived(Ok(inbox))));
    assert_eq!(main.notices.notices.len(), before + 1);
    assert!(shows(&main, "xi — Blue Zenith [FOUR DIMENSIONS]"));
}

#[test]
fn the_switch_on_the_videos_page_turns_to_received_and_back() {
    for lang in Lang::ALL {
        let mut main = staged(lang, "main-videos-received");
        let mine = main.words.t("videos-mine");
        let said = clicked(&main, &mine);
        assert!(said.iter().any(|message| message.contains("Tab(Mine)")), "{lang:?}: {said:?}");
        let _ = main.update(M::Sharing(S::Tab(Tab::Mine)));
        assert!(!shows(&main, &main.words.t("from-whom").to_uppercase()));
        let received = main.words.t("videos-received");
        let said = clicked(&main, &received);
        assert!(said.iter().any(|message| message.contains("Tab(Received)")), "{lang:?}: {said:?}");
    }
}

#[test]
fn a_received_row_opens_its_stage_with_what_can_be_done() {
    let mut main = staged(Lang::En, "main-videos-received");
    for video in &mut main.sharing.videos {
        video.storage = "telegram".to_owned();
    }
    let said = clicked(&main, "xi — Blue Zenith [FOUR DIMENSIONS]");
    assert!(said.iter().any(|message| message.contains("Open(3)")), "{said:?}");
    let _ = main.update(M::Sharing(S::Open(3)));
    assert_eq!(main.sharing.open, Some(3));
    assert!(main.sharing.videos[0].seen);
    main.stage_open = iced::Animation::new(true);
    for key in ["received-telegram", "received-draw", "received-remove"] {
        assert!(shows(&main, &main.words.t(key)), "{key} is not on the stage");
    }
    let said = clicked(&main, &main.words.t("received-telegram"));
    assert!(said.iter().any(|message| message.contains("ToTelegram")), "{said:?}");
    let _ = main.update(M::Escape);
    assert!(main.leaving_player);
}

#[test]
fn removing_a_received_video_closes_the_stage_and_takes_it_off_the_list() {
    let mut main = staged(Lang::En, "main-videos-received-open");
    let _ = main.update(M::Sharing(S::Remove));
    assert_eq!(main.sharing.open, None);
    assert!(main.sharing.videos.iter().all(|video| video.id != 3));
}

#[test]
fn a_failed_download_is_told_and_a_stopped_one_is_not() {
    let mut main = staged(Lang::En, "main-videos-received-getting");
    let before = main.notices.notices.len();
    let _ = main.update(M::Sharing(S::Got(3, Err("stopped".into()))));
    assert!(main.sharing.getting.is_none());
    assert_eq!(main.notices.notices.len(), before);
    let _ = main.update(M::Sharing(S::Got(3, Err("the video came short".into()))));
    assert_eq!(main.notices.notices.len(), before + 1);
    assert_eq!(main.notices.notices[0].mark, notices::Mark::Bad);
}

#[test]
fn the_picker_takes_players_and_sends_only_when_someone_is_chosen() {
    for lang in Lang::ALL {
        let mut main = staged(lang, "main-videos-share");
        let said = clicked(&main, "ssnowy");
        assert!(said.iter().any(|message| message.contains("Toggle(3)")), "{lang:?}: {said:?}");
        let _ = main.update(M::Sharing(S::Toggle(3)));
        let _ = main.update(M::Sharing(S::Toggle(2)));
        assert_eq!(main.sharing.picker.as_ref().unwrap().picked, vec![3]);
        let send = main.words.t("share-send");
        assert!(clicked(&main, &send).iter().any(|message| message.contains("Sharing(Send)")));
        let _ = main.update(M::Sharing(S::Toggle(3)));
        assert!(clicked(&main, &send).iter().all(|message| !message.contains("Sharing(Send)")));
        let _ = main.update(M::Escape);
        assert!(main.sharing.picker.is_none());
    }
}

#[test]
fn a_shared_video_remembers_its_number_and_says_who_got_it() {
    let mut main = staged(Lang::En, "main-videos-share");
    let path = main.sharing.picker.as_ref().unwrap().path.clone();
    let before = main.notices.notices.len();
    let outcome = Shared { sent: vec![2], refused: vec![Refusal { player: 3, why: "closed".into() }] };
    let _ = main.update(M::Sharing(S::Shared(path.clone(), Some(41), false, Ok(outcome))));
    assert_eq!(main.store.videos.iter().find(|video| video.path == path).unwrap().remote, Some(41));
    assert!(main.sharing.picker.is_none());
    assert_eq!(main.notices.notices.len(), before + 2);
    assert_eq!(main.notices.notices[1].note, "kotofey");
    assert!(main.notices.notices[0].note.starts_with("ssnowy"));
    assert_eq!(main.notices.notices[0].mark, notices::Mark::Bad);
}

#[test]
fn a_video_sent_to_telegram_keeps_the_number_the_bot_gave_it() {
    let mut main = staged(Lang::En, "main-player");
    let path = main.store.videos[0].path.clone();
    main.sending = Some(dossier_native::main_screen::Sending { path: path.clone(), done: 0, total: 10, over: None });
    let _ = main.update(M::Sent(Ok(bot::Sent { message: 5, video: Some(77) })));
    assert_eq!(main.store.videos[0].remote, Some(77));
}

#[test]
fn the_receive_setting_is_chosen_in_the_bot_settings_and_falls_back_when_refused() {
    for lang in Lang::ALL {
        let mut main = staged(lang, "main-prefs-bot");
        let everyone = main.words.t("accept-everyone");
        let said = clicked(&main, &everyone);
        assert!(said.iter().any(|message| message.contains("Accept(Everyone)")), "{lang:?}: {said:?}");
        let _ = main.update(M::Prefs(P::Accept(Accept::Everyone)));
        assert_eq!(main.sharing.accept, Accept::Everyone);
        let _ = main.update(M::Sharing(S::Accepted(Accept::Shared, Err("offline".into()))));
        assert_eq!(main.sharing.accept, Accept::Shared);
        let _ = main.update(M::Sharing(S::Accepted(Accept::Shared, Ok("nobody".into()))));
        assert_eq!(main.sharing.accept, Accept::Nobody);
    }
}

#[test]
fn a_notice_about_a_received_video_opens_it() {
    let mut main = staged(Lang::En, "main-videos-received");
    main.overlay = Overlay::None;
    let _ = main.update(M::Sharing(S::Arrived(Ok(Inbox { registered: true, accept: "shared".into(), videos: vec![sent(9, 500, false)] }))));
    let id = main.notices.notices[0].id;
    let _ = main.update(M::ToastLink(id));
    assert_eq!(main.overlay, Overlay::Videos);
    assert_eq!(main.sharing.open, Some(9));
    assert_eq!(main.sharing.tab, Tab::Received);
}

#[test]
fn sharing_own_replays_is_switched_on_the_bot_tab_and_falls_back_when_refused() {
    for lang in Lang::ALL {
        let mut main = staged(lang, "main-prefs-bot");
        let label = main.words.t("share-replays");
        let said = clicked(&main, &label);
        assert!(said.iter().any(|message| message.contains("ShareReplays(false)")), "{lang:?}: {said:?}");
        let _ = main.update(M::Prefs(P::ShareReplays(false)));
        assert_eq!(main.sharing.replays.as_ref().map(|state| (state.on, state.count)), Some((false, 0)));
        assert!(shows(&main, &main.words.t("share-replays-about")));
        let _ = main.update(M::Sharing(S::ReplaysSwitched(true, Err("offline".into()))));
        assert!(main.sharing.replays.as_ref().unwrap().on);
        let fresh = dossier_native::mixed::State { on: true, name: "NaumRedlo".into(), count: 3, most: 300 };
        let _ = main.update(M::Sharing(S::Replays(Ok(fresh))));
        assert!(shows(&main, &main.words.with("share-replays-count", &[("n", "3".to_owned()), ("most", "300".to_owned())])));
    }
}

#[test]
fn the_replays_of_other_players_are_a_source_with_a_name_and_a_switch() {
    for lang in Lang::ALL {
        let main = staged(lang, "main-prefs-shared");
        let at = main.settings.sources.iter().position(|source| source.is_shared()).unwrap();
        let said = clicked(&main, &main.words.t("source-shared"));
        assert!(said.iter().any(|message| message.contains(&format!("Source({at}, false)"))), "{lang:?}: {said:?}");
    }
}

#[test]
fn the_sign_in_sheet_offers_osu_beside_telegram() {
    for lang in Lang::ALL {
        let main = staged(lang, "main-signing");
        let said = clicked(&main, &main.words.t("sign-in-osu"));
        assert!(said.iter().any(|message| message.contains("OpenOsu")), "{lang:?}: {said:?}");
        assert!(shows(&main, &main.words.t("open-telegram")));
    }
    assert_eq!(bot::osu_link("https://bot.example", "K7QN-M4XZ"), "https://bot.example/render/pair/K7QNM4XZ/osu");
}

#[test]
fn an_account_without_telegram_is_asked_to_link_it_before_sending() {
    use dossier_native::main_screen::Pairing;
    for lang in Lang::ALL {
        let mut main = staged(lang, "main-prefs-bot-osu");
        assert!(!main.has_telegram());
        assert!(shows(&main, &main.words.t("account-osu")));
        assert!(shows(&main, &main.words.t("no-telegram-chats")));
        let said = clicked(&main, &main.words.t("link-telegram"));
        assert!(said.iter().any(|message| message.contains("LinkTelegram")), "{lang:?}: {said:?}");
        let _ = main.update(M::SendVideo);
        assert_eq!(main.pairing, Pairing::Asking);
        assert!(main.sending.is_none());
        let _ = main.update(M::TelegramAsked(Ok(("K7QNM4XZ".into(), "https://t.me/bot?start=pair-K7QNM4XZ".into()))));
        assert!(matches!(main.pairing, Pairing::Linking { .. }));
        assert!(shows(&main, &main.words.t("link-telegram-how")));
        assert!(!shows(&main, &main.words.t("sign-in-osu")));
        let linked = bot::Me { telegram_id: 7, name: "Naum".into(), username: "naumredlo".into(), avatar: false, telegram: true, player: Some(1) };
        let _ = main.update(M::Known(Ok(linked)));
        assert_eq!(main.pairing, Pairing::Idle);
        assert!(main.has_telegram());
        assert!(!shows(&main, &main.words.t("no-telegram-chats")));
    }
}

#[test]
fn a_received_video_is_not_offered_to_a_telegram_that_is_not_there() {
    let mut main = staged(Lang::En, "main-videos-received-open");
    assert!(!shows(&main, &main.words.t("received-telegram")), "a video kept in the application is not offered to Telegram");
    for video in &mut main.sharing.videos {
        video.storage = "telegram".to_owned();
    }
    assert!(shows(&main, &main.words.t("received-telegram")));
    main.account = Some(bot::Me { telegram_id: 0, name: "NaumRedlo".into(), username: String::new(), avatar: false, telegram: false, player: Some(1) });
    assert!(!shows(&main, &main.words.t("received-telegram")));
    assert!(shows(&main, &main.words.t("received-draw")));
    main.sharing.open = None;
    main.open_video = Some(0);
    let _ = main.update(M::Sharing(S::Pick));
    assert!(main.sharing.picker.is_some(), "a video goes through Dossier, so it does not wait for Telegram");
    assert_ne!(main.pairing, dossier_native::main_screen::Pairing::Asking);
}

#[test]
fn what_the_bot_says_about_an_account_is_read_with_and_without_telegram() {
    let old: bot::Me = serde_json::from_str(r#"{"telegram_id": 7, "name": "Naum", "username": "naumredlo", "avatar": true}"#).unwrap();
    assert!(old.telegram && old.player.is_none());
    let apart: bot::Me = serde_json::from_str(r#"{"telegram_id": 0, "name": "alice", "username": "", "avatar": false, "telegram": false, "player": 5}"#).unwrap();
    assert!(!apart.telegram && apart.player == Some(5));
    let pairing: bot::Pairing = serde_json::from_str(r#"{"code": "K7QN-M4XZ", "link": "", "expires_in": 600, "osu": true}"#).unwrap();
    assert!(pairing.osu);
    let before: bot::Pairing = serde_json::from_str(r#"{"code": "K7QN-M4XZ"}"#).unwrap();
    assert!(!before.osu);
}
