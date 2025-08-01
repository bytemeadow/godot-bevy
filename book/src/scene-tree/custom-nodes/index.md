# Custom Nodes

This section explains how to work with custom Godot nodes in godot-bevy and
the important distinction between automatic markers for built-in Godot types versus custom nodes.

## Summary

- Built-in Godot types get automatic markers (e.g., `Sprite2DMarker`)
- Custom nodes do NOT get automatic markers for their type, but DO inherit base class markers
- Use `BevyBundle` to define components for custom nodes
- Prefer semantic components over generic markers
- Combine base class markers with custom components for powerful queries

This design gives you full control over your ECS architecture while maintaining performance and clarity.
