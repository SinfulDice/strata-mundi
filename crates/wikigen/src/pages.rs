//! Home, "What's new" and changelog pages, and the history section of entry pages.

use crate::history::{Change, Event};
use crate::html::{escape, link};
use crate::site::{Ctx, Page};
use strata_core::id::Identifier;

/// Sections shown as cards on the home page (path, title key, description key).
const SECTIONS: [(&str, &str, &str); 6] = [
    ("blocks/index.html", "wiki-nav-blocks", "wiki-home-blocks"),
    ("biomes/index.html", "wiki-nav-biomes", "wiki-home-biomes"),
    (
        "mechanics/index.html",
        "wiki-nav-mechanics",
        "wiki-home-mechanics",
    ),
    (
        "commands/index.html",
        "wiki-nav-commands",
        "wiki-home-commands",
    ),
    ("controls.html", "wiki-nav-controls", "wiki-home-controls"),
    ("settings.html", "wiki-nav-settings", "wiki-home-settings"),
];

pub fn home(ctx: &Ctx<'_>) -> Page {
    let path = "index.html".to_owned();
    let cards: String = SECTIONS
        .iter()
        .map(|(to, title, text)| {
            format!(
                "<li><a href=\"{}\">{}</a><p>{}</p></li>",
                link(&path, to),
                escape(&ctx.tr(title)),
                escape(&ctx.tr(text))
            )
        })
        .collect();
    let latest = ctx
        .data
        .history
        .versions
        .last()
        .map_or_else(String::new, |v| {
            format!(
                "<p><a href=\"{}\">{}</a></p>",
                link(&path, "news.html"),
                escape(&ctx.tr_with("wiki-home-latest", &[("version", v.id.clone())]))
            )
        });
    Page {
        title: ctx.tr("wiki-title"),
        body: format!(
            "{}<ul class=\"cards\">{cards}</ul>{latest}",
            ctx.text("pages/home")
        ),
        path,
    }
}

/// Everything added or changed in the latest version.
pub fn news(ctx: &Ctx<'_>) -> Page {
    let path = "news.html".to_owned();
    let body = ctx
        .data
        .history
        .versions
        .last()
        .map_or_else(String::new, |version| {
            version_section(ctx, &path, &version.id, &version.date)
        });
    Page {
        path,
        title: ctx.tr("wiki-nav-news"),
        body,
    }
}

/// Every version, newest first.
pub fn changelog(ctx: &Ctx<'_>) -> Page {
    let path = "changelog.html".to_owned();
    let body: String = ctx
        .data
        .history
        .versions
        .iter()
        .rev()
        .map(|v| version_section(ctx, &path, &v.id, &v.date))
        .collect();
    Page {
        path,
        title: ctx.tr("wiki-changelog"),
        body,
    }
}

fn version_section(ctx: &Ctx<'_>, from: &str, version: &str, date: &str) -> String {
    let items: String = ctx
        .data
        .history
        .in_version(version)
        .map(|(key, event)| {
            format!(
                "<li><a href=\"{}\">{}</a> — {}</li>",
                link(from, &format!("{key}.html")),
                escape(&entry_title(ctx, key)),
                escape(&change_text(ctx, event))
            )
        })
        .collect();
    format!(
        "<h2>{}</h2><ul>{items}</ul><p><a href=\"{}\">{}</a></p>",
        escape(&ctx.tr_with(
            "wiki-version",
            &[("version", version.to_owned()), ("date", date.to_owned())]
        )),
        link(from, "changelog.html"),
        escape(&ctx.tr("wiki-changelog"))
    )
}

/// `## History` of an entry page.
pub fn history_section(ctx: &Ctx<'_>, key: &str) -> String {
    let Some(events) = ctx.data.history.entries.get(key) else {
        ctx.added_in(key);
        return String::new();
    };
    let rows: String = events
        .iter()
        .map(|event| {
            format!(
                "<tr><td>{}</td><td>{}</td></tr>",
                escape(&event.version),
                escape(&change_text(ctx, event))
            )
        })
        .collect();
    format!(
        "<h2>{}</h2><table class=\"data\">{rows}</table>",
        escape(&ctx.tr("wiki-history"))
    )
}

fn change_text(ctx: &Ctx<'_>, event: &Event) -> String {
    match &event.change {
        Change::Added => ctx.tr("wiki-history-added"),
        change => change.text(ctx.lang).unwrap_or_default().to_owned(),
    }
}

/// The title of the page `key` (`blocks/strata/stone`), as its own page shows it.
fn entry_title(ctx: &Ctx<'_>, key: &str) -> String {
    let parts: Vec<&str> = key.split('/').collect();
    let id = |namespace: &str, name: &str| Identifier::new(namespace, name).ok();
    match parts.as_slice() {
        ["blocks", ns, name] => id(ns, name).map(|i| crate::blocks::name(ctx, &i)),
        ["biomes", ns, name] => id(ns, name).map(|i| crate::biomes::name(ctx, &i)),
        ["commands", name] => Some(format!("/{name}")),
        ["mechanics", name] => Some(ctx.tr(&format!("wiki-mechanic-{name}"))),
        _ => None,
    }
    .unwrap_or_else(|| key.to_owned())
}
