use std::time::Instant;

use dossier_hud::hold::{Hold, Keys};
use dossier_hud::{Context, Lang, Outcome, Sprite, Stage};
use dossier_overlay::{Frame, Receiver, Screen, View, Viewport};

pub const SEND_KEY: &str = "Tab";

pub struct Hud {
    context: Context,
    view: Option<View>,
    stage: Stage,
    hold: Hold,
    offers: bool,
    last: Option<Instant>,
    receiver: Receiver,
    dirty: bool,
}

impl Hud {
    pub fn new(lang: Lang, offers: bool) -> Hud {
        Hud { context: Context { lang, ..Context::default() }, view: None, stage: Stage::default(), hold: Hold::default(), offers, last: None, receiver: Receiver::default(), dirty: false }
    }

    pub fn told(&mut self, frame: &Frame) -> bool {
        self.told_at(frame, Instant::now())
    }

    pub fn told_at(&mut self, frame: &Frame, now: Instant) -> bool {
        if self.receiver.accept(frame.clone(), now).is_err() {
            return false;
        }
        self.dirty = true;
        let Some(view) = self.receiver.visible(now).cloned() else {
            self.clear();
            return true;
        };
        let snapshot = &view.snapshot;
        let before = self.view.as_ref().map(|view| view.snapshot.screen);
        let restarted = self.view.as_ref().is_some_and(|old| {
            old.snapshot.beatmap != snapshot.beatmap ||
            old.snapshot.gameplay.as_ref().zip(snapshot.gameplay.as_ref())
                .is_some_and(|(old, new)| new.time_ms < old.time_ms.saturating_sub(500))
        });
        if restarted || (before != Some(snapshot.screen) && snapshot.screen != Screen::Results) {
            self.context.outcome = None;
            self.hold = Hold::default();
        }
        if snapshot.screen != Screen::Playing {
            self.context.play = None;
        }
        dossier_hud::told(&view, &mut self.context);
        self.view = Some(view);
        true
    }

    fn clear(&mut self) {
        self.view = None;
        self.context.play = None;
        self.context.outcome = None;
        self.hold = Hold::default();
        self.stage = Stage::default();
        self.dirty = true;
    }

    pub fn take_dirty(&mut self) -> bool {
        std::mem::take(&mut self.dirty)
    }

    pub fn kept(&mut self, passed: bool) {
        self.context.outcome = Some(Outcome { saved: true, offer: (self.offers && passed).then(|| SEND_KEY.to_owned()), holding: 0.0, sent: None });
    }

    pub fn step(&mut self, keys: Keys, now: Instant) -> bool {
        if self.view.is_some() && self.receiver.visible(now).is_none() {
            self.clear();
        }
        let seconds = self.last.map_or(0.0, |was| now.saturating_duration_since(was).as_secs_f32()).min(0.25);
        self.last = Some(now);
        self.stage.advance(self.view.as_ref(), &self.context, now);
        let results = self.view.as_ref().is_some_and(|view| view.snapshot.screen == Screen::Results);
        let Some(outcome) = self.context.outcome.as_mut().filter(|outcome| results && outcome.saved && outcome.offer.is_some() && outcome.sent.is_none()) else {
            self.hold = Hold::default();
            return false;
        };
        let fired = self.hold.advance(keys, seconds);
        outcome.holding = self.hold.share();
        if fired {
            outcome.offer = None;
        }
        fired
    }

    pub fn moving(&self) -> bool {
        self.stage.moving(self.view.as_ref(), &self.context) || self.hold.share() > 0.0
    }

