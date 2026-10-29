//! The frame of every page (header, navigation, search, language and theme switches) and
//! small HTML helpers.

use crate::site::{Ctx, Page};
use strata_core::lang::LANGUAGES;

/// Main sections, linked from every page (path, title key).
const NAVIGATION: [(&str, &str); 7] = [
    ("blocks/index.html", "wiki-nav-blocks"),
    ("biomes/index.html", "wiki-nav-biomes"),
    ("mechanics/index.html", "wiki-nav-mechanics"),
    ("commands/index.html", "wiki-nav-commands"),
    ("controls.html", "wiki-nav-controls"),
    ("settings.html", "wiki-nav-settings"),
    ("news.html", "wiki-nav-news"),
];

pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

/// From the page at `from` to `to`, both relative to the language folder.
pub fn link(from: &str, to: &str) -> String {
    format!("{}{to}", "../".repeat(from.matches('/').count()))
}

/// From the page at `from` to a shared file at the root of the site (`images/…`).
pub fn shared(from: &str, to: &str) -> String {
    format!("{}../{to}", "../".repeat(from.matches('/').count()))
}

/// An `<img>` of a rendered image, if it exists in the built site.
pub fn image(ctx: &Ctx<'_>, from: &str, kind: &str, key: &str, alt: &str, class: &str) -> String {
    if !ctx
        .data
        .images
        .join(kind)
        .join(format!("{key}.png"))
        .is_file()
    {
        return String::new();
    }
    format!(
        "<img class=\"{class}\" src=\"{}\" alt=\"{}\">",
        shared(from, &format!("images/{kind}/{key}.png")),
        escape(alt)
    )
}

/// A two-column table of facts beside the text, like the Minecraft wiki.
pub fn infobox(title: &str, picture: &str, rows: &[(String, String)]) -> String {
    let mut out = format!(
        "<aside class=\"infobox\"><h2>{}</h2>{picture}<table>",
        escape(title)
    );
    for (name, value) in rows {
        out.push_str(&format!(
            "<tr><th>{}</th><td>{value}</td></tr>",
            escape(name)
        ));
    }
    out.push_str("</table></aside>");
    out
}

pub fn layout(ctx: &Ctx<'_>, page: &Page) -> String {
    let from = page.path.as_str();
    let nav: String = NAVIGATION
        .iter()
        .map(|(path, key)| {
            format!(
                "<a href=\"{}\">{}</a>",
                link(from, path),
                escape(&ctx.tr(key))
            )
        })
        .collect();
    let languages: String = LANGUAGES
        .iter()
        .map(|&lang| {
            let class = if lang == ctx.lang {
                " class=\"current\""
            } else {
                ""
            };
            let target = format!(
                "{}../{lang}/{from}",
                "../".repeat(from.matches('/').count())
            );
            format!(
                "<a{class} href=\"{target}\" hreflang=\"{lang}\">{}</a>",
                lang.to_uppercase()
            )
        })
        .collect();
    format!(
        include_str!("../assets/page.html"),
        // `lol` is the code of the Mongo language: LOLcat is a private English variant.
        lang = if ctx.lang == "lol" {
            "en-x-lolcat"
        } else {
            ctx.lang
        },
        title = escape(&page.title),
        site = escape(&ctx.tr("wiki-title")),
        style = shared(from, "style.css"),
        script = shared(from, "wiki.js"),
        index = link(from, "search-index.js"),
        home = link(from, "index.html"),
        nav = nav,
        languages = languages,
        search = escape(&ctx.tr("wiki-search")),
        theme = escape(&ctx.tr("wiki-theme")),
        body = page.body,
        footer = escape(&ctx.tr("wiki-footer")),
        root = link(from, ""),
        folder = ctx.lang,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_climb_back_to_the_language_folder() {
        assert_eq!(
            link("blocks/strata/stone.html", "news.html"),
            "../../news.html"
        );
        assert_eq!(link("index.html", "news.html"), "news.html");
        assert_eq!(shared("blocks/index.html", "style.css"), "../../style.css");
        assert_eq!(escape("<a & 'b'>"), "&lt;a &amp; &#39;b&#39;&gt;");
    }
}
