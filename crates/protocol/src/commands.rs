//! Console commands: what exists and what each argument expects, shared by the server
//! (which runs world commands) and the client (help and autocompletion).

use serde::{Deserialize, Serialize};

/// What an argument expects, for autocompletion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arg {
    /// A block coordinate; `~` is the player's own, `~5` five blocks from it.
    Coordinate,
    Number,
    /// An item identifier (`strata:stone`).
    Item,
    /// A cinematic identifier from the packs.
    Cinematic,
    /// A command name (for `/help`).
    Command,
    Choice(&'static [&'static str]),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Runs {
    /// Changes or reads the world: sent to the server.
    Server,
    /// Only concerns this screen (help, cinematics).
    Client,
}

#[derive(Debug, Clone, Copy)]
pub struct CommandSpec {
    pub name: &'static str,
    pub args: &'static [Arg],
    pub runs: Runs,
    /// Changes the world in a way that would be cheating: creative only.
    pub cheat: bool,
}

impl CommandSpec {
    /// `command-tp-usage`: the syntax line, translated.
    pub fn usage_key(&self) -> String {
        format!("command-{}-usage", self.name)
    }

    pub fn help_key(&self) -> String {
        format!("command-{}-help", self.name)
    }
}

pub const TIME_PRESETS: [&str; 4] = ["day", "noon", "night", "midnight"];

pub const COMMANDS: [CommandSpec; 7] = [
    CommandSpec {
        name: "help",
        args: &[Arg::Command],
        runs: Runs::Client,
        cheat: false,
    },
    CommandSpec {
        name: "tp",
        args: &[Arg::Coordinate, Arg::Coordinate, Arg::Coordinate],
        runs: Runs::Server,
        cheat: true,
    },
    CommandSpec {
        name: "give",
        args: &[Arg::Item],
        runs: Runs::Server,
        cheat: true,
    },
    CommandSpec {
        name: "gamemode",
        args: &[Arg::Choice(&["creative", "survival"])],
        runs: Runs::Server,
        cheat: true,
    },
    CommandSpec {
        name: "time",
        args: &[Arg::Choice(&["set", "add", "query"]), Arg::Number],
        runs: Runs::Server,
        cheat: true,
    },
    CommandSpec {
        name: "seed",
        args: &[],
        runs: Runs::Server,
        cheat: false,
    },
    CommandSpec {
        name: "cinematic",
        args: &[Arg::Cinematic],
        runs: Runs::Client,
        cheat: false,
    },
];

pub fn find(name: &str) -> Option<&'static CommandSpec> {
    COMMANDS.iter().find(|spec| spec.name == name)
}

/// A console line split into words, without the leading `/`.
pub fn words(line: &str) -> Vec<&str> {
    line.trim()
        .trim_start_matches('/')
        .split_whitespace()
        .collect()
}

/// A message for the console: a translation key and its arguments, so each player reads
/// it in their own language.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Feedback {
    pub key: String,
    pub args: Vec<(String, String)>,
    pub error: bool,
}

impl Feedback {
    pub fn ok(key: &str, args: &[(&str, String)]) -> Self {
        Self::new(key, args, false)
    }

    pub fn error(key: &str, args: &[(&str, String)]) -> Self {
        Self::new(key, args, true)
    }

    fn new(key: &str, args: &[(&str, String)], error: bool) -> Self {
        Self {
            key: key.to_owned(),
            args: args
                .iter()
                .map(|(name, value)| ((*name).to_owned(), value.clone()))
                .collect(),
            error,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_split_into_words() {
        assert_eq!(words(" /tp ~ 80  ~5 "), ["tp", "~", "80", "~5"]);
        assert_eq!(find("time").map(|c| c.runs), Some(Runs::Server));
        assert!(find("nope").is_none());
    }
}