    pub fn sprites(&self, viewport: Viewport) -> Vec<Sprite> {
        dossier_hud::sprites(self.view.as_ref(), &self.context, &self.stage, viewport)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dossier_overlay::{Client, Gameplay, Message, Meter, Snapshot, STALE_AFTER};
    use std::time::Duration;

    const VIEWPORT: Viewport = Viewport { width: 1280, height: 720, scale: 1.0 };
    const UP: Keys = Keys { key: false, other: false, front: true };
    const DOWN: Keys = Keys { key: true, other: false, front: true };

    fn frame(sequence: u64, screen: Screen, resting: bool) -> Frame {
        let gameplay = (screen == Screen::Playing).then(|| Gameplay {
            ruleset: 0,
            time_ms: 1000,
            score: 100,
            combo: 10,
            max_combo: 10,
            accuracy: Some(99.0),
            misses: 0,
            legacy_mods: Some(0),
            mods: Vec::new(),
            unstable_rate: Some(90.0),
            resting: Some(resting),
            pp: Some(41.5),
            pp_clean: Some(120.0),
        });
        Frame::new(1, sequence, Message::Snapshot { snapshot: Snapshot { screen, beatmap: None, gameplay, watching_replay: Some(false), meter: Some(Meter { shown: true, scale: 1.0 }) } })
    }

    fn connected(lang: Lang, offers: bool) -> Hud {
        let mut hud = Hud::new(lang, offers);
        hud.told(&Frame::new(1, 0, Message::Hello { client: Client::Stable, pid: 7, build: String::new() }));
        hud
    }

    #[test]
    fn lost_telemetry_hides_plates_and_cancels_a_pending_send() {
        let mut hud = connected(Lang::Ru, true);
        let now = Instant::now();
        hud.told_at(&frame(1, Screen::Results, false), now);
        hud.kept(true);
        hud.step(UP, now);
        hud.step(DOWN, now + Duration::from_millis(200));
        hud.take_dirty();
        assert!(!hud.step(DOWN, now + STALE_AFTER));
        assert!(keys(&hud).is_empty());
        assert!(hud.take_dirty());
        hud.told_at(&frame(2, Screen::Results, false), now + STALE_AFTER);
        assert_eq!(keys(&hud), ["tile"]);
        assert!(!hud.told_at(&frame(1, Screen::Playing, false), now + STALE_AFTER));
    }

    #[test]
    fn a_new_session_clears_the_previous_result_until_a_fresh_snapshot() {
        let mut hud = connected(Lang::Ru, true);
        hud.told(&frame(1, Screen::Results, false));
        hud.kept(true);
        assert_eq!(keys(&hud), ["tile", "offer"]);
        assert!(hud.told(&Frame::new(2, 0, Message::Hello { client: Client::Stable, pid: 8, build: String::new() })));
        assert!(keys(&hud).is_empty());
        assert!(!hud.told(&frame(2, Screen::Results, false)));
        let mut next = frame(1, Screen::Results, false);
        next.session = 2;
        assert!(hud.told(&next));
        assert_eq!(keys(&hud), ["tile"]);
    }

    fn keys(hud: &Hud) -> Vec<&'static str> {
        hud.sprites(VIEWPORT).iter().map(|sprite| sprite.key).collect()
    }

    fn run(hud: &mut Hud, held: Keys, from: Instant, seconds: f32) -> (usize, Instant) {
        let steps = (seconds / 0.016).round() as u32;
        let fired = (1..=steps).filter(|step| hud.step(held, from + Duration::from_millis(16) * *step)).count();
        (fired, from + Duration::from_millis(16) * steps)
    }

    #[test]
    fn nothing_is_shown_before_the_client_is_seen_and_a_play_brings_its_plates() {
        let mut hud = connected(Lang::Ru, false);
        assert!(keys(&hud).is_empty());
        hud.told(&frame(1, Screen::Menu, false));
        assert_eq!(keys(&hud), ["tile"]);
        hud.told(&frame(2, Screen::Playing, true));
        let (_, now) = run(&mut hud, UP, Instant::now(), 0.5);
        assert_eq!(keys(&hud), ["rate", "tile", "pp"]);
        assert!(hud.moving(), "the recording tile breathes during a play");
        hud.told(&frame(3, Screen::Selection, false));
        run(&mut hud, UP, now, 0.5);
        assert_eq!(keys(&hud), ["tile"]);
        assert!(!hud.moving());
        hud.told(&Frame::new(1, 4, Message::Disconnected));
        assert!(keys(&hud).is_empty());
    }

    #[test]
    fn a_kept_play_offers_to_send_and_a_bare_hold_takes_the_offer_once() {
        let mut hud = connected(Lang::Ru, true);
        hud.told(&frame(1, Screen::Playing, false));
        hud.kept(true);
        hud.told(&frame(2, Screen::Results, false));
        let (fired, now) = run(&mut hud, UP, Instant::now(), 0.2);
        assert_eq!(fired, 0);
        assert_eq!(keys(&hud), ["tile", "offer"]);
        let (fired, now) = run(&mut hud, DOWN, now, 0.3);
        assert_eq!(fired, 0);
        assert!(hud.moving(), "the bar is filling");
        let (fired, now) = run(&mut hud, DOWN, now, 1.0);
        assert_eq!(fired, 1);
        assert_eq!(keys(&hud), ["tile"], "the offer is taken");
        let (fired, _) = run(&mut hud, DOWN, now, 1.0);
        assert_eq!(fired, 0);
    }

    #[test]
    fn a_failed_play_and_a_hud_without_offers_keep_the_play_and_offer_nothing() {
        for (offers, passed) in [(true, false), (false, true)] {
            let mut hud = connected(Lang::En, offers);
            hud.told(&frame(1, Screen::Playing, false));
            hud.told(&frame(2, Screen::Results, false));
            hud.kept(passed);
            let (fired, _) = run(&mut hud, DOWN, Instant::now(), 1.0);
            assert_eq!((fired, keys(&hud)), (0, vec!["tile"]));
        }
    }

    #[test]
    fn the_outcome_of_one_play_does_not_follow_the_player_to_another_screen() {
        let mut hud = connected(Lang::Ru, true);
        hud.told(&frame(1, Screen::Playing, false));
        hud.told(&frame(2, Screen::Results, false));
        hud.kept(true);
        assert_eq!(keys(&hud), ["tile", "offer"]);
        hud.told(&frame(3, Screen::Selection, false));
        hud.told(&frame(4, Screen::Results, false));
        assert_eq!(keys(&hud), ["tile"], "an older score opened from song select is not this play");
    }
}
