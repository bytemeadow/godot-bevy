# Introduction

Welcome to **godot-bevy**, a Rust library that brings [Bevy's](https://bevyengine.org/) powerful Entity Component System (ECS) to the versatile [Godot Game Engine](https://godotengine.org/). 

## What is godot-bevy?

godot-bevy enables you to write high-performance game logic using Bevy's ergonomic ECS within your Godot projects. This is not a Godot plugin for Bevy users, but rather a library for **Godot developers who want to leverage Rust and ECS** for their game logic while keeping Godot's excellent editor and engine features.

## Why godot-bevy?

### The Best of Both Worlds

- **Godot's Strengths**: Visual scene editor, node system, asset pipeline, cross-platform deployment
- **Bevy's Strengths**: High-performance ECS, Rust's safety and speed, data-oriented architecture
- **godot-bevy**: Seamless integration between the two, letting you use each tool where it shines

### Key Benefits

1. **Performance**: Bevy's ECS provides cache-friendly data layouts and parallel system execution
2. **Safety**: Rust's type system catches bugs at compile time
3. **Modularity**: ECS encourages clean, decoupled code architecture
4. **Flexibility**: Mix and match Godot nodes with ECS components as needed

## Core Features

- **Deep ECS Integration**: True Bevy systems controlling Godot nodes
- **Transform Synchronization**: Automatic syncing between Bevy and Godot transforms
- **Signal Handling**: React to Godot signals in your ECS systems
- **Collision Events**: Handle physics collisions through the ECS
- **Resource Management**: Load Godot assets through Bevy's asset system
- **Smart Scheduling**: Separate physics and rendering update rates

## Who Should Use godot-bevy?

This library is ideal for:

- Godot developers wanting to use Rust for game logic
- Teams looking for better code organization through ECS
- Projects requiring high-performance game systems
- Developers familiar with data-oriented design patterns

## Getting Help

- **Discord**: Join our [community Discord](https://discord.gg/gqkeBsH93H)
- **Documentation**: Check the [API docs](https://docs.rs/godot-bevy/latest/godot_bevy/)
- **Examples**: Browse the [example projects](https://github.com/dcvz/godot-bevy/tree/main/examples)
- **Issues**: Report bugs on [GitHub](https://github.com/dcvz/godot-bevy/issues)

## Ready to Get Started?

Head to the [Installation](./getting-started/installation.md) chapter to begin your godot-bevy journey!