use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;
use thiserror::Error;

/// Text identifier `namespace:name`, e.g. `strata:stone`. Lowercase ASCII letters, digits, `_`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Identifier {
    namespace: String,
    name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error(
    "invalid identifier `{0}`: expected `namespace:name` with lowercase letters, digits and `_`"
)]
pub struct InvalidIdentifier(pub String);

impl Identifier {
    pub fn new(namespace: &str, name: &str) -> Result<Self, InvalidIdentifier> {
        if is_valid_part(namespace) && is_valid_part(name) {
            Ok(Self {
                namespace: namespace.to_owned(),
                name: name.to_owned(),
            })
        } else {
            Err(InvalidIdentifier(format!("{namespace}:{name}")))
        }
    }

    /// For identifiers written in the engine itself, known to be valid.
    pub(crate) fn builtin(namespace: &'static str, name: &'static str) -> Self {
        debug_assert!(is_valid_part(namespace) && is_valid_part(name));
        Self {
            namespace: namespace.to_owned(),
            name: name.to_owned(),
        }
    }

    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// Key usable in Fluent files: `block` + `strata:stone` → `block-strata-stone`.
    pub fn translation_key(&self, kind: &str) -> String {
        format!("{kind}-{}-{}", self.namespace, self.name)
    }
}

pub fn is_valid_part(part: &str) -> bool {
    !part.is_empty()
        && part
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

impl FromStr for Identifier {
    type Err = InvalidIdentifier;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let (namespace, name) = text
            .split_once(':')
            .ok_or_else(|| InvalidIdentifier(text.to_owned()))?;
        Self::new(namespace, name).map_err(|_| InvalidIdentifier(text.to_owned()))
    }
}

impl fmt::Display for Identifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.namespace, self.name)
    }
}

impl Serialize for Identifier {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Identifier {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_displays() {
        let id: Identifier = "strata:oak_log".parse().unwrap();
        assert_eq!(id.namespace(), "strata");
        assert_eq!(id.name(), "oak_log");
        assert_eq!(id.to_string(), "strata:oak_log");
        assert_eq!(id.translation_key("block"), "block-strata-oak_log");
    }

    #[test]
    fn rejects_bad_identifiers() {
        for bad in [
            "stone",
            "Strata:stone",
            "strata:",
            ":stone",
            "strata:st one",
            "a:b:c",
        ] {
            assert!(bad.parse::<Identifier>().is_err(), "{bad}");
        }
    }
}
