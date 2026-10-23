//! The menus that can be open, what they ask of the rest of the client, and the model
//! answering for each one.

use super::console::{ConsoleLog, ConsoleModel, ConsoleState};
use super::editor::EditorModel;
use super::inventory::{InventoryModel, InventoryState};
use super::options::{OptionsModel, OptionsState};
use super::pause::{CrashModel, NoticeModel, PauseModel};
use super::title::TitleModel;
use super::worlds::{ConfirmModel, CreateForm, CreateModel, WorldsModel};
use crate::hotbar::Hotbar;
use crate::input::Action;
use crate::settings::Settings;
use crate::ui::{Rect, ScreenDef, ScreenDefs, ScreenModel};
use strata_core::content::Content;
use strata_core::id::Identifier;
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
    Options(OptionsState),
    Inventory(InventoryState),
    HudEditor {
        selected: Option<String>,
    },
    Notice {
        key: &'static str,
    },
    /// The game crashed last time: report it or not.
    Crash,
    Console(ConsoleState),
}

impl Menu {
    pub fn def<'a>(&self, defs: &'a ScreenDefs) -> &'a ScreenDef {
        match self {
            Self::Title => &defs.title,
            Self::Worlds { .. } => &defs.worlds,
            Self::CreateWorld(_) => &defs.create_world,
            Self::ConfirmDelete { .. } => &defs.confirm_delete,
            Self::Pause => &defs.pause,
            Self::Options(_) => &defs.options,
            Self::Inventory(_) => &defs.inventory,
            Self::HudEditor { .. } => &defs.hud_editor,
            Self::Notice { .. } => &defs.notice,
            Self::Crash => &defs.crash,
            Self::Console(_) => &defs.console,
        }
    }

    /// The pause menu always stops the world; others only with "pause in all menus".
    pub fn pauses(&self, settings: &Settings) -> bool {
        matches!(self, Self::Pause) || settings.interface.pause_in_menus
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
    /// Open a pre-filled report of the last crash (true) or forget it (false).
    SettleCrash(bool),
    /// A console line for the server, without the `/`.
    Command(String),
    PlayCinematic(Identifier),
}

/// What every model may read or change besides its own state.
pub struct Shared<'a> {
    pub content: &'a Content,
    pub settings: &'a mut Settings,
    pub worlds: &'a [WorldSummary],
    /// Atlas place of each world's capture, in list order.
    pub icons: &'a [Option<Rect>],
    pub hotbar: Option<&'a mut Hotbar>,
    /// The focused element (id, index), for models that describe it.
    pub focus: Option<(String, usize)>,
    /// Keyboard and gamepad labels of an action's bindings.
    pub key_labels: &'a dyn Fn(Action) -> (String, String),
    /// A newer version of the game, for the title screen.
    pub newer_version: Option<&'a str>,
    pub console: &'a ConsoleLog,
    /// Resource packs that can be enabled.
    pub packs: &'a [String],
    pub effects: &'a mut Vec<Effect>,
}

impl Shared<'_> {
    pub fn tr(&self, key: &str) -> String {
        self.content
            .translations
            .text(&self.settings.interface.language, key)
    }

    pub fn tr_with(&self, key: &str, args: &[(&str, String)]) -> String {
        self.content
            .translations
            .text_with(&self.settings.interface.language, key, args)
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
        Menu::Options(state) => Box::new(OptionsModel {
            state,
            key_labels: shared.key_labels,
            shared,
        }),
        Menu::HudEditor { selected } => Box::new(EditorModel { selected, shared }),
        Menu::Notice { key } => Box::new(NoticeModel { key, shared }),
        Menu::Crash => Box::new(CrashModel { shared }),
        Menu::Console(state) => Box::new(ConsoleModel {
            state,
            log: shared.console,
            shared,
        }),
        Menu::Inventory(state) => match shared.hotbar {
            Some(hotbar) => Box::new(InventoryModel::new(
                state,
                hotbar,
                shared.content,
                &shared.settings.interface.language,
            )),
            None => Box::new(Empty),
        },
    }
}
