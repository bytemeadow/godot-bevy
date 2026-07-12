# Debug Build Performance

Debug builds run much slower than release builds. For per-frame work that crosses the Rust/Godot boundary many times -- transform sync over hundreds of nodes, for example -- a debug build can be several times slower than release. This is normal, and most of the gap comes from two independent causes you can tune separately.

## Optimize your dependencies

By default a debug build compiles everything at `opt-level = 0`, including Bevy, gdext, and the rest of your dependency tree. Those crates do the heavy per-frame math, so leaving them unoptimized is what makes debug builds feel sluggish.

Optimize your dependencies while leaving your own crate unoptimized:

```toml
[profile.dev.package."*"]
opt-level = 3
```

Your own code still compiles quickly and stays fully debuggable, while the hot paths in your dependencies run at release speed. In transform-heavy scenes this is worth roughly a 3x improvement. It is standard Rust practice rather than anything specific to godot-bevy -- the [Bevy book recommends the same setting](https://bevy.org/learn/quick-start/getting-started/setup/). The godot-bevy project wizard scaffolds it into your `Cargo.toml` automatically.

## Tune gdext safeguards

gdext runs runtime validity checks on calls that cross the FFI boundary. These catch real bugs (use-after-free, type mismatches) but add per-call overhead. gdext exposes three levels:

- **Strict** -- default for dev builds, maximum checking.
- **Balanced** -- default for release builds, basic validity checks, fast.
- **Disengaged** -- most checks off, fastest, unsafe.

If FFI validation shows up in your profile, drop the dev level to balanced:

```toml
godot = { version = "0.x", features = ["safeguards-dev-balanced"] }
```

There is a matching `safeguards-release-disengaged` for shipping builds, but only reach for it after testing thoroughly on a higher level. See gdext's [safeguard levels](https://godot-rust.github.io/docs/gdext/master/godot/index.html#safeguard-levels) documentation for details.
