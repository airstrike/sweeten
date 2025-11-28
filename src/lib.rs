mod helpers;
pub mod widget;

pub use helpers::*;

// Re-exports to mirror iced_widget structure (allows minimal diff for widgets)
use iced_core as core;
pub use iced_core::Theme;
pub use iced_widget::Renderer;
pub use iced_widget::{button, scrollable, text_editor};

// Re-export widget modules at crate level (mirrors iced_widget's structure)
pub use widget::overlay;
pub use widget::text_input;
