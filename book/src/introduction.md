# Introduction

Welcome to the godot-bevy book.

The aim of this library is to combine the strength of using Bevy ECS as your game logic with the amazing capabilities of Godot as an editor.

## The library in a nutshell

The quick summary of using this library would be:
1. You create Bevy [Entities](https://bevy-cheatbook.github.io/programming/ec.html) or [Bundles](https://bevy-cheatbook.github.io/programming/bundle.html) and use the [GodotNode]() derive macro to have it generate a Godot class (node) you can place in your editor.
2. You create [Scenes](https://docs.godotengine.org/en/4.5/getting_started/step_by_step/nodes_and_scenes.html) in Godot and use the generated Godot Nodes from our macros to composition your scenes
3. You create Bevy systems to [Instantiate Scenes]() and build game logic systems that can [Query]() the components you derived the Godot Nodes from or from the [Built-in Marker Components]() to build your game!
