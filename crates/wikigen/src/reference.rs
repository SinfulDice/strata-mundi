//! Reference pages: commands, mechanics, controls and settings. Lists come from the game
//! itself (command table, default bindings, options screen), texts from `wiki-content/`.

use crate::html::{escape, link};
use crate::pages::history_section;
use crate::site::{Ctx, Page};
use strata_client::catalog::{REBINDABLE, action_key, settings_catalog};
use strata_client::input::{Binding, Bindings, InputDevice, KeyNames};
use strata_client::settings::{GraphicsSettings, Preset};
use strata_protocol::commands::{COMMANDS, CommandSpec, Runs};

/// Mechanics pages, by text key (`mechanics/<name>`).
const MECHANICS: [&str; 3] = ["world-generation", "saving", "game-modes"];

pub fn pages(ctx: &Ctx<'_>) -> Vec<Page> {
    let mut pages: Vec<Page> = COMMANDS.iter().map(|spec| command(ctx, spec)).collect();
    pages.push(commands_index(ctx));
    pages.extend(MECHANICS.iter().map(|name| mechanic(ctx, name)));
    pages.push(mechanics_index(ctx));
    pages.push(controls(ctx));
    pages.push(settings(ctx));
    pages
}

fn command(ctx: &Ctx<'_>, spec: &CommandSpec) -> Page {
    let path = format!("commands/{}.html", spec.name);
    let key = format!("commands/{}", spec.name);
    let runs = ctx.tr(match spec.runs {
        Runs::Server => "wiki-runs-server",
        Runs::Client => "wiki-runs-client",
    });
    let body = format!(
        "<p><code>{}</code></p><p>{}</p><p class=\"dim\">{}</p>{}{}",
        escape(&ctx.tr(&spec.usage_key())),
        escape(&ctx.tr(&spec.help_key())),
        escape(&runs),
        ctx.text(&key),
        history_section(ctx, &key)
    );
    Page {
        path,
        title: format!("/{}", spec.name),
        body,
    }
}

fn commands_index(ctx: &Ctx<'_>) -> Page {
    let path = "commands/index.html".to_owned();
    let rows: String = COMMANDS
        .iter()
        .map(|spec| {
            format!(
                "<tr><td><a href=\"{}\"><code>/{}</code></a></td><td>{}</td></tr>",
                link(&path, &format!("commands/{}.html", spec.name)),
                spec.name,
                escape(&ctx.tr(&spec.help_key()))
            )
        })
        .collect();
    Page {
        title: ctx.tr("wiki-nav-commands"),
        body: format!(
            "<p>{}</p><table class=\"data\">{rows}</table>",
            escape(&ctx.tr("wiki-commands-intro"))
        ),
        path,
    }
}

fn mechanic(ctx: &Ctx<'_>, name: &str) -> Page {
    let key = format!("mechanics/{name}");
    Page {
        path: format!("{key}.html"),
        title: ctx.tr(&format!("wiki-mechanic-{name}")),
        body: format!("{}{}", ctx.text(&key), history_section(ctx, &key)),
    }
}

fn mechanics_index(ctx: &Ctx<'_>) -> Page {
    let path = "mechanics/index.html".to_owned();
    let items: String = MECHANICS
        .iter()
        .map(|name| {
            format!(
                "<li><a href=\"{}\">{}</a></li>",
                link(&path, &format!("mechanics/{name}.html")),
                escape(&ctx.tr(&format!("wiki-mechanic-{name}")))
            )
        })
        .collect();
    Page {
        path,
        title: ctx.tr("wiki-nav-mechanics"),
        body: format!("<ul class=\"grid\">{items}</ul>"),
    }
}

