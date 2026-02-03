// The Godot versions used here are sourced from Godot-Rust's handling of gdextension API differences:
// https://github.com/godot-rust/gdext/blob/3f1d543580c1817f1b7fab57a400e82b50085581/godot-bindings/src/import.rs

#![allow(unused_imports)]
#![allow(unexpected_cfgs)]

mod node_markers4_2;
mod node_markers4_2_1;
mod node_markers4_2_2;
mod node_markers4_3;
mod node_markers4_4;
mod node_markers4_5;

#[cfg(feature = "api-4-2")]
pub use node_markers4_2::*;
#[cfg(feature = "api-4-2-1")]
pub use node_markers4_2_1::*;
#[cfg(feature = "api-4-2-2")]
pub use node_markers4_2_2::*;
#[cfg(feature = "api-4-3")]
pub use node_markers4_3::*;
#[cfg(feature = "api-4-4")]
pub use node_markers4_4::*;
#[cfg(feature = "api-4-5")]
pub use node_markers4_5::*;

#[cfg(not(any(
    feature = "api-4-2",
    feature = "api-4-2-1",
    feature = "api-4-2-2",
    feature = "api-4-3",
    feature = "api-4-4",
    feature = "api-4-5",
    feature = "api-custom",
    feature = "api-custom-json",
)))]
pub use node_markers4_5::*;
