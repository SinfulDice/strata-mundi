//! New world: the main form (name, seed, rules, world type) and the generation editor
//! opened from it, two pages of one menu sharing what is being filled in.

mod editor;
mod preview;
mod screen;

pub use editor::GenerationEditor;
pub use preview::{Preview, PreviewImage};
pub use screen::GenerationModel;

use super::menu::{Effect, Shared, UiAction};
use super::worlds::{edit, mode_key};
use crate::generation::{GenerationCatalog, MapRequest, MapSource};
use crate::settings::MapPreview;
use crate::ui::ScreenModel;
use std::hash::{BuildHasher, RandomState};
use std::sync::Arc;
use strata_core::content::{Content, ItemKind};
use strata_core::coords::DEFAULT_WORLD_BORDER;
use strata_core::id::Identifier;
use strata_protocol::{GameMode, NewWorld};

const MAX_NAME: usize = 32;
const MAX_SEED: usize = 32;
const MAX_BORDER_DIGITS: usize = 8;
const MIN_BORDER: i32 = 1_000;
/// Pixels of the map on each side and blocks per pixel (`MapPreview::Detailed`).
const MAP_PIXELS: u32 = 96;
const MAP_SCALE: u32 = 20;

/// What the generation screens get from the program around the client.
#[derive(Default)]
pub struct GenerationSources {
    pub catalog: Arc<GenerationCatalog>,
    pub maps: Option<Arc<dyn MapSource>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Main,
    Generation,
}

/// What the "new world" screens are filling in.
pub struct CreateForm {
    pub name: String,
    default_name: String,
    pub seed: String,
    /// Used when the seed is left empty, so the world is the one previewed.
    random_seed: String,
    pub border: String,
    pub mode: GameMode,
    pub fluids: bool,
    pub day_cycle: bool,
    pub fire_spread: bool,
    pub fire_destroy: bool,
    pub editor: GenerationEditor,
    pub page: Page,
    pub tab: usize,
    pub preview: Option<Preview>,
    /// The preview thread could not start: not tried again.
    preview_failed: bool,
    pub code_field: String,
    pub code_status: Option<Result<(), crate::generation::GenerationError>>,
    pub layer_scroll: usize,
    pub biome_scroll: usize,
}

impl CreateForm {
    pub fn new(shared: &Shared<'_>) -> Self {
        let default_name = shared.tr("ui-new-world-default-name");
        let catalog = Arc::clone(&shared.generation.catalog);
        let blocks = layer_blocks(shared.content, &catalog);
        Self {
            name: default_name.clone(),
            default_name,
            seed: String::new(),
            random_seed: random_seed(),
            border: DEFAULT_WORLD_BORDER.to_string(),
            mode: GameMode::Creative,
            fluids: true,
            day_cycle: true,
            fire_spread: true,
            fire_destroy: true,
            editor: GenerationEditor::new(catalog, blocks),
            page: Page::Main,
            tab: 0,
            preview: None,
            preview_failed: false,
            code_field: String::new(),
            code_status: None,
            layer_scroll: 0,
            biome_scroll: 0,
        }
    }

    fn seed(&self) -> &str {
        match self.seed.trim() {
            "" => &self.random_seed,
            seed => seed,
        }
    }

    fn world(&self) -> NewWorld {
        let border = self
            .border
            .parse::<i32>()
            .unwrap_or(DEFAULT_WORLD_BORDER)
            .clamp(MIN_BORDER, DEFAULT_WORLD_BORDER);
        let name = self.name.trim();
        NewWorld {
            name: if name.is_empty() {
                &self.default_name
            } else {
                name
            }
            .to_owned(),
            seed: self.seed().to_owned(),
            mode: self.mode,
            border,
            fluids: self.fluids,
            day_cycle: self.day_cycle,
            fire_spread: self.fire_spread,
            fire_destroy: self.fire_destroy,
            generation: self.editor.settings.clone(),
        }
    }

    pub fn map_request(&self, quality: MapPreview) -> Option<MapRequest> {
        let (pixels, scale) = match quality {
            MapPreview::Off => return None,
            MapPreview::Coarse => (MAP_PIXELS / 2, MAP_SCALE * 2),
            MapPreview::Detailed => (MAP_PIXELS, MAP_SCALE),
        };
        Some(MapRequest {
            seed: self.seed().to_owned(),
            settings: self.editor.settings.clone(),
            size: (pixels, pixels),
            scale,
        })
    }

