//! Screen descriptions read from `assets/ui/*.ron`: what is where. Behaviour lives in Rust
//! screen models. Internal format until v1.1, when mods will be able to change it.

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename = "Screen")]
pub struct ScreenDef {
    pub root: Vec<Node>,
}

/// Where a node attaches to its parent; `at` is an offset from that point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Anchor {
    TopLeft,
    Top,
    TopRight,
    Left,
    Center,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}

#[derive(Debug, Clone, Deserialize)]
pub enum Node {
    /// Darkens everything behind a menu.
    Shade,
    Panel {
        id: String,
        anchor: Anchor,
        at: (i32, i32),
        size: (u32, u32),
        children: Vec<Node>,
    },
    /// `text` is a translation key (it may hold markup, see `ui::rich`), or `@name` for
    /// text supplied by the screen model.
    Label {
        id: String,
        anchor: Anchor,
        at: (i32, i32),
        text: String,
        #[serde(default)]
        boxed: bool,
        #[serde(default)]
        dim: bool,
        /// Whole-number magnification (titles, the logo).
        #[serde(default = "one")]
        size: u32,
    },
    /// `text` as for labels; `@name` gives buttons whose caption changes (on/off, modes).
    Button {
        id: String,
        anchor: Anchor,
        at: (i32, i32),
        size: (u32, u32),
        text: String,
    },
    /// A value moved with left/right or by dragging; the model gives position and caption.
    Slider {
        id: String,
        anchor: Anchor,
        at: (i32, i32),
        size: (u32, u32),
    },
    /// Scrolling list of rows (title, detail, optional icon), e.g. the saved worlds.
    List {
        id: String,
        anchor: Anchor,
        at: (i32, i32),
        size: (u32, u32),
        row_height: u32,
    },
    /// On-screen keyboard: one string per row, one key per character (gamepad typing).
    KeyGrid {
        id: String,
        anchor: Anchor,
        at: (i32, i32),
        rows: Vec<String>,
        key: u32,
    },
    TextField {
        id: String,
        anchor: Anchor,
        at: (i32, i32),
        size: (u32, u32),
        placeholder: String,
    },
    Tabs {
        id: String,
        anchor: Anchor,
        at: (i32, i32),
        size: (u32, u32),
    },
    ItemGrid {
        id: String,
        anchor: Anchor,
        at: (i32, i32),
        columns: u32,
        rows: u32,
    },
    /// A row of item slots (the hotbar).
    Slots {
        id: String,
        anchor: Anchor,
        at: (i32, i32),
        count: u32,
    },
    Crosshair {
        id: String,
        anchor: Anchor,
        at: (i32, i32),
    },
}

fn one() -> u32 {
    1
}

impl Node {
    pub fn at(&self) -> Option<(i32, i32)> {
        match self {
            Self::Shade => None,
            Self::Panel { at, .. }
            | Self::Label { at, .. }
            | Self::Button { at, .. }
            | Self::Slider { at, .. }
            | Self::List { at, .. }
            | Self::KeyGrid { at, .. }
            | Self::TextField { at, .. }
            | Self::Tabs { at, .. }
            | Self::ItemGrid { at, .. }
            | Self::Slots { at, .. }
            | Self::Crosshair { at, .. } => Some(*at),
        }
    }

    /// The id used for focus, models and HUD layout, if the node has one.
    pub fn id(&self) -> Option<&str> {
        match self {
            Self::Shade => None,
            Self::Panel { id, .. }
            | Self::Label { id, .. }
            | Self::Button { id, .. }
            | Self::Slider { id, .. }
            | Self::List { id, .. }
            | Self::KeyGrid { id, .. }
            | Self::TextField { id, .. }
            | Self::Tabs { id, .. }
            | Self::ItemGrid { id, .. }
            | Self::Slots { id, .. }
            | Self::Crosshair { id, .. } => Some(id),
        }
    }
}

impl ScreenDef {
    /// Offset (`at`) of the top-level element `id`, as written in the file.
    pub fn offset_of(&self, id: &str) -> Option<(i32, i32)> {
        self.root
            .iter()
            .find(|node| node.id() == Some(id))
            .and_then(Node::at)
    }

    /// Translation keys of every fixed text, for the missing-translation test.
    pub fn translation_keys(&self) -> Vec<String> {
        let mut keys = Vec::new();
        collect_keys(&self.root, &mut keys);
        keys
    }
}

fn collect_keys(nodes: &[Node], keys: &mut Vec<String>) {
    for node in nodes {
        match node {
            Node::Panel { children, .. } => collect_keys(children, keys),
            Node::Label { text, .. } | Node::Button { text, .. } if !text.starts_with('@') => {
                keys.push(text.clone());
            }
            Node::TextField { placeholder, .. } => keys.push(placeholder.clone()),
            _ => {}
        }
    }
}

/// Size of one item slot, in interface pixels.
pub const SLOT: u32 = 20;
