use std::time::{Duration, Instant};

use dossier_hud::context::Packet;
use dossier_hud::hold::{Asked, Chord, Chords, Hold, Keys};
use dossier_hud::{Context, Lang, Look, Outcome, Panel, Sprite, Stage};
use dossier_overlay::{Frame, Message, Receiver, Screen, View, Viewport};

pub const SEND_KEY: &str = "Tab";
pub const TUNING_FOR: Duration = Duration::from_secs(8);
pub const HIDDEN_NOTE_FOR: Duration = Duration::from_secs(3);
pub const OWN_SCALE_FOR: Duration = Duration::from_secs(3);
pub const GREET_FOR: Duration = Duration::from_secs(5);
pub const TICK_FOR: Duration = Duration::from_secs(3);

pub struct Hud {
    context: Context,
    view: Option<View>,
    stage: Stage,
    hold: Hold,
    offers: bool,
    last: Option<Instant>,
    receiver: Receiver,
    dirty: bool,
    packet: Option<Packet>,
    chords: Chords,
    hidden: bool,
    panel: Option<(Panel, Instant)>,
    own_scale: Option<(u16, Instant)>,
    told_scale: Option<u16>,
    greeted: bool,
    greeting: Option<Instant>,
    saved_at: Option<Instant>,
}

impl Hud {
    pub fn new(lang: Lang, offers: bool) -> Hud {
        Hud { context: Context { lang, ..Context::default() }, view: None, stage: Stage::default(), hold: Hold::default(), offers, last: None, receiver: Receiver::default(), dirty: false, packet: None, chords: Chords::default(), hidden: false, panel: None, own_scale: None, told_scale: None, greeted: false, greeting: None, saved_at: None }
    }

    pub fn told(&mut self, frame: &Frame) -> bool {
        self.told_at(frame, Instant::now())
    }

