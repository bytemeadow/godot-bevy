# Profiling

Godot-Bevy, together with Bevy native, supports several methods of profiling. In this article,
we'll discuss using Tracy. We recommend you read [Bevy's profiling doc](https://github.com/bevyengine/bevy/blob/main/docs/profiling.md) first.

# Instructions

- In your `Cargo.toml`, under `dependencies` add necessary tracy dependencies, e.g.:

```toml
[dependencies]
tracing = "0.1"
tracing-tracy = { version = "0.11.4", default-features = false, features = [
  "enable",
  "manual-lifetime",
  "ondemand",
  "broadcast",       # announce presence
], optional = true }
```

- In your `Cargo.toml`, under `features` add a `trace_tracy` (feel free to rename it):

```toml
[features]
trace_tracy = ["dep:tracing-tracy", "godot-bevy/trace_tracy"]
```

- Install Tracy, see
  https://github.com/bevyengine/bevy/blob/main/docs/profiling.md for details on
  picking the correct version to install. As of July 2025, you need Tracy
  Profiler `0.12.2`, which you can obtain from [The official
  site](https://github.com/wolfpld/tracy). Alternatively, you can use the
  zig-built version, which makes it much easier to build c binaries across
  platforms, see https://github.com/allyourcodebase/tracy
- Once built, run the Tracy Profiler (`tracy-profiler`), and hit the `Connect`
  button so it's listening/ready to receive real time data from your game
- Build your game. You can use either dev or release, both work, though we
  recommend release since you'll still get symbol resolution and your profiling
  numbers will reflect what you're actually shipping in addition to being
  _much_ faster than a dev build.
- Run your game, you should see real time data streaming into the Tracy
  profiler GUI.
- For a complete example of this in action, see our [Bevy Boids
  example](https://github.com/bytemeadow/godot-bevy/tree/main/examples/boids-perf-test)

## Notes

If you see the following warning:

```rust
warning: unexpected `cfg` condition value: `trace_tracy`
```

after ugprading to Godot Bevy `0.9`, add the following at the top of the problematic file (wherever you use the `bevy_app` macro):

```rust
#![allow(unexpected_cfgs)] // silence potential `tracy_trace` feature config warning brought in by `bevy_app` macro
```
