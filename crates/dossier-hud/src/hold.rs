use crate::HOLD_SECONDS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Keys {
    pub key: bool,
    pub other: bool,
    pub front: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Hold {
    held: f32,
    armed: bool,
    spent: bool,
}

impl Hold {
    pub fn share(&self) -> f32 {
        (self.held / HOLD_SECONDS).clamp(0.0, 1.0)
    }

    pub fn advance(&mut self, keys: Keys, seconds: f32) -> bool {
        if !keys.front {
            *self = Hold::default();
            return false;
        }
        if !keys.key {
            *self = Hold {
                armed: true,
                ..Hold::default()
            };
            return false;
        }
        if keys.other {
            *self = Hold::default();
            return false;
        }
        if !self.armed || self.spent {
            return false;
        }
        self.held += seconds.max(0.0);
        if self.held >= HOLD_SECONDS {
            self.spent = true;
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const UP: Keys = Keys {
        key: false,
        other: false,
        front: true,
    };
    const DOWN: Keys = Keys {
        key: true,
        other: false,
        front: true,
    };

    fn run(hold: &mut Hold, keys: Keys, seconds: f32) -> usize {
        let steps = (seconds / 0.016).round() as usize;
        (0..steps).filter(|_| hold.advance(keys, 0.016)).count()
    }

    #[test]
    fn a_bare_hold_of_half_a_second_sends_once_and_fills_the_bar_on_the_way() {
        let mut hold = Hold::default();
        assert_eq!(run(&mut hold, UP, 0.1), 0);
        assert_eq!(run(&mut hold, DOWN, 0.25), 0);
        assert!((hold.share() - 0.5).abs() < 0.05, "{}", hold.share());
        assert_eq!(run(&mut hold, DOWN, 2.0), 1, "a long hold sends once");
        assert_eq!(hold.share(), 1.0);
        assert_eq!(run(&mut hold, UP, 0.1), 0);
        assert_eq!(hold.share(), 0.0);
        assert_eq!(run(&mut hold, DOWN, 0.6), 1, "a second hold sends again");
    }

    #[test]
    fn a_short_press_sends_nothing() {
        let mut hold = Hold::default();
        run(&mut hold, UP, 0.1);
        for _ in 0..6 {
            assert_eq!(run(&mut hold, DOWN, 0.3), 0);
            assert_eq!(run(&mut hold, UP, 0.05), 0);
        }
    }

    #[test]
    fn leaving_through_alt_tab_sends_nothing_however_long_the_keys_stay_down() {
        let mut hold = Hold::default();
        run(&mut hold, UP, 0.1);
        let alt = Keys {
            other: true,
            ..UP
        };
        let alt_tab = Keys {
            other: true,
            ..DOWN
        };
        assert_eq!(run(&mut hold, alt, 0.2), 0);
        assert_eq!(run(&mut hold, alt_tab, 1.0), 0);
        assert_eq!(hold.share(), 0.0);
        assert_eq!(run(&mut hold, DOWN, 1.0), 0, "letting Alt go first does not turn it into a hold");
        let away = Keys {
            front: false,
            ..DOWN
        };
        assert_eq!(run(&mut hold, away, 1.0), 0);
        assert_eq!(run(&mut hold, DOWN, 1.0), 0, "a key already down on the way back does not count");
        run(&mut hold, UP, 0.05);
        assert_eq!(run(&mut hold, DOWN, 0.6), 1);
    }

    #[test]
    fn a_modifier_pressed_during_the_hold_spoils_it_until_the_key_is_let_go() {
        let mut hold = Hold::default();
        run(&mut hold, UP, 0.1);
        assert_eq!(run(&mut hold, DOWN, 0.3), 0);
        let shifted = Keys {
            other: true,
            ..DOWN
        };
        assert_eq!(run(&mut hold, shifted, 0.05), 0);
        assert_eq!(run(&mut hold, DOWN, 1.0), 0);
        assert_eq!(hold.share(), 0.0);
    }

    #[test]
    fn the_game_losing_the_front_in_the_middle_of_a_hold_cancels_it() {
        let mut hold = Hold::default();
        run(&mut hold, UP, 0.1);
        assert_eq!(run(&mut hold, DOWN, 0.4), 0);
        let away = Keys {
            front: false,
            ..DOWN
        };
        assert_eq!(run(&mut hold, away, 0.05), 0);
        assert_eq!(run(&mut hold, DOWN, 1.0), 0);
    }
}
