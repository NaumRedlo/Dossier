use fluent_bundle::{FluentArgs, FluentBundle, FluentResource, FluentValue};
use unic_langid::{langid, LanguageIdentifier};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Lang {
    En,
    Ru,
}

impl Lang {
    pub const ALL: [Lang; 2] = [Lang::En, Lang::Ru];

    pub fn from_system() -> Lang {
        let said = sys_locale::get_locale().unwrap_or_default().to_lowercase();
        if said.starts_with("ru") {
            Lang::Ru
        } else {
            Lang::En
        }
    }

    pub fn tag(self) -> &'static str {
        match self {
            Lang::En => "en-US",
            Lang::Ru => "ru-RU",
        }
    }

    pub fn own_name(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::Ru => "Русский",
        }
    }

    fn id(self) -> LanguageIdentifier {
        match self {
            Lang::En => langid!("en-US"),
            Lang::Ru => langid!("ru-RU"),
        }
    }

    fn source(self) -> &'static str {
        match self {
            Lang::En => include_str!("../lang/en-US.ftl"),
            Lang::Ru => include_str!("../lang/ru-RU.ftl"),
        }
    }

    pub fn group(self, n: u64) -> String {
        let digits = n.to_string();
        let separator = match self {
            Lang::En => ",",
            Lang::Ru => "\u{a0}",
        };
        let mut out = String::new();
        for (i, ch) in digits.chars().enumerate() {
            if i > 0 && (digits.len() - i) % 3 == 0 {
                out.push_str(separator);
            }
            out.push(ch);
        }
        out
    }
}

pub struct Words {
    lang: Lang,
    bundle: FluentBundle<FluentResource>,
    before: Option<(FluentBundle<FluentResource>, f32)>,
    zone: Option<chrono::FixedOffset>,
}

impl Clone for Words {
    fn clone(&self) -> Words {
        Words { zone: self.zone, ..Words::new(self.lang) }
    }
}

fn bundle_for(lang: Lang) -> FluentBundle<FluentResource> {
    let resource = FluentResource::try_new(lang.source().to_owned())
        .unwrap_or_else(|(resource, _)| resource);
    let mut bundle = FluentBundle::new(vec![lang.id()]);
    bundle.set_use_isolating(false);
    let _ = bundle.add_resource(resource);
    bundle
}

pub const ERASING: f32 = 0.4;

pub fn typed(from: &str, to: &str, k: f32) -> String {
    let k = k.clamp(0.0, 1.0);
    if k < ERASING {
        let left = 1.0 - k / ERASING;
        let kept = (from.chars().count() as f32 * left).round() as usize;
        from.chars().take(kept).collect()
    } else {
        let done = (k - ERASING) / (1.0 - ERASING);
        let shown = (to.chars().count() as f32 * done).round() as usize;
        to.chars().take(shown).collect()
    }
}

impl Words {
    pub fn new(lang: Lang) -> Words {
        Words { lang, bundle: bundle_for(lang), before: None, zone: None }
    }

    pub fn in_zone(mut self, seconds_east: i32) -> Words {
        self.zone = chrono::FixedOffset::east_opt(seconds_east);
        self
    }

    fn local(&self, unix: i64) -> Option<chrono::DateTime<chrono::FixedOffset>> {
        use chrono::TimeZone;
        match self.zone {
            Some(zone) => zone.timestamp_opt(unix, 0).single(),
            None => chrono::Local.timestamp_opt(unix, 0).single().map(|t| t.fixed_offset()),
        }
    }

    pub fn retyping_into(self, lang: Lang) -> Words {
        let before = match self.before {
            Some((old, _)) if self.lang == lang => old,
            _ => self.bundle,
        };
        Words { lang, bundle: bundle_for(lang), before: Some((before, 0.0)), zone: self.zone }
    }

    pub fn typed_up_to(&mut self, k: f32) {
        if let Some((_, progress)) = &mut self.before {
            *progress = k.clamp(0.0, 1.0);
        }
    }