/// Every action players can rebind, with its default key and gamepad button.
fn controls(ctx: &Ctx<'_>) -> Page {
    let bindings = Bindings::default();
    let names = KeyNames::default();
    let translations = &ctx.data.content.translations;
    // Only the device's own inputs (the game falls back to another device's).
    let label = |action, device| {
        let pad = device == InputDevice::Gamepad;
        let has = bindings
            .bindings_for(action)
            .any(|b| matches!(b, Binding::Pad(_)) == pad);
        if has {
            names.label(&bindings, device, action, translations, ctx.lang)
        } else {
            "—".to_owned()
        }
    };
    let rows: String = REBINDABLE
        .iter()
        .map(|&action| {
            format!(
                "<tr><td>{}</td><td><kbd>{}</kbd></td><td><kbd>{}</kbd></td></tr>",
                escape(&ctx.tr(&action_key(action))),
                escape(&label(action, InputDevice::KeyboardMouse)),
                escape(&label(action, InputDevice::Gamepad))
            )
        })
        .collect();
    let head = format!(
        "<tr><th>{}</th><th>{}</th><th>{}</th></tr>",
        escape(&ctx.tr("wiki-controls-action")),
        escape(&ctx.tr("wiki-controls-keyboard")),
        escape(&ctx.tr("wiki-controls-gamepad"))
    );
    Page {
        path: "controls.html".to_owned(),
        title: ctx.tr("wiki-nav-controls"),
        body: format!(
            "{}<p>{}</p><table class=\"data\">{head}{rows}</table>",
            ctx.text("pages/controls"),
            escape(&ctx.tr("wiki-controls-layout"))
        ),
    }
}

/// Every option by tab with what it does to performance, then the presets compared.
fn settings(ctx: &Ctx<'_>) -> Page {
    let tabs: String = settings_catalog()
        .iter()
        .map(|(tab, keys)| {
            let rows: String = keys
                .iter()
                .map(|key| {
                    format!(
                        "<tr><td>{}</td><td>{}</td></tr>",
                        escape(&ctx.tr(key)),
                        escape(&ctx.tr(&format!("{key}-help")))
                    )
                })
                .collect();
            format!(
                "<h2>{}</h2><table class=\"data\">{rows}</table>",
                escape(&ctx.tr(tab))
            )
        })
        .collect();
    Page {
        path: "settings.html".to_owned(),
        title: ctx.tr("wiki-nav-settings"),
        body: format!("{}{tabs}{}", ctx.text("pages/settings"), presets(ctx)),
    }
}

/// The main graphics values of every preset, side by side.
fn presets(ctx: &Ctx<'_>) -> String {
    let all: Vec<(Preset, GraphicsSettings)> = Preset::ALL
        .iter()
        .map(|&p| (p, GraphicsSettings::for_preset(p)))
        .collect();
    let head: String = all
        .iter()
        .map(|(p, _)| {
            format!(
                "<th>{}</th>",
                escape(&ctx.tr(&format!("preset-{}", p.id())))
            )
        })
        .collect();
    let line = |key: &str, cell: &dyn Fn(&GraphicsSettings) -> String| {
        let cells: String = all
            .iter()
            .map(|(_, g)| format!("<td>{}</td>", escape(&cell(g))))
            .collect();
        format!("<tr><th>{}</th>{cells}</tr>", escape(&ctx.tr(key)))
    };
    let yes_no = |value: bool| ctx.tr(if value { "wiki-yes" } else { "wiki-no" });
    let body = [
        line("option-render-distance", &|g| g.render_distance.to_string()),
        line("option-vertical-distance", &|g| {
            g.vertical_distance.to_string()
        }),
        line("option-render-scale", &|g| format!("{} %", g.render_scale)),
        line("option-fps-limit", &|g| match g.fps_limit {
            0 => ctx.tr("option-unlimited"),
            fps => fps.to_string(),
        }),
        line("option-ambient-occlusion", &|g| yes_no(g.ambient_occlusion)),
        line("option-mipmaps", &|g| yes_no(g.mipmaps)),
    ]
    .concat();
    format!(
        "<h2>{}</h2><table class=\"data\"><tr><th></th>{head}</tr>{body}</table>",
        escape(&ctx.tr("wiki-presets"))
    )
}
