# Dodge the Creeps

This is a simple game where your character must move
and avoid the enemies for as long as possible.

This is an in-progress version of the game featured in the
["Your first 2D game"](https://docs.godotengine.org/en/latest/getting_started/first_2d_game/index.html)
tutorial in the documentation adapted for `godot-bevy`.

Language: Rust

Renderer: Compatibility

## Screenshots

![GIF from the documentation](https://docs.godotengine.org/en/latest/_images/dodge_preview.gif)

![Screenshot](screenshots/dodge.png)

## Running This Example

1. **Build**: `cargo build`
2. **Run**: You can either:
    1. Open the Godot project and run the scene
    1. Run: `cargo run`. NOTE: This requires the Godot binary, which we attempt
       to locate either through your environment's path or by searching common
       locations. If this doesn't work, update your path to include Godot. If
       this fails for other reasons, it may be because your version of Godot
       is different than the one the example was built with, in that case,
       try opening the Godot project first.
3. **Interact**: Try different input methods:
   - Use arrow keys (mapped to move_left, move_right, etc. actions)

This example is particularly useful for:
- Learn how to organize a godot-bevy project
- Understanding basic game states: how to define and transition between them
- Learning how to process input events in Bevy systems
- Learn how to use bevy resources to keep a simple game score

## Copying

`art/House In a Forest Loop.ogg` Copyright &copy; 2012 [HorrorPen](https://opengameart.org/users/horrorpen), [CC-BY 3.0: Attribution](http://creativecommons.org/licenses/by/3.0/). Source: https://opengameart.org/content/loop-house-in-a-forest

Images are from "Abstract Platformer". Created in 2016 by kenney.nl, [CC0 1.0 Universal](http://creativecommons.org/publicdomain/zero/1.0/). Source: https://www.kenney.nl/assets/abstract-platformer

Font is "Xolonium". Copyright &copy; 2011-2016 Severin Meyer <sev.ch@web.de>, with Reserved Font Name Xolonium, SIL open font license version 1.1. Details are in `fonts/LICENSE.txt`.
