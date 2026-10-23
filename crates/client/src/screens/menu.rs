//! The menus that can be open, what they ask of the rest of the client, and the model
//! answering for each one.

use super::editor::EditorModel;
use super::inventory::{InventoryModel, InventoryState};
use super::pause::{NoticeModel, OptionsModel, PauseModel};
use super::title::TitleModel;
use super::worlds::{ConfirmModel, CreateForm, CreateModel, WorldsModel};
use crate::hotbar::Hotbar;
use crate::interface_settings::InterfaceSettings;
use crate::ui::{Rect, ScreenDef, ScreenDefs, ScreenModel};
use strata_core::content::Content;
use strata_protocol::{NewWorld, WorldSummary};

pub enum Menu {
    Title,
    Worlds {
        selected: Option<usize>,
        scroll: usize,
    },
    CreateWorld(CreateForm),
    ConfirmDelete {
        id: String,
        name: String,
    },
    Pause,
    Options,
    Inventory(InventoryState),
    HudEditor {
        selected: Option<String>,
    },
    Notice {
        key: &'static str,
    },
}

impl Menu {
    pub fn def<'a>(&self, defs: &'a ScreenDefs) -> &'a ScreenDef {
        match self {
            Self::Title => &defs.title,
            Self::Worlds { .. } => &defs.worlds,
            Self::CreateWorld(_) => &defs.create_world,
            Self::ConfirmDelete { .. } => &defs.confirm_delete,
            Self::Pause => &defs.pause,
            Self::Options => &defs.options,
            Self::Inventory(_) => &defs.inventory,
            Self::HudEditor { .. } => &defs.hud_editor,
            Self::Notice { .. } => &defs.notice,
        }
    }

    /// The pause menu always stops the world; others only with "pause in all menus".
    pub fn pauses(&self, settings: &InterfaceSettings) -> bool {
        matches!(self, Self::Pause) || settings.pause_in_menus
    }
}

/// What a menu asks for after an input.
pub enum Effect {
    Push(Menu),
    Pop,
    /// Back to the game (in a world) or to the title screen.
    CloseAll,
    /// Interface settings changed: apply and write them.
    SettingsChanged,
    Action(UiAction),
}

/// Requests only the session can carry out (they need the launcher or the server).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiAction {
    ShowWorlds,
    Play(String),
    Create(NewWorld),
    Delete(String),
    SaveAndQuit,
    Quit,
}

/// What every model may read or change besides its own state.
pub struct Shared<'a> {
    pub content: &'a Content,
    pub settings: &'a mut InterfaceSettings,
    pub worlds: &'a [WorldSummary],
    /// Atlas place of each world's capture, in list order.
    pub icons: &'a [Option<Rect>],
    pub hotbar: Option<&'a mut Hotbar>,
    pub effects: &'a mut Vec<Effect>,
}

impl Shared<'_> {
    pub fn tr(&self, key: &str) -> String {
        self.content.translations.text(&self.settings.language, key)
    }

    pub fn tr_with(&self, key: &str, args: &[(&str, String)]) -> String {
        self.content
            .translations
            .text_with(&self.settings.language, key, args)
    }
}

/// A model with nothing to show (a menu that needs a world, opened without one).
struct Empty;

impl ScreenModel for Empty {}

pub fn model<'a>(menu: &'a mut Menu, shared: Shared<'a>) -> Box<dyn ScreenModel + 'a> {
    match menu {
        Menu::Title => Box::new(TitleModel { shared }),
        Menu::Worlds { selected, scroll } => Box::new(WorldsModel {
            selected,
            scroll,
            shared,
        }),
        Menu::CreateWorld(form) => Box::new(CreateModel { form, shared }),
        Menu::ConfirmDelete { id, name } => Box::new(ConfirmModel { id, name, shared }),
        Menu::Pause => Box::new(PauseModel { shared }),
        Menu::Options => Box::new(OptionsModel { shared }),
        Menu::HudEditor { selected } => Box::new(EditorModel { selected, shared }),
        Menu::Notice { key } => Box::new(NoticeModel { key, shared }),
        Menu::Inventory(state) => match shared.hotbar {
            Some(hotbar) => Box::new(InventoryModel::new(
                state,
                hotbar,
                shared.content,
                &shared.settings.language,
            )),
            None => Box::new(Empty),
        },
    }
}
