use std::collections::BTreeSet;

use crate::memory::Memory;
use crate::stable::{self, Anchors, Mode};

#[derive(Debug, Default)]
pub struct Watch {
    rounds: u32,
    unread: u32,
    screens: BTreeSet<String>,
    health: Option<(f64, f64)>,
    flag: [u32; 3],
    emptied: bool,
    last: String,
}

impl Watch {
    pub fn observe(&mut self, memory: &dyn Memory, anchors: &Anchors) -> Option<String> {
        self.rounds += 1;
        let Some(seen) = stable::glance(memory, anchors) else {
            self.unread += 1;
            return None;
        };
        let screen = seen.mode.map_or_else(|| format!("Other{}", seen.raw_mode), |mode| format!("{mode:?}"));
        self.screens.insert(screen.clone());
        let health = if seen.mode == Some(Mode::Play) { stable::health(memory, anchors) } else { None };
        if let Some(health) = health {
            let (low, high) = self.health.unwrap_or((health, health));
            self.health = Some((low.min(health), high.max(health)));
            self.emptied |= health <= 0.0;
        }
        self.flag[match seen.watching {
            Some(false) => 0,
            Some(true) => 1,
            None => 2,
        }] += 1;
        let map = seen.map.as_ref().map_or_else(|| "none".to_owned(), |map| format!("{} [{}]", map.title, map.version));
        let health = health.map_or_else(|| "none".to_owned(), |health| format!("{health:.0}"));
        let flag = seen.watching.map_or("unreadable", |watching| if watching { "watching a replay" } else { "not watching" });
        let line = format!("screen {screen}, map {map}, health {health}, replay flag {flag}");
        (line != self.last).then(|| {
            self.last = line.clone();
            line
        })
    }

    pub fn summary(&self) -> Vec<String> {
        let screens: Vec<&str> = self.screens.iter().map(String::as_str).collect();
        let played = self.screens.contains("Play");
        vec![
            format!("readings: {}, unreadable: {}", self.rounds, self.unread),
            format!("screens seen: {}", if screens.is_empty() { "none".to_owned() } else { screens.join(", ") }),
            match (played, self.health) {
                (false, _) => "health: no play was seen".to_owned(),
                (true, None) => "health: a play was seen and the health bar could not be read".to_owned(),
                (true, Some((low, high))) => format!("health: read during play, from {low:.0} to {high:.0}{}", if self.emptied { ", and it ran out" } else { "" }),
            },
            format!("replay flag: not watching {} times, watching {} times, unreadable {} times", self.flag[0], self.flag[1], self.flag[2]),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stable::tests::{set_health, staged, with_frames, DATA};

    #[test]
    fn a_report_says_what_was_read_on_every_screen_and_what_could_not_be() {
        let (mut fake, anchors, _) = with_frames();
        set_health(&mut fake, 140.0);
        let mut watch = Watch::default();
        let first = watch.observe(&fake, &anchors).expect("a first line");
        assert!(first.contains("screen Play") && first.contains("health 140"), "{first}");
        assert!(watch.observe(&fake, &anchors).is_none(), "a reading that changed nothing says nothing");
        set_health(&mut fake, 0.0);
        assert!(watch.observe(&fake, &anchors).expect("the bar fell").contains("health 0"));
        fake.set_u32(DATA + 0x10, 5);
        assert!(watch.observe(&fake, &anchors).expect("the screen changed").contains("health none"));
        let said = watch.summary().join("\n");
        assert!(said.contains("health: read during play, from 0 to 140, and it ran out"), "{said}");
        assert!(said.contains("screens seen: Play, SelectPlay"), "{said}");
    }

    #[test]
    fn a_play_whose_bar_cannot_be_read_is_told_as_such() {
        let (fake, anchors, _) = with_frames();
        let mut watch = Watch::default();
        watch.observe(&fake, &anchors);
        assert!(watch.summary().iter().any(|line| line == "health: a play was seen and the health bar could not be read"));
    }

    #[test]
    fn a_client_that_was_never_at_a_play_has_no_health_to_report() {
        let (mut fake, anchors) = staged();
        fake.set_u32(DATA + 0x10, 5);
        let mut watch = Watch::default();
        watch.observe(&fake, &anchors);
        assert!(watch.summary().iter().any(|line| line == "health: no play was seen"));
    }
}
