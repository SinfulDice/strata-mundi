//! Light markup inside translated texts (Fluent leaves `<` alone):
//! - `<key:Inventory>`: the key or button bound to an action, drawn as a key cap and
//!   following the last device used (keyboard or gamepad);
//! - `<c:accent>…</c>`: text in the accent color.
//!
//! Anything else is shown as written.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Span<'a> {
    Text { text: &'a str, accent: bool },
    Key(&'a str),
}

pub fn parse(text: &str) -> Vec<Span<'_>> {
    let mut spans = Vec::new();
    let mut accent = false;
    let mut rest = text;
    while !rest.is_empty() {
        let (tag, before, after) = match next_tag(rest) {
            Some(found) => found,
            None => {
                spans.push(Span::Text { text: rest, accent });
                break;
            }
        };
        if !before.is_empty() {
            spans.push(Span::Text {
                text: before,
                accent,
            });
        }
        match tag {
            Tag::Key(action) => spans.push(Span::Key(action)),
            Tag::Accent => accent = true,
            Tag::End => accent = false,
        }
        rest = after;
    }
    spans
}

enum Tag<'a> {
    Key(&'a str),
    Accent,
    End,
}

/// The first well-formed tag: (tag, text before it, text after it).
fn next_tag(text: &str) -> Option<(Tag<'_>, &str, &str)> {
    let mut search = 0;
    while let Some(start) = text[search..].find('<').map(|i| i + search) {
        let end = text[start..].find('>').map(|i| i + start)?;
        let inner = &text[start + 1..end];
        let tag = match inner {
            "c:accent" => Some(Tag::Accent),
            "/c" => Some(Tag::End),
            _ => inner
                .strip_prefix("key:")
                .filter(|a| !a.is_empty())
                .map(Tag::Key),
        };
        if let Some(tag) = tag {
            return Some((tag, &text[..start], &text[end + 1..]));
        }
        search = start + 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_and_accents_are_recognised() {
        let spans = parse("Press <key:Jump> to <c:accent>fly</c>!");
        assert_eq!(
            spans,
            [
                Span::Text {
                    text: "Press ",
                    accent: false
                },
                Span::Key("Jump"),
                Span::Text {
                    text: " to ",
                    accent: false
                },
                Span::Text {
                    text: "fly",
                    accent: true
                },
                Span::Text {
                    text: "!",
                    accent: false
                },
            ]
        );
    }

    #[test]
    fn unknown_or_broken_tags_stay_text() {
        assert_eq!(
            parse("a < b <x> c"),
            [Span::Text {
                text: "a < b <x> c",
                accent: false
            }]
        );
        assert_eq!(
            parse("<key:>"),
            [Span::Text {
                text: "<key:>",
                accent: false
            }]
        );
    }
}
