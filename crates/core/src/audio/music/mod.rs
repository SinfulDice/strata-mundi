//! Music made by code: themes from the packs, composed into calm piano and pad pieces
//! for a seed, rendered block by block and cached as WAV files.

mod cache;
mod compose;
mod fingerprint;
mod render;
#[cfg(test)]
mod tests;
mod theme;
mod voices;

pub use cache::{ALGORITHM, Cached, Piece, WavStream, cached_piece, prune};
pub use compose::{Instrument, Mood, Note, Score, compose, hertz};
pub use render::{PieceRenderer, render_piece};
pub(super) use theme::load_themes;
pub use theme::{MAX_PIECE_SECONDS, MAX_PIECES, Motif, Pad, Piano, Room, Scale, Theme, ThemeDef};
