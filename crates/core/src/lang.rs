//! Translations in Fluent (`lang/<language>/*.ftl` in packs). No displayed text is written
//! in the code: everything goes through a key.

use crate::pack::{PackError, PackSet};
use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::{FluentArgs, FluentResource};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::Path;
use unic_langid::LanguageIdentifier;

/// Languages of the game. `lol` (LOLcat) uses English grammar rules.
pub const LANGUAGES: [&str; 3] = ["fr", "en", "lol"];
const FALLBACK: &str = "en";

pub struct Translations {
    bundles: BTreeMap<&'static str, FluentBundle<FluentResource>>,
    keys: BTreeMap<&'static str, BTreeSet<String>>,
}

/// No text at all: every key shows as itself (tests, tools).
impl Default for Translations {
    fn default() -> Self {
        Self {
            bundles: BTreeMap::new(),
            keys: BTreeMap::new(),
        }
    }
}

impl fmt::Debug for Translations {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Translations")
            .field("keys", &self.keys)
            .finish()
    }
}

impl Translations {
    /// Later packs override keys of earlier ones.
    pub fn load(packs: &PackSet, errors: &mut Vec<PackError>) -> Self {
        let mut translations = Self {
            bundles: BTreeMap::new(),
            keys: BTreeMap::new(),
        };
        for language in LANGUAGES {
            translations.bundles.insert(language, new_bundle(language));
            translations.keys.insert(language, BTreeSet::new());
            for pack in packs.packs() {
                translations.load_dir(language, &pack.root.join("lang").join(language), errors);
            }
        }
        translations
    }

    fn load_dir(&mut self, language: &'static str, dir: &Path, errors: &mut Vec<PackError>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        let mut files: Vec<_> = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|e| e == "ftl"))
            .collect();
        files.sort();
        for path in files {
            match std::fs::read_to_string(&path) {
                Ok(text) => self.add(language, text, &path, errors),
                Err(error) => errors.push(PackError::io(&path, error)),
            }
        }
    }

    fn add(
        &mut self,
        language: &'static str,
        text: String,
        path: &Path,
        errors: &mut Vec<PackError>,
    ) {
        let resource = FluentResource::try_new(text).unwrap_or_else(|(partial, syntax)| {
            for error in syntax {
                errors.push(PackError::invalid(
                    path,
                    format!("Fluent syntax error: {error:?}"),
                ));
            }
            partial
        });
        let keys = self.keys.entry(language).or_default();
        for entry in resource.entries() {
            if let fluent_syntax::ast::Entry::Message(message) = entry {
                keys.insert(message.id.name.to_owned());
            }
        }
        if let Some(bundle) = self.bundles.get_mut(language) {
            bundle.add_resource_overriding(resource);
        }
    }

    pub fn has(&self, language: &str, key: &str) -> bool {
        self.keys
            .get(language)
            .is_some_and(|keys| keys.contains(key))
    }

    pub fn keys(&self, language: &str) -> impl Iterator<Item = &str> {
        self.keys
            .get(language)
            .into_iter()
            .flatten()
            .map(String::as_str)
    }

    /// Text of `key` in `language`, falling back to English, then to the key itself.
    pub fn text(&self, language: &str, key: &str) -> String {
        self.format(language, key, None)
            .or_else(|| self.format(FALLBACK, key, None))
            .unwrap_or_else(|| key.to_owned())
    }

    /// `text` with variables, e.g. `[("minutes", "12".into())]` for `{ $minutes }`.
    pub fn text_with(&self, language: &str, key: &str, args: &[(&str, String)]) -> String {
        let mut fluent = FluentArgs::new();
        for (name, value) in args {
            fluent.set(*name, value.clone());
        }
        self.format(language, key, Some(&fluent))
            .or_else(|| self.format(FALLBACK, key, Some(&fluent)))
            .unwrap_or_else(|| key.to_owned())
    }

    /// `value` with `decimals` digits after the language's separator (`decimal-separator`,
    /// `,` in French): 7.5 → « 7,50 ».
    pub fn decimal(&self, language: &str, value: f64, decimals: usize) -> String {
        let text = format!("{value:.decimals$}");
        let separator = self.text(language, DECIMAL_SEPARATOR);
        match separator.as_str() {
            "." | "" => text,
            _ if separator == DECIMAL_SEPARATOR => text,
            _ => text.replace('.', &separator),
        }
    }

    /// `decimal` without the useless trailing zeros: 4.0 → « 4 », 1.5 → « 1,5 ».
    pub fn short_decimal(&self, language: &str, value: f64) -> String {
        let text = format!("{value:.2}");
        let trimmed = text.trim_end_matches('0').trim_end_matches('.');
        let decimals = trimmed.split_once('.').map_or(0, |(_, after)| after.len());
        self.decimal(language, value, decimals)
    }

    pub fn format(
        &self,
        language: &str,
        key: &str,
        args: Option<&FluentArgs<'_>>,
    ) -> Option<String> {
        let bundle = self.bundles.get(language)?;
        let pattern = bundle.get_message(key)?.value()?;
        let mut errors = Vec::new();
        let text = bundle.format_pattern(pattern, args, &mut errors);
        for error in errors {
            tracing::warn!(%error, key, language, "translation could not be formatted");
        }
        Some(text.into_owned())
    }

    /// `(language, key)` pairs missing from some language: every key of `required`, plus
    /// every key present in another language.
    pub fn missing(&self, required: &[String]) -> Vec<(&'static str, String)> {
        let mut all: BTreeSet<&str> = required.iter().map(String::as_str).collect();
        all.extend(self.keys.values().flatten().map(String::as_str));
        LANGUAGES
            .iter()
            .flat_map(|&language| {
                all.iter()
                    .filter(move |key| !self.has(language, key))
                    .map(move |key| (language, (*key).to_owned()))
            })
            .collect()
    }
}

fn new_bundle(language: &str) -> FluentBundle<FluentResource> {
    let grammar = if language == "lol" {
        FALLBACK
    } else {
        language
    };
    let langid: LanguageIdentifier = grammar.parse().unwrap_or_default();
    let mut bundle = FluentBundle::new_concurrent(vec![langid]);
    // Unicode isolation marks would show as boxes in the bitmap font.
    bundle.set_use_isolating(false);
    bundle
}

/// Fluent key of the decimal separator of each language.
const DECIMAL_SEPARATOR: &str = "decimal-separator";

#[cfg(test)]
mod decimal_tests {
    use super::*;
    use crate::pack::PackSet;
    use std::path::Path;

    #[test]
    fn decimals_follow_the_language() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/packs");
        let packs = PackSet::discover(&dir, &mut Vec::new());
        let translations = Translations::load(&packs, &mut Vec::new());
        assert_eq!(translations.decimal("fr", 7.5, 2), "7,50");
        assert_eq!(translations.decimal("en", 7.5, 2), "7.50");
        assert_eq!(translations.short_decimal("fr", 4.0), "4");
        assert_eq!(translations.short_decimal("fr", 1.25), "1,25");
        assert_eq!(translations.short_decimal("en", 0.5), "0.5");
        let seconds = [("seconds", translations.decimal("fr", 7.5, 2))];
        let text = translations.text_with("fr", "wiki-seconds", &seconds);
        assert!(text.contains("7,50\u{a0}s"), "{text:?}");
    }
}
