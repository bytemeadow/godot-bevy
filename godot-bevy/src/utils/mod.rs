//! Shared utilities used across multiple domains.

pub mod debug;
pub mod math;

pub use math::{clamp_to_range, is_reasonable_float, lerp, move_toward, normalize_angle};

pub use debug::{print_scene_tree, print_tree_structure};
