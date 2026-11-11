//! The generation editor page: tabs of sliders, layers or biome, and the shareable code,
//! beside the map of what they generate.

use super::editor::{GenerationEditor, MAX_THICKNESS, Refused, Slider};
use super::{CreateForm, Page};
use crate::screens::menu::{Effect, Shared};
use crate::screens::worlds::edit;
use crate::settings::MapPreview;
use crate::ui::{OptionRow, Rect, ScreenModel, UiAtlas};
use strata_core::id::Identifier;

/// Rows visible at once (match `world_generation.ron`).
pub const LAYER_ROWS: usize = 7;
pub const BIOME_ROWS: usize = 10;
/// Characters of the code per line shown, and lines.
const CODE_LINE: usize = 30;
const CODE_LINES: usize = 3;
const MAX_CODE: usize = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    General,
    Layers,
    Biome,
    Code,
}

pub struct GenerationModel<'a> {
    pub form: &'a mut CreateForm,
    pub shared: Shared<'a>,
}

/// Position and caption of the world type slider (both pages).
pub fn type_slider(editor: &GenerationEditor, shared: &Shared<'_>) -> (f32, String) {
    let types = &editor.catalog().types;
    let index = types
        .iter()
        .position(|t| t.id == editor.settings.world_type)
        .unwrap_or(0);
    let fraction = index as f32 / types.len().saturating_sub(1).max(1) as f32;
    let name = shared.tr(&editor.settings.world_type.translation_key("world-type"));
    (fraction, shared.tr_with("ui-gen-type", &[("type", name)]))
}

pub fn adjust_type(editor: &mut GenerationEditor, step: i32, fraction: Option<f32>) {
    let count = editor.catalog().types.len();
    let current = editor
        .catalog()
        .types
        .iter()
        .position(|t| t.id == editor.settings.world_type)
        .unwrap_or(0);
    let step = match fraction {
        Some(f) => {
            (f.clamp(0.0, 1.0) * count.saturating_sub(1) as f32).round() as i32 - current as i32
        }
        None => step,
    };
    if step != 0 {
        editor.cycle_type(step);
    }
}

fn slider_of(id: &str) -> Option<Slider> {
    Some(match id {
        "biome_size" => Slider::BiomeSize,
        "sea_level" => Slider::SeaLevel,
        "caves" => Slider::Caves,
        "ores" => Slider::Ores,
        _ => return None,
    })
}

