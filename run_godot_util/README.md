This crate contains utilities for running Godot from within a custom binary target `run_godot.rs` script.

Example usage:
`run_godot.rs`
```rust
use std::path::PathBuf;
use run_godot_util::run_godot;
use std::process::exit;

fn main() {
  let godot_project_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../godot");
  if let Err(e) = run_godot(godot_project_path.as_path(), env!("CARGO_PKG_NAME"), None) {
    eprintln!("Error running Godot: {}", e);
    exit(1);
  }
}
```

`Cargo.toml`
```toml
[package]
name = "my_godot_project"
# ...
[[bin]]
path = "../run_godot.rs"
name = "run_godot"
# ...
```

Run with `cargo run --project my_godot_project`.
