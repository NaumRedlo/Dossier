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
}

impl Clone for Words {
    fn clone(&self) -> Words {
        Words::new(self.lang)
    }
}

impl Words {
    pub fn new(lang: Lang) -> Words {
        let resource = FluentResource::try_new(lang.source().to_owned())
            .unwrap_or_else(|(resource, _)| resource);
        let mut bundle = FluentBundle::new(vec![lang.id()]);
        bundle.set_use_isolating(false);
        let _ = bundle.add_resource(resource);
        Words { lang, bundle }
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

    fn say(&self, key: &str, args: Option<&FluentArgs>) -> String {
        let Some(message) = self.bundle.get_message(key) else {
            return format!("[{key}]");
        };
        let Some(pattern) = message.value() else {
            return format!("[{key}]");
        };
        let mut errors = Vec::new();
        self.bundle.format_pattern(pattern, args, &mut errors).into_owned()
    }
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
    fn a_missing_key_shows_itself_rather_than_nothing() {
        assert_eq!(Words::new(Lang::En).t("no-such-key"), "[no-such-key]");
    }
}
