# Godot to Bevy Events/Messages

Sometimes we need to communicate from Godot to Bevy.
For example, we might want a button press to trigger the game to start.

This approach leverages the `godot_bevy::app::with_bevy_app_singleton()` helper function.
With it, we can access the Bevy App object from any Godot node attached to the scene tree.
In our example, we call it from the button-pressed signal handler of our `StartGameButton` node.
`with_bevy_app_singleton` is only accessible from Rust code, so we need to define a
custom Godot node.

First, let's define our custom Godot node. Any type that extends Node will work:
```rust,ignore
#[derive(GodotClass)]
#[class(base=Button, init)]
pub struct StartGameButton {
    base: Base<Button>,
}
```

Next, we will implement the `IButton` trait, and add a signal handler for the `pressed` signal.
```rust,ignore
#[godot_api]
impl IButton for StartGameButton {
    fn ready(&mut self) {
        self.signals()
            .pressed()
            .connect_self(Self::on_button_pressed);
    }
}

#[godot_api]
impl StartGameButton {
    pub fn on_button_pressed(&mut self) {
        // ...signal handler code...
    }
}
```

We are now ready to define our Bevy event. Messages will work as well.
You can read about the differences between events and messages on
[taintedcoders's helpful blog about them](https://taintedcoders.com/bevy/events).
```rust,ignore
#[derive(Event)]
pub struct StartGameEvent;
```

Now we can implement our signal handler.
```rust,ignore
    pub fn on_button_pressed(&mut self) {
        with_bevy_app_singleton(&self.base(), |app| {
            // Sends the StartGameEvent to the Bevy App where it can be handled by a system.
            app.world_mut().trigger(StartGameEvent);
        });
    }
```

Here's an example of what a Bevy event handler might look like.
```rust,ignore
pub fn trigger_level_generation_step(
    event: On<StartGameEvent>,
    mut parameters_query: Query<(&mut LevelGeneratorParameters, &GodotNodeHandle), With<GridMapMarker>>,
    mut godot: GodotAccess,
) {
    debug!("Starting game! Generating level...");
}
```



## Full Example

```rust,ignore
use bevy::prelude::Event;
use godot::classes::{Button, IButton};
use godot::obj::{Base, WithBaseField, WithUserSignals};
use godot::prelude::{GodotClass, godot_api};
use godot_bevy::app::with_bevy_app_singleton;

#[derive(Event)]
pub struct StartGameEvent;

#[derive(GodotClass)]
#[class(base=Button, init)]
pub struct StartGameButton {
    base: Base<Button>,
}

#[godot_api]
impl IButton for StartGameButton {
    fn ready(&mut self) {
        self.signals()
            .pressed()
            .connect_self(Self::on_button_pressed);
    }
}

#[godot_api]
impl StartGameButton {
    pub fn on_button_pressed(&mut self) {
        with_bevy_app_singleton(&self.base(), |app| {
            app.world_mut().trigger(StartGameEvent);
        });
    }
}
```
