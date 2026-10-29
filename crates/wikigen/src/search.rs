//! Search index of one language: a script defining `SEARCH_INDEX`, so search works when
//! the wiki is opened from disk (browsers refuse to load JSON files from `file://`).

use crate::site::Page;
use crate::texts::plain;
use serde::Serialize;

/// Characters of text kept per page: enough to find a page by what it talks about.
const SUMMARY: usize = 600;

#[derive(Serialize)]
struct Entry<'a> {
    /// Title.
    t: &'a str,
    /// Path from the language folder.
    u: &'a str,
    /// Words of the page.
    s: String,
}

pub fn index(pages: &[Page]) -> String {
    let entries: Vec<Entry<'_>> = pages
        .iter()
        .map(|page| Entry {
            t: &page.title,
            u: &page.path,
            s: plain(&page.body).chars().take(SUMMARY).collect(),
        })
        .collect();
    let json = serde_json::to_string(&entries).unwrap_or_else(|_| "[]".to_owned());
    // `</` would close a script element if the index were ever inlined.
    format!("window.SEARCH_INDEX = {};\n", json.replace("</", "<\\/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_index_is_a_script() {
        let pages = [Page {
            path: "a.html".into(),
            title: "Pierre".into(),
            body: "<p>Grise</p>".into(),
        }];
        let script = index(&pages);
        assert!(script.starts_with("window.SEARCH_INDEX = [{"));
        assert!(script.contains("\"u\":\"a.html\"") && script.contains("Grise"));
    }
}