    pub fn told_at(&mut self, frame: &Frame, now: Instant) -> bool {
        if self.receiver.accept(frame.clone(), now).is_err() || matches!(frame.message, Message::Kept { .. }) {
            return false;
        }
        if matches!(frame.message, Message::Hello { .. }) {
            self.greeted = false;
            self.greeting = None;
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
        if !self.greeted {
            self.greeted = true;
            self.greeting = Some(now);
        }
        true
    }

    fn clear(&mut self) {
        self.view = None;
        self.context.play = None;
        self.context.outcome = None;
        self.hold = Hold::default();
        self.stage = Stage::default();
        self.saved_at = None;
        self.dirty = true;
    }

    pub fn heard(&mut self, packet: Packet) {
        self.packet = Some(packet);
        self.dirty = true;
    }

    fn look(&self) -> Look {
        let mut look = self.packet.as_ref().and_then(|packet| packet.look).unwrap_or_default();
        if let Some((scale, _)) = self.own_scale {
            look.scale = scale;
        }
        look
    }

    pub fn chord(&mut self, chord: Chord, front: bool, now: Instant) {
        let Some(asked) = self.chords.advance(chord, front) else {
            return;
        };
        let playing = self.view.as_ref().is_some_and(|view| view.snapshot.screen == Screen::Playing);
        match asked {
            Asked::Hide => {
                self.hidden = !self.hidden;
                self.panel = self.hidden.then_some((Panel::Hidden, now));
            }
            _ if playing || self.hidden => return,
            Asked::Open => {
                self.panel = match self.panel {
                    Some((Panel::Tuning, _)) => None,
                    _ => Some((Panel::Tuning, now)),
                };
            }
            Asked::Bigger | Asked::Smaller => {
                let scale = dossier_hud::scale_step(self.look().scale, if asked == Asked::Bigger { 1 } else { -1 });
                self.own_scale = Some((scale, now));
                self.told_scale = Some(scale);
                self.panel = Some((Panel::Tuning, now));
            }
        }
        self.dirty = true;
    }

    pub fn take_scale(&mut self) -> Option<u16> {
        self.told_scale.take()
    }

    fn settle(&mut self, now: Instant) {
        let lasting = match self.panel {
            Some((Panel::Tuning, _)) => TUNING_FOR,
            _ => HIDDEN_NOTE_FOR,
        };
        if self.panel.is_some_and(|(_, at)| now.saturating_duration_since(at) >= lasting) {
            self.panel = None;
            self.dirty = true;
        }
        let told = self.packet.as_ref().and_then(|packet| packet.look).map(|look| look.scale);
        if self.own_scale.is_some_and(|(scale, at)| told == Some(scale) || now.saturating_duration_since(at) >= OWN_SCALE_FOR) {
            self.own_scale = None;
            self.dirty = true;
        }
        if self.greeting.is_some_and(|at| now.saturating_duration_since(at) >= GREET_FOR) {
            self.greeting = None;
            self.dirty = true;
        }
        if self.saved_at.is_some_and(|at| now.saturating_duration_since(at) >= TICK_FOR) {
            self.saved_at = None;
            self.dirty = true;
        }
    }

    fn shown(&self) -> Context {
        let mut shown = self.context.clone();
        shown.look = self.look();
        shown.hidden = self.hidden;
        shown.panel = self.panel.map(|(panel, _)| panel);
        let told = self.packet.as_ref().zip(self.view.as_ref()).and_then(|(packet, view)| packet.context(view, dossier_hud::context::now_ms()));
        shown.greet = self.greeting.is_some();
        shown.ticked = self.saved_at.is_some();
        if let Some(told) = told {
            shown.card = told.card;
            shown.notices = told.notices;
        }
        shown
    }

    pub fn take_dirty(&mut self) -> bool {
        std::mem::take(&mut self.dirty)
    }

    pub fn kept(&mut self, passed: bool) {
        self.kept_at(passed, Instant::now());
    }

    pub fn kept_at(&mut self, passed: bool, now: Instant) {
        self.context.outcome = Some(Outcome { saved: true, offer: (self.offers && passed).then(|| SEND_KEY.to_owned()), holding: 0.0 });
        self.saved_at = Some(now);
        self.dirty = true;
    }

    pub fn step(&mut self, keys: Keys, now: Instant) -> bool {
        if self.view.is_some() && self.receiver.visible(now).is_none() {
            self.clear();
        }
        self.settle(now);
        let seconds = self.last.map_or(0.0, |was| now.saturating_duration_since(was).as_secs_f32()).min(0.25);
        self.last = Some(now);
        let shown = self.shown();
        self.stage.advance(self.view.as_ref(), &shown, now);
        let results = self.view.as_ref().is_some_and(|view| view.snapshot.screen == Screen::Results);
        let Some(outcome) = self.context.outcome.as_mut().filter(|outcome| !self.hidden && results && outcome.saved && outcome.offer.is_some()) else {
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
        self.stage.moving(self.view.as_ref(), &self.shown()) || self.hold.share() > 0.0
    }

    pub fn sprites(&self, viewport: Viewport) -> Vec<Sprite> {
        dossier_hud::sprites(self.view.as_ref(), &self.shown(), &self.stage, viewport)
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
            pp_record: None,
        });
        Frame::new(1, sequence, Message::Snapshot { snapshot: Snapshot { screen, beatmap: None, gameplay, watching_replay: Some(false), meter: Some(Meter { shown: true, scale: 1.0 }), display: None } })
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

    fn packet() -> Packet {
        Packet {
            version: 1, pid: 7, at_ms: dossier_hud::context::now_ms(), lang: Lang::Ru, map_md5: None, card: None,
            notices: vec![dossier_hud::Notice { title: "Видео готово".into(), ..dossier_hud::Notice::default() }], look: None,
        }
    }

    #[test]
    fn dossier_tells_the_notices_for_the_menu_and_a_sent_play_leaves_no_plate() {
        let mut hud = connected(Lang::Ru, true);
        hud.told(&frame(1, Screen::Menu, false));
        assert_eq!(keys(&hud), ["tile"]);
        hud.heard(packet());
        assert!(hud.take_dirty());
        assert_eq!(keys(&hud), ["notice-1", "tile"]);
        hud.told(&frame(2, Screen::Results, false));
        hud.kept(true);
        let now = Instant::now();
        hud.step(UP, now);
        let (fired, _) = run(&mut hud, DOWN, now, 1.0);
        assert_eq!(fired, 1);
        hud.heard(packet());
        assert_eq!(keys(&hud), ["tile"]);
    }

    #[test]
    fn the_greeting_comes_once_a_session_and_the_tick_stays_three_seconds() {
        let mut hud = connected(Lang::Ru, false);
        let now = Instant::now();
        hud.told_at(&frame(1, Screen::Menu, false), now);
        assert!(hud.shown().greet);
        let later = now + GREET_FOR;
        hud.told_at(&frame(2, Screen::Menu, false), later);
        hud.step(UP, later);
        assert!(!hud.shown().greet);
        hud.told_at(&frame(3, Screen::Selection, false), later);
        assert!(!hud.shown().greet, "it does not come back on another screen");
        hud.kept_at(true, later);
        assert!(hud.shown().ticked);
        hud.told_at(&frame(4, Screen::Results, false), later + TICK_FOR);
        hud.step(UP, later + TICK_FOR);
        assert!(!hud.shown().ticked);
        hud.told_at(&Frame::new(2, 0, Message::Hello { client: Client::Stable, pid: 8, build: String::new() }), later + TICK_FOR);
        let mut next = frame(1, Screen::Menu, false);
        next.session = 2;
        hud.told_at(&next, later + TICK_FOR);
        assert!(hud.shown().greet, "a new game greets again");
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

    const CHORD: Chord = Chord { ctrl_shift: true, open: false, hide: false, bigger: false, smaller: false };

    fn press(hud: &mut Hud, chord: Chord, now: Instant) {
        hud.chord(CHORD, true, now);
        hud.chord(chord, true, now);
        hud.chord(CHORD, true, now);
    }

    #[test]
    fn the_keys_open_the_tuning_panel_change_the_scale_and_tell_dossier_once() {
        let mut hud = connected(Lang::Ru, false);
        let now = Instant::now();
        hud.told_at(&frame(1, Screen::Menu, false), now);
        press(&mut hud, Chord { open: true, ..CHORD }, now);
        assert_eq!(keys(&hud), ["tuning", "tile"]);
        press(&mut hud, Chord { bigger: true, ..CHORD }, now);
        assert_eq!(hud.take_scale(), Some(115));
        assert_eq!(hud.take_scale(), None);
        assert_eq!(hud.shown().look.scale, 115, "the overlay follows its own word until Dossier answers");
        let mut told = packet();
        told.look = Some(Look { scale: 100, ..Look::default() });
        hud.heard(told.clone());
        hud.step(UP, now + Duration::from_secs(1));
        assert_eq!(hud.shown().look.scale, 115, "an older packet does not take the new scale back");
        told.look = Some(Look { scale: 115, ..Look::default() });
        hud.heard(told);
        hud.step(UP, now + Duration::from_secs(2));
        assert_eq!(hud.shown().look.scale, 115);
        assert!(hud.own_scale.is_none());
        let later = now + TUNING_FOR + Duration::from_secs(1);
        hud.told_at(&frame(2, Screen::Menu, false), later);
        hud.step(UP, later);
        assert_eq!(keys(&hud), ["notice-1", "tile"], "the panel closes by itself");
    }

    #[test]
    fn a_play_takes_no_tuning_but_the_plates_can_be_hidden_and_brought_back() {
        let mut hud = connected(Lang::Ru, true);
        let now = Instant::now();
        hud.told_at(&frame(1, Screen::Playing, false), now);
        let (_, now) = run(&mut hud, UP, now, 0.5);
        press(&mut hud, Chord { bigger: true, ..CHORD }, now);
        press(&mut hud, Chord { open: true, ..CHORD }, now);
        assert_eq!(hud.take_scale(), None);
        assert_eq!(keys(&hud), ["rate", "tile", "pp"]);
        press(&mut hud, Chord { hide: true, ..CHORD }, now);
        assert_eq!(keys(&hud), ["hidden"]);
        let later = now + HIDDEN_NOTE_FOR + Duration::from_millis(100);
        hud.told_at(&frame(2, Screen::Results, false), later);
        hud.step(UP, later);
        assert!(keys(&hud).is_empty());
        hud.kept(true);
        let (fired, later) = run(&mut hud, DOWN, later, 1.0);
        assert_eq!(fired, 0, "a hidden offer sends nothing");
        hud.told_at(&frame(3, Screen::Results, false), later);
        press(&mut hud, Chord { hide: true, ..CHORD }, later);
        assert_eq!(keys(&hud), ["tile", "offer"]);
    }
}
