mod which_finger {
    use dossier_replay::{Keys, ReplayFrame};
    use dossier_sim::CursorTrack;

    fn track(frames: &[(i64, u8)]) -> CursorTrack {
        CursorTrack::new(
            frames
                .iter()
                .map(|&(time_ms, keys)| ReplayFrame {
                    time_ms,
                    x: 256.0,
                    y: 192.0,
                    keys: Keys(keys),
                })
                .collect(),
        )
    }

    fn held(track: &CursorTrack, lazer: bool) -> [usize; 4] {
        let spans = track.holds_each(lazer);
        [
            spans[0].len(),
            spans[1].len(),
            spans[2].len(),
            spans[3].len(),
        ]
    }

    #[test]
    fn stable_says_which_and_is_taken_at_its_word() {
        let stable = track(&[(0, 0), (10, Keys::K1 | Keys::M1), (20, 0)]);
        assert_eq!(held(&stable, false), [1, 0, 0, 0]);
    }

    #[test]
    fn a_stable_play_on_the_mouse_alone_still_reads_as_the_mouse() {
        let mouse = track(&[(0, 0), (10, Keys::M1), (20, 0), (30, Keys::M2), (40, 0)]);
        assert_eq!(held(&mouse, false), [0, 0, 1, 1]);
    }

    #[test]
    fn lazer_never_says_and_is_not_read_as_saying_the_mouse() {
        let lazer = track(&[
            (0, 0),
            (10, Keys::M1),
            (20, 0),
            (30, Keys::M2),
            (40, 0),
            (50, Keys::M1),
            (60, 0),
        ]);
        assert_eq!(
            held(&lazer, true),
            [2, 1, 0, 0],
            "the actions belong in the key lanes"
        );
    }

    #[test]
    fn a_play_with_no_presses_at_all_says_nothing_either_way() {
        let quiet = track(&[(0, 0), (10, 0), (20, 0)]);
        assert_eq!(held(&quiet, false), [0, 0, 0, 0]);
        assert_eq!(held(&quiet, true), [0, 0, 0, 0]);
    }

    #[test]
    fn a_stable_play_that_uses_both_keeps_them_apart() {
        let mixed = track(&[
            (0, 0),
            (10, Keys::M1),
            (20, 0),
            (30, Keys::K1 | Keys::M1),
            (40, 0),
        ]);
        assert_eq!(held(&mixed, false), [1, 0, 1, 0]);
    }
}
