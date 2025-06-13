//! Debug utilities for scene tree inspection and diagnostics.

use godot::{classes::Node, global::godot_print, obj::Gd};

use crate::prelude::SceneTreeRef;

/// Prints the tree structure starting from the given node with proper indentation.
pub fn print_tree_structure(node: Gd<Node>, indent_level: usize) {
    let indent = "  ".repeat(indent_level);
    godot_print!("{}Node: {}", indent, node.get_name());

    for child in node.get_children().iter_shared() {
        print_tree_structure(child, indent_level + 1);
    }
}

/// Prints the entire scene tree structure starting from the root node.
pub fn print_scene_tree(scene_tree: &mut SceneTreeRef) {
    let root = scene_tree.get().get_root().unwrap();
    godot_print!("Scene Tree Structure:");
    print_tree_structure(root.upcast(), 0);
}
