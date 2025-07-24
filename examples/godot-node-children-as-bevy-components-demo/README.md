# Godot Node Children as Bevy Components Demo

Demonstrates how to use the `ComponentAsGodotNodeChildPlugin` to add components
to your nodes by adding them as children.

In this example, we annotate the `Oribter` and `InitialPosition` components
with the `ComponentAsGodotNode` derive macro which registers nodes `OrbiterBevyComponent`
and `InitialPositionBevyComponent` with Godot. Adding those nodes as children
to the Icon will automatically register the corresponding components with
the Icon parent they are attached to.

![Final Product](scene_tree.png)

## Running This Example

1. **Build**: `cargo build`
2. **Run**: You can either:
    1. Open the Godot project and run the scene
    1. Run: `cargo run`. NOTE: This requires the Godot binary, which we attempt
       to locate either through your environment's path or by searching common
       locations. If this doesn't work, update your path to include Godot. If
       this fails for other reasons, it may be because your version of Godot
       is different than the one the example was built with, in that case,
       try opening the Godot project first.
