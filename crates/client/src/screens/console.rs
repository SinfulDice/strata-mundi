//! Command console (T or `/`): a line to type in, the answers above it, the history on
//! up/down and completion on Tab. World commands go to the server; `/help` and
//! `/cinematic` are answered here.

use super::Screens;
use super::menu::{Effect, Menu, Shared, UiAction};
use crate::gameplay::Gameplay;
use crate::input::{Action, InputState};
use crate::ui::{LogLine, ScreenModel, Tone};
use std::collections::VecDeque;
use strata_core::content::Content;
use strata_protocol::commands::{Arg, COMMANDS, Feedback, Runs, TIME_PRESETS, find, words};

const MAX_LINES: usize = 100;
const MAX_HISTORY: usize = 50;

/// What the console keeps between openings.
#[derive(Default)]
pub struct ConsoleLog {
    lines: VecDeque<LogLine>,
    history: Vec<String>,
}

impl ConsoleLog {
    fn push(&mut self, text: String, tone: Tone) {
        if self.lines.len() == MAX_LINES {
            self.lines.pop_front();
        }
        self.lines.push_back(LogLine { text, tone });
    }

    /// A server answer, in the player's language.
    pub fn feedback(&mut self, feedback: &Feedback, content: &Content, language: &str) {
        let args: Vec<(&str, String)> = feedback
            .args
            .iter()
            .map(|(name, value)| (name.as_str(), translated_arg(value, content, language)))
            .collect();
        let text = content
            .translations
            .text_with(language, &feedback.key, &args);
        let tone = if feedback.error {
            Tone::Error
        } else {
            Tone::Normal
        };
        self.push(text, tone);
    }

    fn remember(&mut self, line: &str) {
        if self.history.last().is_none_or(|last| last != line) {
            self.history.push(line.to_owned());
        }
        if self.history.len() > MAX_HISTORY {
            self.history.remove(0);
        }
    }
}

/// `command-tp-usage` given as an argument stands for its translation.
fn translated_arg(value: &str, content: &Content, language: &str) -> String {
    if value.starts_with("command-") {
        content.translations.text(language, value)
    } else {
        value.to_owned()
    }
}

/// The line being typed.
#[derive(Debug)]
pub struct ConsoleState {
    pub line: String,
    /// Position in the history while browsing it with up/down.
    browsing: Option<usize>,
}

impl ConsoleState {
    pub fn new() -> Self {
        Self {
            line: "/".to_owned(),
            browsing: None,
        }
    }
}

pub struct ConsoleModel<'a> {
    pub state: &'a ConsoleState,
    pub log: &'a ConsoleLog,
    pub shared: Shared<'a>,
}

impl ScreenModel for ConsoleModel<'_> {
    fn text(&self, id: &str) -> Option<String> {
        match id {
            "line" => Some(self.state.line.clone()),
            "hint" => {
                let language = &self.shared.settings.interface.language;
                Some(hint(&self.state.line, self.shared.content, language))
            }
            _ => None,
        }
    }

    fn log_lines(&self, _: &str) -> Vec<LogLine> {
        self.log.lines.iter().cloned().collect()
    }
}

/// The completions of the word being typed, or the usage of the command.
fn hint(line: &str, content: &Content, language: &str) -> String {
    let options = completions(line, content);
    if options.len() > 1 {
        return options.join("  ");
    }
    words(line)
        .first()
        .and_then(|name| find(name))
        .map_or_else(String::new, |spec| {
            content.translations.text(language, &spec.usage_key())
        })
}

/// Every value the word being typed could become.
fn completions(line: &str, content: &Content) -> Vec<String> {
    let ends_with_space = line.ends_with(' ');
    let words = words(line);
    let index = if ends_with_space || words.is_empty() {
        words.len()
    } else {
        words.len() - 1
    };
    let typed = if index < words.len() {
        words[index]
    } else {
        ""
    };
    let candidates: Vec<String> = if index == 0 {
        COMMANDS.iter().map(|c| c.name.to_owned()).collect()
    } else {
        let arg = find(words[0]).and_then(|spec| spec.args.get(index - 1));
        match arg {
            Some(arg) => candidates_for(*arg, words.get(1).copied(), content),
            None => Vec::new(),
        }
    };
    candidates
        .into_iter()
        .filter(|c| c.starts_with(typed) || short_name(c).starts_with(typed))
        .collect()
}

fn candidates_for(arg: Arg, first: Option<&str>, content: &Content) -> Vec<String> {
    match arg {
        Arg::Item => content
            .items
            .iter()
            .map(|(_, id, _)| id.to_string())
            .collect(),
        Arg::Cinematic => content.cinematics.keys().map(ToString::to_string).collect(),
        Arg::Command => COMMANDS.iter().map(|c| c.name.to_owned()).collect(),
        Arg::Choice(choices) => choices.iter().map(|c| (*c).to_owned()).collect(),
        Arg::Number if first == Some("set") => {
            TIME_PRESETS.iter().map(|t| (*t).to_owned()).collect()
        }
        Arg::Coordinate => vec!["~".to_owned()],
        Arg::Number => Vec::new(),
    }
}

/// `strata:stone` may be typed as `stone`.
fn short_name(id: &str) -> &str {
    id.split_once(':').map_or(id, |(_, name)| name)
}

/// The line with its last word completed as far as all candidates agree.
fn complete(line: &str, content: &Content) -> String {
    let options = completions(line, content);
    let Some(first) = options.first() else {
        return line.to_owned();
    };
    let common = options.iter().fold(first.clone(), |prefix, option| {
        prefix
            .chars()
            .zip(option.chars())
            .take_while(|(a, b)| a == b)
            .map(|(a, _)| a)
            .collect()
    });
    let start = line.rfind([' ', '/']).map_or(0, |i| i + 1);
    let typed = &line[start..];
    if common.len() < typed.len() && options.len() > 1 {
        return line.to_owned();
    }
    let suffix = if options.len() == 1 { " " } else { "" };
    format!("{}{common}{suffix}", &line[..start])
}

