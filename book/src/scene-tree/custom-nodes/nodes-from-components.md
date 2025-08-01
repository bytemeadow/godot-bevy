# Nodes from Components

Often, we want to make a Godot node from a component.
We can easily generate a node by using the `GodotNode` derive macro.

```rust
#[derive(Component, GodotNode)]
#[godot_node(base(Node2D), class_name(PlayerNode))]
pub struct Player {
    #[godot_export]
    pub active: bool,
    #[godot_export(
        export_type(Vector2),
        transform_with(transform_to_vec2),
        default(Vector2::new(5.0, 15.0))
    )]
    pub position: Vec2,
    // Won't be exposed to Godot
    pub internal_data: Vec<f32>,
}
```

This will generate a `PlayerNode` node that can be added to the scene tree.
The `Player` Bevy component will be automatically added to the node.

[See the GodotNode Rust docs for more information about parameters and syntax.](https://docs.rs/godot-bevy/latest/godot_bevy/prelude/derive.GodotNode.html)
