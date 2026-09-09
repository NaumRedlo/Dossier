const NAME_YARDSTICK: &str = "-legusshhka-";

pub(super) fn name_size(name: &str, font: &crate::text::Font, size: f32) -> f32 {
    let room = font.width(NAME_YARDSTICK, size);
    let measured = font.width(name, size);
    if room <= 0.0 || measured <= room {
        return size;
    }
    size * room / measured
}

pub(super) fn compact(value: u64) -> String {
    const STEPS: [(u64, char); 3] = [(1_000_000_000, 'b'), (1_000_000, 'm'), (1_000, 'k')];
    if value < 10_000 {
        return grouped(value);
    }
    for (unit, suffix) in STEPS {
        if value >= unit {
            let scaled = value as f64 / unit as f64;

            let text = match scaled {
                s if s < 10.0 => format!("{s:.2}"),
                s if s < 100.0 => format!("{s:.1}"),
                s => format!("{s:.0}"),
            };
            return format!("{text}{suffix}");
        }
    }
    grouped(value)
}

pub(super) fn grouped(value: u64) -> String {
    let digits = value.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod names {
    use super::{name_size, NAME_YARDSTICK};

    fn font() -> crate::text::Font {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/fonts/Huninn-Regular.ttf"
        );
        let bytes = std::fs::read(path).expect("the repo ships this font");
        crate::text::Font::from_bytes(&bytes).expect("and it parses")
    }

    #[test]
    fn a_name_that_fits_is_left_alone() {
        let font = font();
        assert_eq!(name_size(NAME_YARDSTICK, &font, 20.0), 20.0);
        assert_eq!(name_size("sw1t", &font, 20.0), 20.0);
    }

    #[test]
    fn a_long_name_is_set_smaller_until_it_fits() {
        let font = font();
        let room = font.width(NAME_YARDSTICK, 20.0);
        for name in [
            "WWWWWWWWWWWWWWW",
            "Sakiko Togawa the second",
            "entxrth3vxid_2026",
        ] {
            let size = name_size(name, &font, 20.0);
            assert!(size < 20.0, "{name:?} was not shrunk");
            assert!(
                font.width(name, size) <= room + 1e-3,
                "{name:?} at {size} is still wider than the yardstick"
            );
        }
    }

    #[test]
    fn width_is_not_a_count_of_characters() {
        let font = font();
        let narrow = "iiiiiiiiiiiiiii";
        let wide = "WWWWWWWWWWWWWWW";
        assert_eq!(narrow.chars().count(), wide.chars().count());
        assert_eq!(name_size(narrow, &font, 20.0), 20.0);
        assert!(name_size(wide, &font, 20.0) < 20.0);
    }

    #[test]
    fn a_real_long_name_barely_shrinks() {
        let font = font();
        for name in ["Sakiko Togawa t", "entxrth3vxid_20"] {
            let factor = name_size(name, &font, 20.0) / 20.0;
            assert!(
                factor > 0.7,
                "{name:?} came out at {factor:.2} of the size — too small to sit in a list"
            );
        }
    }
}

#[cfg(test)]
mod compacting {
    use super::compact;

    #[test]
    fn a_score_is_said_in_three_figures_and_a_suffix() {
        assert_eq!(compact(1_234_567), "1.23m");
        assert_eq!(compact(12_345_678), "12.3m");
        assert_eq!(compact(125_645_112), "126m");
        assert_eq!(compact(987_654), "988k");
        assert_eq!(compact(87_340), "87.3k");
    }

    #[test]
    fn small_scores_are_left_as_they_are() {
        assert_eq!(compact(0), "0");
        assert_eq!(compact(950), "950");
        assert_eq!(compact(9_994), "9 994");
        assert_eq!(compact(10_000), "10.0k");
    }

    #[test]
    fn both_scoring_systems_come_out_the_same_width() {
        assert_eq!(compact(1_002_431).len(), compact(125_645_112).len() + 1);
        assert!(compact(125_645_112).len() <= 5);
        assert!(compact(1_002_431).len() <= 5);
    }
}

#[cfg(test)]
mod grouping {
    use super::grouped;

    #[test]
    fn digits_group_in_threes_from_the_right() {
        assert_eq!(grouped(0), "0");
        assert_eq!(grouped(999), "999");
        assert_eq!(grouped(1_000), "1 000");
        assert_eq!(grouped(317_279_960), "317 279 960");

        assert_eq!(grouped(12_345), "12 345");
    }
}