impl Screens {
    /// Typing, history, completion, sending; Escape closes.
    pub(super) fn handle_console(
        &mut self,
        input: &InputState,
        game: Option<&mut Gameplay>,
        content: &Content,
    ) {
        let Some(Menu::Console(state)) = self.stack.last_mut() else {
            return;
        };
        if input.just_pressed(Action::MenuBack) {
            self.effects.push(Effect::Pop);
            return;
        }
        state.line.push_str(input.typed());
        if input.just_pressed(Action::Erase) {
            state.line.pop();
        }
        if input.just_pressed(Action::Complete) {
            state.line = complete(&state.line, content);
        }
        let step = i32::from(input.just_pressed(Action::MenuDown))
            - i32::from(input.just_pressed(Action::MenuUp));
        if step != 0 {
            browse(state, &self.console.history, step);
        }
        if input.just_pressed(Action::MenuConfirm) {
            let line = std::mem::replace(&mut state.line, "/".to_owned());
            state.browsing = None;
            self.submit(&line, game.is_some(), content);
        }
    }

    fn submit(&mut self, line: &str, in_world: bool, content: &Content) {
        let words = words(line);
        let Some(&name) = words.first() else {
            return;
        };
        self.console.remember(line.trim());
        self.console.push(line.trim().to_owned(), Tone::Dim);
        let language = self.settings.interface.language.clone();
        let tr = |key: &str| content.translations.text(&language, key);
        match find(name) {
            None => {
                let args = [("name", name.to_owned())];
                let text = content
                    .translations
                    .text_with(&language, "command-unknown", &args);
                self.console.push(text, Tone::Error);
            }
            Some(_) if !in_world => self.console.push(tr("command-needs-world"), Tone::Error),
            Some(spec) if spec.runs == Runs::Server => {
                let command = line.trim().trim_start_matches('/').to_owned();
                self.effects
                    .push(Effect::Action(UiAction::Command(command)));
            }
            Some(_) if name == "help" => self.help(words.get(1).copied(), content, &language),
            Some(_) => match words.get(1).and_then(|id| id.parse().ok()) {
                Some(id) if content.cinematics.contains_key(&id) => {
                    self.effects
                        .push(Effect::Action(UiAction::PlayCinematic(id)));
                    self.effects.push(Effect::CloseAll);
                }
                _ => self
                    .console
                    .push(tr("command-cinematic-unknown"), Tone::Error),
            },
        }
    }

    fn help(&mut self, about: Option<&str>, content: &Content, language: &str) {
        let shown: Vec<_> = COMMANDS
            .iter()
            .filter(|spec| about.is_none_or(|name| spec.name == name))
            .collect();
        for spec in shown {
            let usage = content.translations.text(language, &spec.usage_key());
            let help = content.translations.text(language, &spec.help_key());
            self.console.push(usage, Tone::Normal);
            self.console.push(format!("  {help}"), Tone::Dim);
        }
    }
}

fn browse(state: &mut ConsoleState, history: &[String], step: i32) {
    if history.is_empty() {
        return;
    }
    let last = history.len() - 1;
    let next = match (state.browsing, step < 0) {
        (None, true) => Some(last),
        (None, false) => None,
        (Some(i), true) => Some(i.saturating_sub(1)),
        (Some(i), false) if i < last => Some(i + 1),
        (Some(_), false) => None,
    };
    state.browsing = next;
    state.line = next.map_or_else(|| "/".to_owned(), |i| history[i].clone());
}

/// Keys the console may show, for the missing-translation test.
#[cfg(test)]
pub fn translation_keys() -> Vec<String> {
    let fixed = [
        "command-empty",
        "command-usage",
        "command-unknown",
        "command-needs-world",
        "command-cinematic-unknown",
        "command-tp-done",
        "command-give-done",
        "command-give-unknown",
        "command-gamemode-done",
        "command-gamemode-later",
        "command-time-now",
        "command-seed-is",
    ];
    COMMANDS
        .iter()
        .flat_map(|spec| [spec.usage_key(), spec.help_key()])
        .chain(fixed.map(str::to_owned))
        .collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use strata_core::pack::PackSet;

    fn content() -> Content {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/packs");
        let packs = PackSet::discover(&dir, &mut Vec::new());
        Content::load(&packs, &PackSet::default()).0
    }

    #[test]
    fn tab_completes_commands_and_arguments() {
        let content = content();
        assert_eq!(complete("/ti", &content), "/time ");
        assert_eq!(complete("/time s", &content), "/time set ");
        assert_eq!(complete("/time set no", &content), "/time set noon ");
        assert_eq!(
            complete("/give sandst", &content),
            "/give strata:sandstone "
        );
        assert_eq!(complete("/zzz", &content), "/zzz");
        let hint = hint("/g", &content, "fr");
        assert!(hint.contains("give") && hint.contains("gamemode"));
    }

    #[test]
    fn history_is_browsed_with_up_and_down() {
        let history = vec!["/seed".to_owned(), "/tp 0 80 0".to_owned()];
        let mut state = ConsoleState::new();
        browse(&mut state, &history, -1);
        assert_eq!(state.line, "/tp 0 80 0");
        browse(&mut state, &history, -1);
        assert_eq!(state.line, "/seed");
        browse(&mut state, &history, 1);
        browse(&mut state, &history, 1);
        assert_eq!(state.line, "/");
    }
}
