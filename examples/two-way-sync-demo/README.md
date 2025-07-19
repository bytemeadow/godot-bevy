# Two way transform syncing demo

This example demonstrates a Godot Node's x-position getting updated every frame
in GDScript, while the same game object's Transform is accessed via godot-bevy
where it's y-position is similarly updated every frame. While you wouldn't do
this in a real game, this contrived example demonstrates that you have the
flexibility to read and write the same game object's Transform either in Godot
or Bevy.

## What You'll See

A quad rotating in a circular motion on screen.

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
