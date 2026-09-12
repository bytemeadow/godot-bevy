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

Diagnostic capture: `capture/rain-2000.json` checks rendered load with 2,000 particles per implementation, side by side at 480×640. Both use seed 11, start above a black viewport, and wait for all particles and textures before frame 0. Three regions check the clear colour at frame 0 and at least 30% non-blank pixels at frames 60 and 120. FPS is outside the verdict. The `rain-2000-unspawned` negative scenario skips spawning and must fail the six later region checks. Capture hides the controls and uses white particles; the interactive diagnostic keeps its existing presentation. After the [capture harness preparation](../../itest/capture/README.md), run `devenv shell -- python3 itest/capture/capture.py --example perf-test --scenario rain-2000 --run perf-rain-1`. Build this example with `--features capture`; use fresh run IDs for repeats. A CLI spawn timeout exits 1 without starting measurement.

## Implementation

`godot/scripts/godot_particles.gd` updates Node2D positions directly. `rust/src/particle_rain.rs` updates ECS components and sends transforms through godot-bevy's sync systems. Both simulate falling particles with gravity, horizontal drift, and wraparound.

For library regression measurements, use the interleaved comparison described in [itest/BENCHMARKING.md](../../itest/BENCHMARKING.md).