    /// While the editor is open: asks for the map of the current settings and returns
    /// the newest one drawn.
    pub fn refresh_preview(
        &mut self,
        quality: MapPreview,
        maps: Option<&Arc<dyn MapSource>>,
    ) -> Option<&PreviewImage> {
        if self.page != Page::Generation {
            return None;
        }
        let request = self.map_request(quality)?;
        if self.preview.is_none() && !self.preview_failed {
            self.preview = Preview::start(Arc::clone(maps?));
            self.preview_failed = self.preview.is_none();
        }
        let preview = self.preview.as_mut()?;
        preview.request(request);
        preview.poll();
        preview.image.as_ref()
    }

    pub fn open_page(&mut self, page: Page, effects: &mut Vec<Effect>) {
        self.page = page;
        self.tab = 0;
        if page == Page::Main
            && let Some(preview) = &mut self.preview
        {
            preview.cancel();
        }
        effects.push(Effect::ClearFocus);
    }
}

/// Blocks that can be placed, plus those of the types' own layers (bedrock).
fn layer_blocks(content: &Content, catalog: &GenerationCatalog) -> Vec<Identifier> {
    let mut blocks: Vec<Identifier> = content
        .items
        .iter()
        .filter_map(|(_, _, item)| match item.kind {
            ItemKind::Block(block) => content.blocks.identifier(block).cloned(),
            ItemKind::Flat(_) => None,
        })
        .collect();
    let own = catalog
        .types
        .iter()
        .flat_map(|t| t.flat.iter().flat_map(|f| &f.layers));
    for layer in own {
        if !blocks.contains(&layer.block) {
            blocks.push(layer.block.clone());
        }
    }
    blocks
}

fn random_seed() -> String {
    let random = RandomState::new().hash_one(std::time::SystemTime::now()) as i64;
    random.to_string()
}

/// The main page of the new world menu.
pub struct CreateModel<'a> {
    pub form: &'a mut CreateForm,
    pub shared: Shared<'a>,
}

impl CreateModel<'_> {
    fn toggle(&self, key: &str, arg: &str, on: bool) -> String {
        self.shared.tr_with(key, &[(arg, on_off(on).to_owned())])
    }
}

impl ScreenModel for CreateModel<'_> {
    fn text(&self, id: &str) -> Option<String> {
        let form = &self.form;
        match id {
            "name" => Some(form.name.clone()),
            "seed" => Some(form.seed.clone()),
            "border" => Some(form.border.clone()),
            "mode" => Some(self.shared.tr_with(
                "ui-new-world-mode",
                &[("mode", self.shared.tr(mode_key(form.mode)))],
            )),
            "fluids" => Some(self.toggle("ui-new-world-fluids", "flow", form.fluids)),
            "day_cycle" => Some(self.toggle("ui-new-world-day-cycle", "cycle", form.day_cycle)),
            "fire_spread" => {
                Some(self.toggle("ui-new-world-fire-spread", "spread", form.fire_spread))
            }
            "fire_destroy" => {
                Some(self.toggle("ui-new-world-fire-destroy", "destroy", form.fire_destroy))
            }
            _ => None,
        }
    }

    fn slider(&self, id: &str) -> Option<(f32, String)> {
        (id == "world_type").then(|| screen::type_slider(&self.form.editor, &self.shared))
    }

    fn adjust(&mut self, id: &str, _: usize, step: i32, fraction: Option<f32>) {
        if id == "world_type" {
            screen::adjust_type(&mut self.form.editor, step, fraction);
        }
    }

    fn type_text(&mut self, id: &str, text: &str) {
        match id {
            "name" => edit(&mut self.form.name, text, MAX_NAME, |_| true),
            "seed" => edit(&mut self.form.seed, text, MAX_SEED, |_| true),
            "border" => edit(&mut self.form.border, text, MAX_BORDER_DIGITS, |c| {
                c.is_ascii_digit()
            }),
            _ => {}
        }
    }

    fn activate(&mut self, id: &str, _: usize) {
        let form = &mut *self.form;
        let flag = match id {
            "fluids" => Some(&mut form.fluids),
            "day_cycle" => Some(&mut form.day_cycle),
            "fire_spread" => Some(&mut form.fire_spread),
            "fire_destroy" => Some(&mut form.fire_destroy),
            _ => None,
        };
        if let Some(flag) = flag {
            *flag = !*flag;
            return;
        }
        let effect = match id {
            "customize" => {
                form.open_page(Page::Generation, self.shared.effects);
                return;
            }
            "create" => Effect::Action(UiAction::Create(form.world())),
            "back" => Effect::Pop,
            // Survival arrives in v0.3: creative is the only mode for now ("mode").
            _ => return,
        };
        self.shared.effects.push(effect);
    }
}

fn on_off(on: bool) -> &'static str {
    if on { "on" } else { "off" }
}

#[cfg(test)]
#[path = "screen_tests.rs"]
mod screen_tests;