    pub fn settle(&mut self) {
        self.before = None;
    }

    pub fn lang(&self) -> Lang {
        self.lang
    }

    pub fn t(&self, key: &str) -> String {
        self.say(key, None)
    }

    pub fn n(&self, key: &str, n: u64) -> String {
        let mut args = FluentArgs::new();
        args.set("n", FluentValue::from(n));
        self.say(key, Some(&args))
    }

    pub fn of(&self, n: u64, total: u64) -> String {
        let mut args = FluentArgs::new();
        args.set("n", FluentValue::from(n));
        args.set("total", FluentValue::from(total));
        self.say("n-of", Some(&args))
    }

    pub fn who(&self, key: &str, who: &str) -> String {
        let mut args = FluentArgs::new();
        args.set("who", FluentValue::from(who));
        self.say(key, Some(&args))
    }

    pub fn count(&self, key: &str, n: u64) -> String {
        format!("{} {}", self.lang.group(n), self.n(key, n))
    }

    fn mb_number(&self, bytes: u64) -> String {
        let text = format!("{:.1}", bytes as f64 / (1024.0 * 1024.0));
        match self.lang {
            Lang::En => text,
            Lang::Ru => text.replace('.', ","),
        }
    }

    pub fn mb(&self, bytes: u64) -> String {
        format!("{} {}", self.mb_number(bytes), self.t("mb"))
    }

    pub fn mb_of(&self, done: u64, total: u64) -> String {
        format!("{} / {} {}", self.mb_number(done), self.mb_number(total), self.t("mb"))
    }

    pub fn day(&self, unix: i64, now: i64) -> String {
        use chrono::Datelike;
        let (Some(when), Some(today)) = (self.local(unix), self.local(now)) else {
            return String::new();
        };
        if when.date_naive() == today.date_naive() {
            return self.t("today");
        }
        let month = self.t(&format!("month-{}", when.month()));
        let same_year = when.year() == today.year();
        match (self.lang, same_year) {
            (Lang::En, true) => format!("{month} {}", when.day()),
            (Lang::En, false) => format!("{month} {}, {}", when.day(), when.year()),
            (Lang::Ru, true) => format!("{} {month}", when.day()),
            (Lang::Ru, false) => format!("{} {month} {}", when.day(), when.year()),
        }
    }

    pub fn clock(&self, unix: i64) -> String {
        use chrono::Timelike;
        match self.local(unix) {
            Some(when) => format!("{:02}:{:02}", when.hour(), when.minute()),
            None => String::new(),
        }
    }

    pub fn percent(&self, value: f64) -> String {
        let text = format!("{value:.2}%");
        match self.lang {
            Lang::En => text,
            Lang::Ru => text.replace('.', ","),
        }
    }

    pub fn of_max(&self, max: u32) -> String {
        let mut args = FluentArgs::new();
        args.set("max", max);
        self.say("of-max", Some(&args))
    }

    pub fn length(&self, ms: i64) -> String {
        let seconds = (ms.max(0) / 1000) as u64;
        format!("{}:{:02}", seconds / 60, seconds % 60)
    }

    fn say(&self, key: &str, args: Option<&FluentArgs>) -> String {
        let now = said_by(&self.bundle, key, args);
        match &self.before {
            Some((old, k)) if *k < 1.0 => typed(&said_by(old, key, args), &now, *k),
            _ => now,
        }
    }
}

