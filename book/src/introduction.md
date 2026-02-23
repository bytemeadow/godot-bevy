# Introduction

Welcome to the godot-bevy book.

The aim of this library is to combine the strengths of using Bevy ECS for your game logic with the amazing capabilities of Godot as an editor.

## The library in a nutshell

A quick summary of using this library would be:
1. You create Bevy [Entities](https://bevy-cheatbook.github.io/programming/ec.html) or [Bundles](https://bevy-cheatbook.github.io/programming/bundle.html), and use the [GodotNode](./core-workflow/godot-node-entities-and-bundles.md) derive macro to generate a Godot class (node) you can place in your editor.
2. You create [Scenes](https://docs.godotengine.org/en/4.5/getting_started/step_by_step/nodes_and_scenes.html) in Godot and use the generated Godot nodes from our macros to compose your scenes.
3. You create Bevy systems to [Instantiate Scenes](./core-workflow/scenes-and-instantiation.md) and build your game logic with systems that [Query](./core-workflow/queries-and-marker-components.md) the components your Godot nodes were derived from, or the [Built-in Marker Components](./core-workflow/queries-and-marker-components.md).

## How the library is organized

The library itself is organized into plugins, similar to how Bevy is organized. [GodotCorePlugins](./feature-areas/plugins-and-schedules.md) is always included and has must-have base functionality -- like the Bevy plugins we require, new schedules to match Godot's separation between [Physics and Visual Updates](./feature-areas/plugins-and-schedules.md), and scene tree observation systems. Those systems are the core of how this library works.

The library also ships with a [Godot Editor Plugin](https://docs.godotengine.org/en/stable/tutorials/plugins/editor/installing_plugins.html), which offers a couple of different benefits: helping set up a godot-bevy project, providing script utilities that make the library faster, and an in-editor [Entity Visualizer](./tooling/editor-plugin-and-entity-visualizer.md) like Godot's scene visualizer that shows the entities in a running game.
