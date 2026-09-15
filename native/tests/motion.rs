use std::time::{Duration, Instant};

use dossier_native::first_run::{FirstRun, Message, REVEAL};
use dossier_native::gallery;
use iced::Size;

fn settle(flow: &mut FirstRun) {
    std::thread::sleep(REVEAL + Duration::from_millis(80));
    let _ = flow.update(Message::Tick(Instant::now()));
}

#[test]
fn a_page_turn_moves_for_a_moment_and_then_rests() {
    let (mut flow, _) = FirstRun::new();
    let _ = flow.update(Message::Looked(vec![]));
    let _ = flow.update(Message::Tick(Instant::now()));
    assert!(flow.moving(), "a fresh page reveals itself");
    settle(&mut flow);
    assert!(!flow.moving(), "a revealed page asks for no more frames");
    let _ = flow.update(Message::Continue);
    let _ = flow.update(Message::Tick(Instant::now()));
    assert!(flow.moving(), "turning the page starts the reveal again");
    settle(&mut flow);
    assert!(!flow.moving());
}

#[test]
fn the_reveal_changes_the_picture_and_then_stops_changing_it() {
    let (mut flow, _) = FirstRun::new();
    let _ = flow.update(Message::Looked(vec![]));
    let _ = flow.update(Message::Continue);
    let _ = flow.update(Message::Tick(Instant::now()));
    let size = Size::new(980.0, 720.0);
    let early = gallery::snapshot(&flow, size).expect("a frame");
    let stem = std::env::temp_dir().join("dossier-motion-early");
    let _ = std::fs::remove_file(gallery::written_as(&stem));
    early.matches_image(&stem).expect("written");
    settle(&mut flow);
    let late = gallery::snapshot(&flow, size).expect("a frame");
    assert!(!late.matches_image(&stem).expect("compared"), "the settled frame differs from the first one");
    let again = gallery::snapshot(&flow, size).expect("a frame");
    let rest = std::env::temp_dir().join("dossier-motion-rest");
    let _ = std::fs::remove_file(gallery::written_as(&rest));
    late.matches_image(&rest).expect("written");
    assert!(again.matches_image(&rest).expect("compared"), "once settled, the frame holds still");
}
