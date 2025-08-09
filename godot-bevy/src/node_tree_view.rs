/// Trait for objects that can be created from a node reference.
///
/// This is implemented by the `#[derive(NodeTreeView)]` macro.
pub trait NodeTreeView {
    /// Create a new instance from a node reference.
    fn from_node<T: godot::obj::Inherits<godot::classes::Node>>(node: godot::obj::Gd<T>) -> Self;
}

/// Find a node by matching a pattern with wildcards.
///
/// Supports patterns like:
/// - `/root/*/HUD/CurrentLevel` - matches any single node name where * appears
/// - `/root/Level*/HUD/CurrentLevel` - matches node names starting with "Level"
/// - `*/HUD/CurrentLevel` - matches relative to the base node
pub fn find_node_by_pattern(
    base_node: &godot::obj::Gd<godot::classes::Node>,
    pattern: &str,
) -> Option<godot::obj::Gd<godot::classes::Node>> {
    // Handle absolute vs relative paths
    let (search_root, pattern_parts) = if let Some(stripped) = pattern.strip_prefix('/') {
        // Absolute path - start from scene tree root
        let scene_tree = base_node.get_tree()?;
        let root = scene_tree.get_root()?;
        let root_as_node = root.upcast::<godot::classes::Node>();
        let mut parts: Vec<&str> = stripped.split('/').filter(|s| !s.is_empty()).collect();

        // If the first part is "root", skip it since we're already starting from the root
        if !parts.is_empty() && parts[0] == "root" {
            parts.remove(0);
        }

        (root_as_node, parts)
    } else {
        // Relative path - start from base node
        let parts: Vec<&str> = pattern.split('/').filter(|s| !s.is_empty()).collect();
        (base_node.clone(), parts)
    };

    find_node_recursive(&search_root, &pattern_parts, 0)
}

fn find_node_recursive(
    current_node: &godot::obj::Gd<godot::classes::Node>,
    pattern_parts: &[&str],
    depth: usize,
) -> Option<godot::obj::Gd<godot::classes::Node>> {
    // If we've matched all pattern parts, we found our target
    if depth >= pattern_parts.len() {
        return Some(current_node.clone());
    }

    let pattern_part = pattern_parts[depth];

    // If this pattern part is a wildcard
    if pattern_part == "*" {
        // Try all children
        for i in 0..current_node.get_child_count() {
            if let Some(child) = current_node.get_child(i)
                && let Some(result) = find_node_recursive(&child, pattern_parts, depth + 1)
            {
                return Some(result);
            }
        }
    } else if pattern_part.contains('*') {
        // Handle prefix/suffix wildcards like "Level*" or "*Button"
        for i in 0..current_node.get_child_count() {
            if let Some(child) = current_node.get_child(i) {
                let child_name = child.get_name().to_string();
                if matches_wildcard_pattern(&child_name, pattern_part)
                    && let Some(result) = find_node_recursive(&child, pattern_parts, depth + 1)
                {
                    return Some(result);
                }
            }
        }
    } else {
        // Exact name match
        if current_node.has_node(pattern_part) {
            let child = current_node.get_node_as::<godot::classes::Node>(pattern_part);
            if let Some(result) = find_node_recursive(&child, pattern_parts, depth + 1) {
                return Some(result);
            }
        }
    }

    None
}

fn matches_wildcard_pattern(text: &str, pattern: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    // Handle simple prefix/suffix patterns
    if pattern.starts_with('*') && pattern.len() > 1 {
        let suffix = &pattern[1..];
        return text.ends_with(suffix);
    }

    if pattern.ends_with('*') && pattern.len() > 1 {
        let prefix = &pattern[..pattern.len() - 1];
        return text.starts_with(prefix);
    }

    // Handle patterns with * in the middle (basic implementation)
    if let Some(star_pos) = pattern.find('*') {
        let prefix = &pattern[..star_pos];
        let suffix = &pattern[star_pos + 1..];
        return text.starts_with(prefix)
            && text.ends_with(suffix)
            && text.len() >= prefix.len() + suffix.len();
    }

    // No wildcard, exact match
    text == pattern
}
