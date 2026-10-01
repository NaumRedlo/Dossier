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
