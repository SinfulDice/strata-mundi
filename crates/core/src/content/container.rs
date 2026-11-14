use serde::Deserialize;

/// Slots of a container block, in rows of nine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename = "Container", deny_unknown_fields)]
pub struct ContainerDef {
    pub rows: u8,
    /// Two of them side by side open as one (double chest).
    #[serde(default)]
    pub joins: bool,
}

impl ContainerDef {
    pub const COLUMNS: usize = 9;
    pub const MAX_ROWS: u8 = 6;

    pub fn slots(&self) -> usize {
        usize::from(self.rows) * Self::COLUMNS
    }
}
