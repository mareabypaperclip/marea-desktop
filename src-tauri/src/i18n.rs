//! Translations. The app speaks the system language when it knows it (Spanish, English, French or Catalan) and
//! English otherwise; other languages of Spain (Galician, Basque, Asturian, Aragonese) fall back to Spanish.
//!
//! The catalogs live in `ui/locales/*.json` and are shared with the connection screen, which receives the active
//! one from `app_info`, so both sides always agree on the language.

use std::collections::HashMap;
use std::sync::OnceLock;

/// Supported languages, in the order they are offered.
#[cfg(test)]
const LANGUAGES: &[&str] = &["en", "es", "fr", "ca"];
const FALLBACK: &str = "en";

static CATALOGS: OnceLock<HashMap<&'static str, HashMap<String, String>>> = OnceLock::new();
static LANG: OnceLock<&'static str> = OnceLock::new();

fn catalogs() -> &'static HashMap<&'static str, HashMap<String, String>> {
    CATALOGS.get_or_init(|| {
        let parse = |text: &str| serde_json::from_str::<HashMap<String, String>>(text).unwrap_or_default();
        HashMap::from([
            ("en", parse(include_str!("../../ui/locales/en.json"))),
            ("es", parse(include_str!("../../ui/locales/es.json"))),
            ("fr", parse(include_str!("../../ui/locales/fr.json"))),
            ("ca", parse(include_str!("../../ui/locales/ca.json"))),
        ])
    })
}

/// Picks a supported language for a system locale such as `es-ES`, `ca-ES-valencia` or `pt-BR`.
pub fn pick(locale: &str) -> &'static str {
    let code = locale.split(['-', '_', '.', '@']).next().unwrap_or_default().to_ascii_lowercase();
    match code.as_str() {
        "es" | "gl" | "eu" | "ast" | "an" => "es",
        "fr" => "fr",
        "ca" => "ca",
        "en" => "en",
        _ => FALLBACK,
    }
}

/// The language in use, detected once from the system.
pub fn lang() -> &'static str {
    LANG.get_or_init(|| pick(&sys_locale::get_locale().unwrap_or_default()))
}

/// The whole catalog of the language in use (for the connection screen).
pub fn catalog() -> &'static HashMap<String, String> {
    let all = catalogs();
    all.get(lang()).or_else(|| all.get(FALLBACK)).expect("the English catalog is always there")
}

/// A translated text; falls back to English and then to the key itself.
pub fn t(key: &str) -> String {
    let all = catalogs();
    all.get(lang())
        .and_then(|c| c.get(key))
        .or_else(|| all.get(FALLBACK).and_then(|c| c.get(key)))
        .cloned()
        .unwrap_or_else(|| key.to_string())
}

/// A translated text with `{name}` placeholders filled in.
pub fn tf(key: &str, values: &[(&str, &str)]) -> String {
    values.iter().fold(t(key), |text, (name, value)| text.replace(&format!("{{{name}}}"), value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_languages() {
        assert_eq!(pick("es-ES"), "es");
        assert_eq!(pick("es_ES.UTF-8"), "es");
        assert_eq!(pick("ca-ES-valencia"), "ca");
        assert_eq!(pick("gl-ES"), "es");
        assert_eq!(pick("fr-CA"), "fr");
        assert_eq!(pick("en-GB"), "en");
        assert_eq!(pick("de-DE"), "en");
        assert_eq!(pick(""), "en");
    }

    #[test]
    fn every_catalog_has_every_key() {
        let all = catalogs();
        let english = &all["en"];
        assert!(english.len() > 50, "the English catalog looks empty");
        for lang in LANGUAGES {
            let catalog = &all[lang];
            for key in english.keys() {
                assert!(catalog.contains_key(key), "{lang} lacks {key}");
            }
            for key in catalog.keys() {
                assert!(english.contains_key(key), "{lang} has an unknown key {key}");
            }
        }
    }

    #[test]
    fn placeholders_survive_translation() {
        let all = catalogs();
        for (key, english) in &all["en"] {
            let wanted: Vec<&str> = english.match_indices('{').map(|(i, _)| &english[i..]).collect();
            for lang in LANGUAGES {
                let text = &all[lang][key];
                for piece in &wanted {
                    let name = &piece[..piece.find('}').map_or(piece.len(), |e| e + 1)];
                    assert!(text.contains(name), "{lang}.{key} lacks {name}");
                }
            }
        }
    }

    #[test]
    fn fills_placeholders() {
        let text = tf("update.uptodate.body", &[("current", "9.9.9")]);
        assert!(text.contains("9.9.9"));
    }
}