fn said_by(bundle: &FluentBundle<FluentResource>, key: &str, args: Option<&FluentArgs>) -> String {
    let Some(message) = bundle.get_message(key) else {
        return format!("[{key}]");
    };
    let Some(pattern) = message.value() else {
        return format!("[{key}]");
    };
    let mut errors = Vec::new();
    bundle.format_pattern(pattern, args, &mut errors).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(lang: Lang) -> Vec<String> {
        let mut names: Vec<String> = lang
            .source()
            .lines()
            .filter(|line| line.contains(" = ") && !line.starts_with(' '))
            .map(|line| line.split(" = ").next().unwrap_or("").to_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn every_key_in_one_language_is_in_the_other() {
        assert_eq!(keys(Lang::En), keys(Lang::Ru));
        assert!(keys(Lang::En).len() > 40);
    }

    #[test]
    fn russian_counts_decline() {
        let ru = Words::new(Lang::Ru);
        assert_eq!(ru.count("maps-label", 1), "1 карта");
        assert_eq!(ru.count("maps-label", 3), "3 карты");
        assert_eq!(ru.count("maps-label", 1342), "1\u{a0}342 карты");
        assert_eq!(ru.count("maps-label", 5), "5 карт");
        assert_eq!(ru.count("maps-label", 21), "21 карта");
    }

    #[test]
    fn english_counts_group_thousands() {
        let en = Words::new(Lang::En);
        assert_eq!(en.count("maps-label", 1), "1 map");
        assert_eq!(en.count("replays-label", 1342), "1,342 replays");
    }

    #[test]
    fn a_change_of_language_erases_the_old_words_and_types_the_new() {
        assert_eq!(typed("Setting up", "Настройка", 0.0), "Setting up");
        assert_eq!(typed("Setting up", "Настройка", 1.0), "Настройка");
        assert_eq!(typed("Setting up", "Настройка", 0.2), "Setti");
        assert_eq!(typed("Setting up", "Настройка", 0.4), "");
        assert_eq!(typed("Setting up", "Настройка", 0.7), "Наст");
        assert_eq!(typed("Setting up", "Настройка", 0.88), "Настрой");
        let mut words = Words::new(Lang::En).retyping_into(Lang::Ru);
        words.typed_up_to(0.0);
        assert_eq!(words.t("setting-up"), "Setting up");
        words.typed_up_to(1.0);
        assert_eq!(words.t("setting-up"), "Настройка");
        words.settle();
        assert_eq!(words.t("continue"), "Продолжить");
    }

    #[test]
    fn sizes_read_in_megabytes_with_the_language_s_decimal_mark() {
        assert_eq!(Words::new(Lang::En).mb_of(12_950_000, 28_832_991), "12.4 / 27.5 MB");
        assert_eq!(Words::new(Lang::Ru).mb(28_832_991), "27,5 МБ");
    }

    #[test]
    fn a_day_is_named_the_way_its_language_says_it() {
        use chrono::TimeZone;
        let now = chrono::Local.with_ymd_and_hms(2026, 9, 16, 12, 0, 0).single().unwrap().timestamp();
        let same_day = chrono::Local.with_ymd_and_hms(2026, 9, 16, 9, 30, 0).single().unwrap().timestamp();
        let august = chrono::Local.with_ymd_and_hms(2026, 8, 14, 21, 34, 0).single().unwrap().timestamp();
        let last_year = chrono::Local.with_ymd_and_hms(2025, 5, 10, 8, 0, 0).single().unwrap().timestamp();
        let en = Words::new(Lang::En);
        let ru = Words::new(Lang::Ru);
        assert_eq!(en.day(same_day, now), "today");
        assert_eq!(ru.day(same_day, now), "сегодня");
        assert_eq!(en.day(august, now), "Aug 14");
        assert_eq!(ru.day(august, now), "14 авг");
        assert_eq!(en.day(last_year, now), "May 10, 2025");
        assert_eq!(ru.day(last_year, now), "10 мая 2025");
        assert_eq!(en.clock(august), "21:34");
        assert_eq!(en.percent(98.7123), "98.71%");
        assert_eq!(ru.percent(98.7123), "98,71%");
        assert_eq!(en.length(231_400), "3:51");
    }

    #[test]
    fn a_missing_key_shows_itself_rather_than_nothing() {
        assert_eq!(Words::new(Lang::En).t("no-such-key"), "[no-such-key]");
    }
}
