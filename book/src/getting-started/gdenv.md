# Run your Godot project with Cargo

The following steps will help you set up `cargo run` to run your Godot project. 
We will use the [`gdenv`](https://github.com/bytemeadow/gdenv) utility which has two parts,
a standalone command line tool and a crate library.

We will be working with the following files.
```
- project-root/
  - Cargo.toml
  - gdenv.toml
  - run_godot.rs
```

Update `Cargo.toml` with the following contents:
```toml
# Add this new section:
[[bin]]
name = "project_name_here_bin"
path = "run_godot.rs"

# Update your dependencies section with:
[dependencies]
gdenv-lib = { git = "https://github.com/bytemeadow/gdenv.git", tag = "v1.0.0" }
# Add godot-bevy-test if you want to also set up integration tests
godot-bevy-test = { version = "0.11", optional = true }

# Update or add your features section with:
[features]
# Add godot-bevy-test if you want to also set up integration tests
itest = ["dep:godot-bevy-test"]
```

Create a new `run_godot.rs` file. Put it at the same folder level as `Cargo.toml`.
Populate it with the following contents:
```rust
#[cfg(not(feature = "itest"))] // Keep this conditional compilation statement if you want to set up integration tests
fn main() {
    gdenv_lib::api::godot_runner::GodotRunner::init()
        .and_then(|r| r.build())
        .and_then(|r| r.execute())
        .unwrap_or_else(gdenv_lib::api::errors::print_error_stack);
}

// Keep the following code block if you want to set up integration tests
// Run with `cargo run --features itest` to run integration tests
#[cfg(feature = "itest")]
fn main() {
    gdenv_lib::api::godot_runner::GodotRunner::init()
        .and_then(|r| {
            r.godot_cli_arguments(Some(vec![
                "--headless".to_string(),
                "--scene".to_string(),
                "res://addons/godot-bevy/test/TestRunner.tscn".to_string(),
                "--quit-after".to_string(),
                "10000".to_string(),
            ]))
            .build()
        })
        .and_then(|r| r.execute())
        .unwrap_or_else(gdenv_lib::api::errors::print_error_stack);

    std::process::exit(godot_bevy_test::exit_code::read_and_cleanup_exit_code().unwrap_or(1));
}
```

Add a `gdenv.toml` file at the root of your project.
Populate it with the following contents:
```toml
[godot]
version = "4.6.2"
project_dir = "<relative path to your godot project>"

[gdextension.1.Rust]
cargo_crate_path = "<relative path to your rust project>"
```
Adjust the Godot `version`, `project_dir`, and `cargo_crate_path` to match your project structure.

The `[gdextension.1.Rust]` section will automatically generate a `rust.gdextension` file in your Godot project.
If you are using `git` as your version control system, you can delete `rust.gdextension` and `rust.gdextension.uid`
from your Godot project and commit the deletions to your version control system.
Then you can add the following lines to the `.gitignore` file in your Godot project:
```
rust.gdextension
rust.gdextension.uid
```
This helps make your project more portable across developer machines
(specifically if they have changed the cargo build output directory).
