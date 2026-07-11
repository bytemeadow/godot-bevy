# Adding to an Existing gdext Project

The `#[bevy_app]` macro is the quickest way to start, but it generates the entire
`ExtensionLibrary` implementation for you. If your crate already has one -- because
you began as a plain [gdext](https://godot-rust.github.io/) project and want to adopt
godot-bevy incrementally -- a second `ExtensionLibrary` produces a duplicate
entry-symbol link error.

For that case, skip the macro and register your Bevy app from your own
`ExtensionLibrary`.

## Register from your own ExtensionLibrary

Call `godot_bevy::app::init` during the `Core` init stage, and
`godot_bevy::app::deinit` during `Core` deinit:

```rust
use bevy::prelude::*;
use godot::init::{ExtensionLibrary, gdextension};
use godot::prelude::InitStage;
use godot_bevy::prelude::*;

struct MyExtension;

#[gdextension]
unsafe impl ExtensionLibrary for MyExtension {
    fn on_stage_init(stage: InitStage) {
        if stage == InitStage::Core {
            godot_bevy::app::init(build_app);
            // ... your own gdext initialization ...
        }
    }

    fn on_stage_deinit(stage: InitStage) {
        if stage == InitStage::Core {
            godot_bevy::app::deinit();
        }
    }
}

fn build_app(app: &mut App) {
    app.add_plugins(GodotDefaultPlugins);
    // your systems, plugins, and resources ...
}
```

`build_app` is exactly what you would have written inside `#[bevy_app]` -- the macro
is just sugar over `init_with_config`. Everything else is unchanged: you still add the
`BevyAppSingleton` autoload (see [Installation](./installation.md)) and add the plugins
you need.

## Configuration

`init` uses the default configuration. To change scene-tree behavior, use
`init_with_config`:

```rust
godot_bevy::app::init_with_config(
    godot_bevy::app::BevyAppConfig {
        scene_tree_auto_despawn_children: false,
    },
    build_app,
);
```

The same options are available on the macro attribute -- for example
`#[bevy_app(scene_tree_auto_despawn_children = false)]` -- when you do use it.
