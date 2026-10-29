//! `wiki-content/history.ron`: the released versions and, for each entry, when it was added
//! or changed.

use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Default, Deserialize)]
pub struct History {
    /// Oldest first.
    pub versions: Vec<Version>,
    /// By page key (`blocks/strata/stone`).
    pub entries: BTreeMap<String, Vec<Event>>,
}

#[derive(Debug, Deserialize)]
pub struct Version {
    pub id: String,
    pub date: String,
}

#[derive(Debug, Deserialize)]
pub struct Event {
    pub version: String,
    pub change: Change,
}

#[derive(Debug, Deserialize)]
pub enum Change {
    Added,
    /// What changed, in each language.
    Changed {
        fr: String,
        en: String,
        lol: String,
    },
}

impl Change {
    /// The description in `lang`; `None` for an addition (the page says "added").
    pub fn text(&self, lang: &str) -> Option<&str> {
        match self {
            Self::Added => None,
            Self::Changed { fr, en, lol } => Some(match lang {
                "fr" => fr,
                "lol" => lol,
                _ => en,
            }),
        }
    }
}

impl History {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?;
        ron::from_str(&text).map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))
    }

    /// The version that added `key`.
    pub fn added_in(&self, key: &str) -> Option<&str> {
        self.entries.get(key)?.iter().find_map(|event| {
            matches!(event.change, Change::Added).then_some(event.version.as_str())
        })
    }

    /// Entries with an event in `version`, by page key.
    pub fn in_version<'a>(
        &'a self,
        version: &'a str,
    ) -> impl Iterator<Item = (&'a str, &'a Event)> {
        self.entries.iter().flat_map(move |(key, events)| {
            events
                .iter()
                .filter(move |event| event.version == version)
                .map(move |event| (key.as_str(), event))
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn history_tells_when_entries_came() {
        let history: History = ron::from_str(
            r#"(versions: [(id: "0.1.0", date: "2026-10-23")],
                entries: {"blocks/strata/stone": [
                    (version: "0.1.0", change: Added),
                    (version: "0.2.0", change: Changed(fr: "a", en: "b", lol: "c")),
                ]})"#,
        )
        .unwrap();
        assert_eq!(history.added_in("blocks/strata/stone"), Some("0.1.0"));
        assert_eq!(history.in_version("0.2.0").count(), 1);
    }
}
