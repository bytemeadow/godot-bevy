//! Shared utility modules used across multiple domains.
//!
//! This module contains only truly cross-cutting utilities that are used
//! in multiple parts of the codebase. Domain-specific utilities are located
//! within their respective domains (e.g., audio validation in audio module).

pub mod debug;
pub mod math;
pub mod profiling;

// Re-export commonly used shared functions
pub use math::{clamp_to_range, is_reasonable_float, lerp, move_toward, normalize_angle};

// Re-export debug functions
pub use debug::{print_scene_tree, print_tree_structure};

// Re-export tracy client singleton
#[cfg(feature = "trace_tracy")]
pub use profiling::TRACY_CLIENT;