impl GenerationModel<'_> {
    fn tabs_list(&self) -> Vec<Tab> {
        let editor = &self.form.editor;
        let own = if editor.flat().is_some() {
            Some(Tab::Layers)
        } else if editor.world_type().is_some_and(|t| t.biome.is_some()) {
            Some(Tab::Biome)
        } else {
            None
        };
        std::iter::once(Tab::General)
            .chain(own)
            .chain(std::iter::once(Tab::Code))
            .collect()
    }

    fn tab(&self) -> Tab {
        let tabs = self.tabs_list();
        tabs.get(self.form.tab).copied().unwrap_or(Tab::General)
    }

    fn editor(&self) -> &GenerationEditor {
        &self.form.editor
    }

    fn block_name(&self, block: &Identifier) -> String {
        self.shared.tr(&block.translation_key("block"))
    }

    fn slider_caption(&self, slider: Slider) -> String {
        let editor = self.editor();
        let value = editor.value(slider);
        let shown = match slider {
            Slider::SeaLevel => value.to_string(),
            _ => self
                .shared
                .tr_with("ui-gen-percent", &[("value", value.to_string())]),
        };
        let shown = if editor.is_default(slider) {
            self.shared.tr_with("ui-gen-default", &[("value", shown)])
        } else {
            shown
        };
        let key = match slider {
            Slider::BiomeSize => "ui-gen-biome-size",
            Slider::SeaLevel => "ui-gen-sea-level",
            Slider::Caves => "ui-gen-caves",
            Slider::Ores => "ui-gen-ores",
        };
        self.shared.tr_with(key, &[("value", shown)])
    }

    /// Layer shown on visible row `row` (the top layer first).
    fn layer_at(&self, row: usize) -> Option<usize> {
        let count = self.editor().layers().len();
        let from_top = self.form.layer_scroll + row;
        (from_top < count).then(|| count - 1 - from_top)
    }

    /// Scrolls the layer list so the selected layer shows.
    fn reveal_layer(&mut self) {
        let count = self.editor().layers().len();
        let from_top = count.saturating_sub(1 + self.form.editor.layer);
        let scroll = &mut self.form.layer_scroll;
        if from_top < *scroll {
            *scroll = from_top;
        } else if from_top >= *scroll + LAYER_ROWS {
            *scroll = from_top + 1 - LAYER_ROWS;
        }
        *scroll = (*scroll).min(count.saturating_sub(LAYER_ROWS));
    }

    fn layer_rows(&self) -> Vec<OptionRow> {
        let editor = self.editor();
        (0..LAYER_ROWS)
            .filter_map(|row| self.layer_at(row))
            .map(|index| {
                let layer = &editor.layers()[index];
                OptionRow {
                    label: self.block_name(&layer.block),
                    value: self.shared.tr_with(
                        "ui-gen-thickness",
                        &[("count", layer.thickness.to_string())],
                    ),
                    gauge: None,
                    highlight: index == editor.layer,
                }
            })
            .collect()
    }

    fn biome_rows(&self) -> Vec<OptionRow> {
        let chosen = self.editor().biome();
        self.editor()
            .catalog()
            .biomes
            .iter()
            .skip(self.form.biome_scroll)
            .take(BIOME_ROWS)
            .map(|biome| OptionRow {
                label: self.shared.tr(&biome.translation_key("biome")),
                value: String::new(),
                gauge: None,
                highlight: Some(biome) == chosen,
            })
            .collect()
    }

    fn block_slider(&self) -> Option<(f32, String)> {
        let editor = self.editor();
        let block = &editor.layers().get(editor.layer)?.block;
        let blocks = editor.blocks();
        let index = blocks.iter().position(|b| b == block).unwrap_or(0);
        let fraction = index as f32 / blocks.len().saturating_sub(1).max(1) as f32;
        let name = self.block_name(block);
        Some((
            fraction,
            self.shared.tr_with("ui-gen-block", &[("block", name)]),
        ))
    }

    fn adjust_block(&mut self, step: i32, fraction: Option<f32>) {
        let editor = &mut self.form.editor;
        let Some(current) = editor.layers().get(editor.layer).map(|l| l.block.clone()) else {
            return;
        };
        let blocks = editor.blocks();
        if blocks.is_empty() {
            return;
        }
        let index = blocks.iter().position(|b| *b == current).unwrap_or(0) as i32;
        let last = blocks.len() as i32 - 1;
        let next = match fraction {
            Some(f) => (f.clamp(0.0, 1.0) * last as f32).round() as i32,
            None => (index + step).clamp(0, last),
        };
        let block = blocks[next as usize].clone();
        editor.set_block(block);
    }

    fn thickness_slider(&self) -> Option<(f32, String)> {
        let editor = self.editor();
        let thickness = editor.layers().get(editor.layer)?.thickness;
        let fraction = f32::from(thickness - 1) / f32::from(MAX_THICKNESS - 1);
        let count = thickness.to_string();
        Some((
            fraction,
            self.shared
                .tr_with("ui-gen-layer-thickness", &[("count", count)]),
        ))
    }

    fn adjust_thickness(&mut self, step: i32, fraction: Option<f32>) {
        let editor = &mut self.form.editor;
        let Some(current) = editor
            .layers()
            .get(editor.layer)
            .map(|l| i32::from(l.thickness))
        else {
            return;
        };
        let target = match fraction {
            Some(f) => 1 + (f.clamp(0.0, 1.0) * f32::from(MAX_THICKNESS - 1)).round() as i32,
            None => current + step,
        };
        let layer = editor.layer;
        editor.change_thickness(layer, target - current);
    }

    fn code_line(&self, line: usize) -> String {
        let code: Vec<char> = self.editor().code().chars().collect();
        let mut chunks = code.chunks(CODE_LINE);
        let text: String = chunks
            .nth(line)
            .map(|c| c.iter().collect())
            .unwrap_or_default();
        let more = code.len() > CODE_LINE * CODE_LINES;
        if line + 1 == CODE_LINES && more {
            format!("{text}…")
        } else {
            text
        }
    }

    fn code_status(&self) -> String {
        match &self.form.code_status {
            None => String::new(),
            Some(Ok(())) => self.shared.tr("ui-gen-code-applied"),
            Some(Err(refused)) => self.refusal(refused),
        }
    }

    fn refusal(&self, refused: &Refused) -> String {
        let id = refused.id.clone().unwrap_or_default();
        self.shared.tr_with(refused.message, &[("id", id)])
    }

    fn preview_status(&self) -> String {
        let quality = self.shared.settings.graphics.map_preview;
        let preview = self.form.preview.as_ref();
        let key = if quality == MapPreview::Off {
            "ui-gen-preview-off"
        } else if let Some(error) = preview.and_then(|p| p.error.as_ref()) {
            return self.shared.tr_with(
                error.message,
                &[("id", error.id.clone().unwrap_or_default())],
            );
        } else if preview.is_none_or(|p| p.is_busy()) {
            "ui-gen-preview-busy"
        } else {
            "ui-gen-preview-legend"
        };
        self.shared.tr(key)
    }

    fn apply_code(&mut self) {
        let field = self.form.code_field.clone();
        let result = self.form.editor.apply_code(&field);
        if result.is_ok() {
            self.form.tab = self.tabs_list().len() - 1;
            self.form.layer_scroll = 0;
            self.form.biome_scroll = 0;
        }
        self.form.code_status = Some(result);
    }

    fn edit_layers(&mut self, id: &str) {
        let editor = &mut self.form.editor;
        match id {
            "add" => {
                let block = editor.layers().get(editor.layer).map(|l| l.block.clone());
                if let Some(block) = block {
                    editor.add_layer(block);
                }
            }
            "remove" => editor.remove_layer(),
            "up" => editor.move_layer(true),
            "down" => editor.move_layer(false),
            _ => return,
        }
        self.reveal_layer();
    }
}

