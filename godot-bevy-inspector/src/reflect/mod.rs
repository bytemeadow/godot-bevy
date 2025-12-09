//! Reflection-based UI generation.
//!
//! This module provides the core logic for traversing Bevy's reflection system
//! and generating appropriate Godot UI widgets for each type.

mod inspector_ui;
mod type_impls;

pub use inspector_ui::{Context, InspectorUi};
