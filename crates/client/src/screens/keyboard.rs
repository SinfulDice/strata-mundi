//! On-screen keyboard for typing with a gamepad (layout in `assets/ui/keyboard.ron`).
//! Four characters of the layout are commands rather than letters.

use crate::ui::ScreenModel;
use strata_core::lang::Translations;

const ERASE: char = '<';
const SHIFT: char = '^';
const SPACE: char = '_';
const DONE: char = '>';

/// The keyboard open over a text field.
pub struct Keyboard {
    pub field: String,
    shift: bool,
}

impl Keyboard {
    pub fn new(field: String) -> Self {
        Self {
            field,
            shift: false,
        }
    }
}

pub struct KeyboardModel<'a> {
    pub keyboard: &'a mut Keyboard,
    pub rows: &'a [String],
    pub translations: &'a Translations,
    pub language: &'a str,
    /// Characters for the field (`\u{8}` erases).
    pub typed: &'a mut String,
    pub done: &'a mut bool,
}

impl KeyboardModel<'_> {
    fn key(&self, index: usize) -> Option<char> {
        self.rows.iter().flat_map(|row| row.chars()).nth(index)
    }
}

impl ScreenModel for KeyboardModel<'_> {
    /// Captions: `keys:<char>` for each key.
    fn text(&self, id: &str) -> Option<String> {
        let ch = id.strip_prefix("keys:")?.chars().next()?;
        let key = match ch {
            ERASE => "ui-key-erase",
            SHIFT => "ui-key-shift",
            SPACE => "ui-key-space",
            DONE => "ui-key-done",
            _ if self.keyboard.shift => return Some(ch.to_uppercase().collect()),
            _ => return None,
        };
        Some(self.translations.text(self.language, key))
    }

    fn activate(&mut self, id: &str, index: usize) {
        if id != "keys" {
            return;
        }
        match self.key(index) {
            Some(ERASE) => self.typed.push('\u{8}'),
            Some(SHIFT) => self.keyboard.shift = !self.keyboard.shift,
            Some(SPACE) => self.typed.push(' '),
            Some(DONE) => *self.done = true,
            Some(ch) => {
                if std::mem::take(&mut self.keyboard.shift) {
                    self.typed.extend(ch.to_uppercase());
                } else {
                    self.typed.push(ch);
                }
            }
            None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_type_letters_and_commands() {
        let rows = vec!["ab^".to_owned(), "<_>".to_owned()];
        let translations = Translations::default();
        let mut keyboard = Keyboard::new("name".into());
        let (mut typed, mut done) = (String::new(), false);
        let mut model = KeyboardModel {
            keyboard: &mut keyboard,
            rows: &rows,
            translations: &translations,
            language: "fr",
            typed: &mut typed,
            done: &mut done,
        };
        for index in [0, 2, 1, 4, 3, 5] {
            model.activate("keys", index);
        }
        assert_eq!(typed, "aB \u{8}");
        assert!(done);
    }
}
