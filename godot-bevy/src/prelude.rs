pub use crate::GodotPlugin;
pub use crate::bridge::*;
pub use crate::plugins::{core::*, packed_scene::*};

pub use godot_bevy_macros::bevy_app;

pub use crate::node_tree_view::NodeTreeView;
pub use godot_bevy_macros::NodeTreeView;

pub mod godot_prelude {
    pub use godot::prelude::*;
}
pub use godot;
