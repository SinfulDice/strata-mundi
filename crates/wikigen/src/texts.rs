//! Written texts: `wiki-content/<lang>/<key>.md`, turned into HTML.

use pulldown_cmark::{Options, Parser, html};
use std::path::{Path, PathBuf};

pub struct Texts {
    root: PathBuf,
}

impl Texts {
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_owned(),
        }
    }

    /// HTML of `<lang>/<key>.md`, `None` when the file is missing or empty.
    pub fn html(&self, lang: &str, key: &str) -> Option<String> {
        let path = self.root.join(lang).join(format!("{key}.md"));
        let markdown = std::fs::read_to_string(path).ok()?;
        (!markdown.trim().is_empty()).then(|| to_html(&markdown))
    }

    pub fn history_path(&self) -> PathBuf {
        self.root.join("history.ron")
    }
}

pub fn to_html(markdown: &str) -> String {
    let mut out = String::new();
    html::push_html(&mut out, Parser::new_ext(markdown, Options::ENABLE_TABLES));
    out
}

/// Text without tags, for the search index.
pub fn plain(html: &str) -> String {
    let mut text = String::with_capacity(html.len());
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                text.push(' ');
            }
            _ if !in_tag => text.push(ch),
            _ => {}
        }
    }
    let words = text.split_whitespace().collect::<Vec<_>>().join(" ");
    [
        ("&lt;", "<"),
        ("&gt;", ">"),
        ("&quot;", "\""),
        ("&#39;", "'"),
        ("&amp;", "&"),
    ]
    .iter()
    .fold(words, |text, (entity, ch)| text.replace(entity, ch))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_becomes_html_and_plain_text() {
        let html = to_html("## Usage\n\nBuild **walls**.");
        assert!(html.contains("<h2>Usage</h2>") && html.contains("<strong>walls</strong>"));
        assert_eq!(plain(&html), "Usage Build walls .");
    }
}
