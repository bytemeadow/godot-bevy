# Godot-Bevy Documentation

This directory contains detailed documentation for godot-bevy concepts and features.

## Core Concepts

### [Timing and Schedules](TIMING_AND_SCHEDULES.md)
Understanding how godot-bevy integrates with Godot's frame timing:
- Frame execution model (visual vs physics frames)
- Schedule relationships and ordering guarantees
- Data synchronization and transform updates
- Best practices for different use cases
- Delta time considerations

### [Input Systems](INPUT_SYSTEMS.md)
Choosing and using input systems in godot-bevy:
- When to use Bevy's built-in input vs Godot's bridged input
- Cross-platform considerations
- Input event types and processing
- Thread-safe design and integration

## Examples

The project includes several complete examples demonstrating different aspects:

- **[Dodge the Creeps 2D](../examples/dodge-the-creeps-2d/)**: Complete 2D game with ECS-driven gameplay
- **[Timing Test](../examples/timing-test/)**: Demonstrates schedule execution and timing behavior
- **[Input Event Demo](../examples/input-event-demo/)**: Shows cross-platform input handling

## API Reference

For detailed API documentation, see [docs.rs/godot-bevy](https://docs.rs/godot-bevy).

## Getting Help

- **Issues**: Report bugs or request features on [GitHub Issues](https://github.com/rand0m-cloud/godot-bevy/issues)
- **Examples**: Check the `examples/` directory for complete working projects 