impl ScreenModel for GenerationModel<'_> {
    fn tabs(&self, _: &str) -> Vec<String> {
        self.tabs_list()
            .into_iter()
            .map(|tab| {
                self.shared.tr(match tab {
                    Tab::General => "ui-gen-tab-general",
                    Tab::Layers => "ui-gen-tab-layers",
                    Tab::Biome => "ui-gen-tab-biome",
                    Tab::Code => "ui-gen-tab-code",
                })
            })
            .collect()
    }

    fn selected(&self, id: &str) -> Option<usize> {
        (id == "tabs").then_some(self.form.tab)
    }

    fn shown(&self, id: &str) -> bool {
        let tab = self.tab();
        match id {
            "general" => tab == Tab::General,
            "layers" => tab == Tab::Layers,
            "biome" => tab == Tab::Biome,
            "code" => tab == Tab::Code,
            _ => true,
        }
    }

    fn slider(&self, id: &str) -> Option<(f32, String)> {
        match id {
            "world_type" => Some(type_slider(self.editor(), &self.shared)),
            "block" => self.block_slider(),
            "thickness" => self.thickness_slider(),
            _ => {
                let slider = slider_of(id)?;
                Some((self.editor().fraction(slider), self.slider_caption(slider)))
            }
        }
    }

    fn adjust(&mut self, id: &str, index: usize, step: i32, fraction: Option<f32>) {
        match id {
            "world_type" => {
                adjust_type(&mut self.form.editor, step, fraction);
                self.form.layer_scroll = 0;
            }
            "block" => self.adjust_block(step, fraction),
            "thickness" => self.adjust_thickness(step, fraction),
            "layers" => {
                if let Some(layer) = self.layer_at(index) {
                    self.form.editor.change_thickness(layer, step);
                }
            }
            _ => {
                let Some(slider) = slider_of(id) else {
                    return;
                };
                let editor = &mut self.form.editor;
                match fraction {
                    Some(fraction) => editor.set_fraction(slider, fraction),
                    None => editor.step(slider, step),
                }
            }
        }
    }

    fn option_rows(&self, id: &str) -> Vec<OptionRow> {
        match id {
            "layers" => self.layer_rows(),
            "biomes" => self.biome_rows(),
            _ => Vec::new(),
        }
    }

    fn scroll(&mut self, id: &str, rows: i32) {
        let (scroll, count, visible) = match id {
            "layers" => (
                &mut self.form.layer_scroll,
                self.form.editor.layers().len(),
                LAYER_ROWS,
            ),
            "biomes" => (
                &mut self.form.biome_scroll,
                self.form.editor.catalog().biomes.len(),
                BIOME_ROWS,
            ),
            _ => return,
        };
        let max = count.saturating_sub(visible) as i32;
        *scroll = (*scroll as i32 + rows).clamp(0, max) as usize;
    }

    fn text(&self, id: &str) -> Option<String> {
        Some(match id {
            "code_0" => self.code_line(0),
            "code_1" => self.code_line(1),
            "code_2" => self.code_line(2),
            "code_field" => self.form.code_field.clone(),
            "code_status" => self.code_status(),
            "preview_status" => self.preview_status(),
            _ => return None,
        })
    }

    fn image(&self, id: &str) -> Option<Rect> {
        let quality = self.shared.settings.graphics.map_preview;
        let image = self.form.preview.as_ref()?.image.as_ref();
        (id == "preview" && quality != MapPreview::Off)
            .then_some(image)
            .flatten()
            .and_then(|image| UiAtlas::preview_slot(image.size))
    }

    fn type_text(&mut self, id: &str, text: &str) {
        if id == "code_field" {
            edit(&mut self.form.code_field, text, MAX_CODE, |c| {
                c.is_ascii_alphanumeric() || c == '-' || c == ' '
            });
            self.form.code_status = None;
        }
    }

    fn paste(&mut self, id: &str, text: &str) {
        if id == "code_field" {
            self.form.code_field = text.trim().chars().take(MAX_CODE).collect();
            self.apply_code();
        }
    }

    fn activate(&mut self, id: &str, index: usize) {
        match id {
            "tabs" => {
                self.form.tab = index.min(self.tabs_list().len() - 1);
                self.shared.effects.push(Effect::ClearFocus);
            }
            "layers" => {
                if let Some(layer) = self.layer_at(index) {
                    self.form.editor.layer = layer;
                }
            }
            "biomes" => {
                let biome = self
                    .editor()
                    .catalog()
                    .biomes
                    .get(self.form.biome_scroll + index)
                    .cloned();
                if let Some(biome) = biome {
                    self.form.editor.set_biome(biome);
                }
            }
            "copy" => {
                let code = self.editor().code();
                self.shared.effects.push(Effect::Copy(code));
            }
            "paste" => self.shared.effects.push(Effect::Paste("code_field".into())),
            "apply" => self.apply_code(),
            "done" => self.form.open_page(Page::Main, self.shared.effects),
            _ => self.edit_layers(id),
        }
    }

    fn back(&mut self) -> bool {
        self.form.open_page(Page::Main, self.shared.effects);
        true
    }
}
