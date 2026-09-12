# Transform sync performance diagnostic

This tool compares rendered particle rain in GDScript and godot-bevy. It exercises entity creation and transform updates under load. It is separate from the learning examples.

## Running the diagnostic

From the repository root, with Godot on your `PATH`:

```bash
cargo run --manifest-path examples/perf-test/rust/Cargo.toml
```

The launcher builds the library, generates its GDExtension descriptor, and opens Godot.

## Controls and readings

Select Godot (GDScript) or godot-bevy (Rust + ECS). Set the particle count, then click Start Benchmark. Stop ends the run; Reset Metrics clears the readings.

The display reports current, average, minimum, and maximum FPS, plus the active particle count. Results depend on the build profile, renderer, hardware, and particle count. Treat these readings as diagnostics, with no expected FPS target.

## Implementation

`godot/scripts/godot_particles.gd` updates Node2D positions directly. `rust/src/particle_rain.rs` updates ECS components and sends transforms through godot-bevy's sync systems. Both simulate falling particles with gravity, horizontal drift, and wraparound.

For library regression measurements, use the interleaved comparison described in [itest/BENCHMARKING.md](../../itest/BENCHMARKING.md).
