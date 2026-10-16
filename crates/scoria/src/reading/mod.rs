//! Reading view: the rendered form of a note.

pub mod model;
mod view;

pub use model::{build_with, BuildOptions, EmbedSource, LinkTarget};
pub use view::{ReadingEvent, ReadingView};
