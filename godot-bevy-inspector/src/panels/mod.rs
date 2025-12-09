//! High-level inspector panels.
//!
//! This module provides ready-to-use inspector panels:
//! - Entity hierarchy tree
//! - Component inspector
//! - Resource browser
//! - World inspector (combines hierarchy + inspector)

mod hierarchy;
mod inspector;
mod world;

pub use hierarchy::{EntityDataCollector, HierarchyPanel};
pub use inspector::{ComponentDataSerializer, InspectorPanel};
pub use world::{WorldInspectorPanel, WorldInspectorWindow};
