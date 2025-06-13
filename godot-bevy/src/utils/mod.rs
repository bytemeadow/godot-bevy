//! Shared utility modules used across multiple domains.
//! 
//! This module contains only truly cross-cutting utilities that are used
//! in multiple parts of the codebase. Domain-specific utilities are located
//! within their respective domains (e.g., audio validation in audio module).

pub mod debug;
pub mod math;

// Re-export commonly used shared functions
pub use math::{clamp_to_range, normalize_angle, lerp, move_toward, is_reasonable_float};

// Re-export debug functions
pub use debug::{print_tree_structure, print_scene_tree};