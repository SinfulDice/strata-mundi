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
    /// `text` is a translation key, or `@name` for text supplied by the screen model.
    Label {
        id: String,
        anchor: Anchor,
        at: (i32, i32),
        text: String,
        #[serde(default)]
        boxed: bool,
        #[serde(default)]
        dim: bool,
    },
    Button {
        id: String,
        anchor: Anchor,
        at: (i32, i32),
        size: (u32, u32),
        text: String,
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

impl ScreenDef {
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
