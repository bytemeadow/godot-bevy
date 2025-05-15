/// Trait for objects that can be created from a node reference.
/// 
/// This is implemented by the `#[derive(NodeTreeView)]` macro.
pub trait NodeTreeView {
    /// Create a new instance from a node reference.
    fn from_node<T: godot::obj::Inherits<godot::classes::Node>>(node: godot::obj::Gd<T>) -> Self;
}
