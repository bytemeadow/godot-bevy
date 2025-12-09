//! # godot-bevy-inspector
//!
//! A runtime inspector for godot-bevy projects, similar to bevy-inspector-egui but using
//! native Godot UI controls instead of egui.
//!
//! ## Features
//!
//! - **Entity Hierarchy**: Tree view of all entities with their names and components
//! - **Component Inspector**: Edit component values at runtime using Bevy's reflection
//! - **Resource Browser**: View and edit Bevy resources
//! - **Native Godot UI**: Uses Godot's Control nodes for seamless integration
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use bevy_app::App;
//! use godot_bevy_inspector::InspectorPlugin;
//!
//! #[bevy_app]
//! fn build_app(app: &mut App) {
//!     app.add_plugins(InspectorPlugin::default());
//! }
//! ```
//!
//! ## Architecture
//!
//! The inspector is built on several layers:
//!
//! 1. **Options Layer** (`options`): Field-level configuration for how values are displayed
//! 2. **Widget Layer** (`widgets`): Godot Control-based widgets for editing values
//! 3. **Reflection Layer** (`reflect`): Traverses Bevy's reflection system to generate UI
//! 4. **Panel Layer** (`panels`): High-level UI panels (hierarchy, inspector, etc.)

pub mod options;
pub mod panels;
mod plugin;
pub mod reflect;
pub mod widgets;

pub use options::{InspectorOptions, NumberOptions, Target};
pub use plugin::InspectorPlugin;
pub use reflect::InspectorUi;

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::options::{InspectorOptions, NumberOptions, QuatOptions, Target};
    pub use crate::plugin::InspectorPlugin;
    pub use crate::reflect::InspectorUi;
}
