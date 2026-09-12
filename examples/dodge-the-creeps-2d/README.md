# Dodge the Creeps

This is a simple game where your character must move
and avoid the enemies for as long as possible.

This is an in-progress version of the game featured in the
["Your first 2D game"](https://docs.godotengine.org/en/latest/getting_started/first_2d_game/index.html)
tutorial in the documentation adapted for `godot-bevy`.

Language: Rust

Renderer: Mobile

## Screenshots

![GIF from the documentation](https://docs.godotengine.org/en/latest/_images/dodge_preview.gif)

## Running this example

From the repository root, with Godot on your `PATH`:

```bash
cargo run --manifest-path examples/dodge-the-creeps-2d/rust/Cargo.toml
```

The launcher builds the library, generates its GDExtension descriptor, and opens Godot. Click Start and use the arrow keys to avoid enemies.

The menu's Show score checkbox sends `show_score_changed` from GDScript through `BevyAppSingleton.send_event`. A Bevy observer handles the typed event and changes the score label's visibility. Start uses `GodotSignals` to request the countdown.

The Rust modules also show game states, input handling, and a score resource. The menu integration test toggles the real checkbox and checks the observer's effect on the label:

```bash
cargo run --manifest-path examples/dodge-the-creeps-2d/rust/Cargo.toml --features itest
```

The [capture harness](../harness/README.md) runs `title-and-first-creeps` for 180 frames with seed 5 and checkpoints at 0, 60, and 180. It waits for the menu and its Start connection, then emits the real button's `pressed` signal at frame 1 through `GodotSignals<StartGameRequested>`. Capture hides the message label when the menu closes and holds creep bodies at their spawn positions; normal play keeps the countdown message and moving creeps. The first creep must appear at `[416.2791, 0]`, sampled from the authored perimeter using the seeded RNG. `title-unseeded` deliberately skips the manifest seed and must fail that position check. PNGs remain evidence; node facts and region statistics decide the result. The harness README has the preparation and launch recipe.

## Copying

`godot/audio/House In a Forest Loop.ogg` Copyright &copy; 2012 [HorrorPen](https://opengameart.org/users/horrorpen), [CC-BY 3.0: Attribution](http://creativecommons.org/licenses/by/3.0/). Source: https://opengameart.org/content/loop-house-in-a-forest

Images are from "Abstract Platformer". Created in 2016 by kenney.nl, [CC0 1.0 Universal](http://creativecommons.org/publicdomain/zero/1.0/). Source: https://www.kenney.nl/assets/abstract-platformer

Font is "Xolonium". Copyright &copy; 2011-2016 Severin Meyer <sev.ch@web.de>, with Reserved Font Name Xolonium, SIL open font license version 1.1. Details are in `godot/fonts/LICENSE.txt`.
