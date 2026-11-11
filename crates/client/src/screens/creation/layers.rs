//! The layers and biome tabs of the generation editor.

use super::screen::{BIOME_ROWS, GenerationModel, LAYER_ROWS};
use crate::ui::OptionRow;
use strata_core::generation::LAYER_THICKNESS;

impl GenerationModel<'_> {
    /// Layer shown on visible row `row` (the top layer first).
    pub(super) fn layer_at(&self, row: usize) -> Option<usize> {
        let count = self.editor().layers().len();
        let from_top = self.form.layer_scroll + row;
        (from_top < count).then(|| count - 1 - from_top)
    }

    /// Scrolls the layer list so the selected layer shows.
    pub(super) fn reveal_layer(&mut self) {
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

    pub(super) fn layer_rows(&self) -> Vec<OptionRow> {
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

    pub(super) fn biome_rows(&self) -> Vec<OptionRow> {
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

    pub(super) fn block_slider(&self) -> Option<(f32, String)> {
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

    pub(super) fn adjust_block(&mut self, step: i32, fraction: Option<f32>) {
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

    pub(super) fn thickness_slider(&self) -> Option<(f32, String)> {
        let editor = self.editor();
        let thickness = editor.layers().get(editor.layer)?.thickness;
        let fraction = f32::from(thickness - 1) / f32::from(*LAYER_THICKNESS.end() - 1);
        let count = thickness.to_string();
        Some((
            fraction,
            self.shared
                .tr_with("ui-gen-layer-thickness", &[("count", count)]),
        ))
    }

    pub(super) fn adjust_thickness(&mut self, step: i32, fraction: Option<f32>) {
        let editor = &mut self.form.editor;
        let Some(current) = editor
            .layers()
            .get(editor.layer)
            .map(|l| i32::from(l.thickness))
        else {
            return;
        };
        let target = match fraction {
            Some(f) => {
                1 + (f.clamp(0.0, 1.0) * f32::from(*LAYER_THICKNESS.end() - 1)).round() as i32
            }
            None => current + step,
        };
        let layer = editor.layer;
        editor.change_thickness(layer, target - current);
    }

    pub(super) fn edit_layers(&mut self, id: &str) {
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
