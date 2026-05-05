//! Layout engine for webgpui.
//!
//! Supports:
//! - **Absolute** – positioned at explicit `(x, y)` relative to the parent
//!   content area.
//! - **Stack** – children laid out along the `direction` axis.
//!   - `Direction::Column` (default): top-to-bottom.
//!   - `Direction::Row`: left-to-right.
//! - **`flex_grow`** – proportionally distributes remaining main-axis space
//!   after fixed-size children are placed.
//! - **Text nodes** – auto-sized via the pluggable [`TextMeasure`] trait.
//!
//! Margin and padding are fully respected.  `width` / `height` may be an
//! explicit pixel value or `None` (fill / shrink-wrap).

mod direction;
mod engine;
mod measure;
mod node;
mod style;

pub use direction::Direction;
pub use engine::LayoutEngine;
pub use measure::{DefaultTextMeasure, TextMeasure};
pub use node::{LayoutNode, LayoutResult};
pub use style::{LayoutStyle, PositionType};
