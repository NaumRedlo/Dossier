use std::time::{Duration, Instant};

use crate::community_screen::{Message as C, Section};
use crate::main_screen::{Message, Overlay};

const PACE: Duration = Duration::from_millis(1500);
const WEIGH_EVERY: Duration = Duration::from_secs(30);
const PEOPLE: usize = 12;
const REPLAYS: usize = 24;
const VIDEOS: usize = 4;
const VIDEO_WATCHED: usize = 8;
const FEED_PAGES: usize = 10;
const FEED_PAGE_HIGH: f32 = 2400.0;

pub fn round(turn: usize) -> Vec<Message> {
    let community = |inner: C| Message::Community(inner);
    let mut steps = vec![Message::Show(Overlay::Community), community(C::Section(Section::Feed))];
    for page in 1..=FEED_PAGES {
        steps.push(community(C::FeedMore(page as f32 * FEED_PAGE_HIGH)));
    }
    steps.push(community(C::ShowNew));
    steps.push(community(C::Section(Section::People)));
    for person in 0..PEOPLE {
        steps.push(community(C::Person(Some((turn * PEOPLE + person) % (PEOPLE * 4)))));
        steps.push(community(C::Person(None)));
    }
    for section in [Section::Boards, Section::Titles, Section::Pools, Section::Duels, Section::Balance, Section::Profile] {
        steps.push(community(C::Section(section)));
        steps.push(Message::SoakRest);
    }
    steps.push(Message::Show(Overlay::None));
    steps.extend(std::iter::repeat_n(Message::Step(1), REPLAYS));
    steps.push(Message::Show(Overlay::Videos));
    for video in 0..VIDEOS {
        steps.push(Message::OpenVideo((turn * VIDEOS + video) % (VIDEOS * 4)));
        steps.extend(std::iter::repeat_n(Message::SoakRest, VIDEO_WATCHED));
        steps.push(Message::ClosePlayer);
    }
    steps.push(Message::Show(Overlay::Settings));
    steps.push(Message::Show(Overlay::None));
    steps
}

pub fn run(minutes: u64, push: &mut dyn FnMut(Message) -> bool) {
    let began = Instant::now();
    let until = Duration::from_secs(minutes * 60);
    let mut weighed = Instant::now() - WEIGH_EVERY;
    let mut turn = 0;
    while began.elapsed() < until {
        for step in round(turn) {
            if weighed.elapsed() >= WEIGH_EVERY {
                weighed = Instant::now();
                if !push(Message::SoakWeigh(began.elapsed().as_secs(), turn)) {
                    return;
                }
            }
            std::thread::sleep(PACE);
            if !push(step) {
                return;
            }
        }
        turn += 1;
    }
    let _ = push(Message::SoakWeigh(began.elapsed().as_secs(), turn));
    let _ = push(Message::SoakOver);
}

pub fn line(seconds: u64, turn: usize, held: &[(&'static str, usize, usize)]) -> String {
    let mut said = format!("soak t={seconds} turn={turn}");
    for (name, count, bytes) in held {
        said.push_str(&format!(" {name}={count}/{}", bytes / 1024));
    }
    said
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_round_visits_the_feed_people_replays_and_videos_and_moves_on_each_turn() {
        let first = round(0);
        let named = |steps: &[Message]| steps.iter().map(|step| format!("{step:?}")).collect::<Vec<_>>();
        let said = named(&first);
        for wanted in ["Show(Community)", "Community(FeedMore(2400.0))", "Community(Person(Some(0)))", "Community(Section(Balance))", "Step(1)", "Show(Videos)", "OpenVideo(0)", "ClosePlayer", "Show(Settings)"] {
            assert!(said.iter().any(|step| step == wanted), "{wanted} is missing");
        }
        assert_eq!(said.last().map(String::as_str), Some("Show(None)"), "a round ends where the next one starts");
        let second = named(&round(1));
        assert!(second.iter().any(|step| step == "Community(Person(Some(12)))") && second.iter().any(|step| step == "OpenVideo(4)"), "the next round opens other people and videos");
    }

    #[test]
    fn a_weighing_is_one_line_with_counts_and_kilobytes() {
        assert_eq!(line(90, 2, &[("thumbs", 3, 4096), ("flags", 7, 0)]), "soak t=90 turn=2 thumbs=3/4 flags=7/0");
    }
}
