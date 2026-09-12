use crate::GameState;
use crate::components::Player;
use crate::gameplay::audio::GameAudio;
use crate::gameplay::player::PlayerSystemSet;
use crate::level_manager::{CurrentLevel, LevelId, PendingLevel};
use crate::main_menu::MenuAssets;
use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use godot::classes::{
    AnimatedSprite2D, Button, Camera2D, CharacterBody2D, Input, InputEventKey, InputMap,
    camera_2d::Camera2DProcessCallback,
};
use godot::global::Key;
use godot::prelude::*;
use godot_bevy::prelude::{GodotAccess, GodotNodeHandle, GodotResource, SceneTreeRef};
use godot_bevy_test::capture::{self, CaptureAdapter, CaptureManifest, Phase};

pub(super) const PLAYER_SPAWN_POSITION: [f32; 2] = [400.0, 184.0];
const TITLE_SCENE: &str = "res://scenes/levels/main_menu.tscn";
const LEVEL_SCENE: &str = "res://scenes/levels/level_1.tscn";

#[derive(Resource, Default, PartialEq, Eq)]
enum StartAction {
    #[default]
    Waiting,
    Pressed,
    Released,
}

pub(super) fn install(app: &mut App) {
    if !capture::is_enabled() {
        return;
    }

    app.init_resource::<StartAction>().configure_sets(
        FixedUpdate,
        (
            PlayerSystemSet::InputDetection,
            PlayerSystemSet::Movement,
            PlayerSystemSet::Animation,
        )
            .run_if(|| capture::phase() == Phase::Running),
    );
    capture::install(
        app,
        CaptureAdapter {
            name: "platformer-render",
            extension: |_, _, _| Ok(()),
            scenarios: &["title", "level", "level-wrong-spawn"],
            is_settled,
            reset,
            before_frame: |_, _, _| Ok(()),
        },
    );
}

fn start_key(pressed: bool) -> Gd<InputEventKey> {
    let mut event = InputEventKey::new_gd();
    event.set_keycode(Key::ENTER);
    event.set_physical_keycode(Key::NONE);
    event.set_pressed(pressed);
    event
}

fn is_settled(world: &mut World, manifest: &CaptureManifest) -> Result<bool, String> {
    if manifest.scenario != "title" && *world.resource::<StartAction>() == StartAction::Pressed {
        Input::singleton().parse_input_event(&start_key(false));
        *world.resource_mut::<StartAction>() = StartAction::Released;
        return Ok(false);
    }

    let Some(audio) = world.get_resource::<GameAudio>() else {
        return Ok(false);
    };
    let assets = world.resource::<Assets<GodotResource>>();
    if [
        &audio.action_theme,
        &audio.waltz_theme,
        &audio.jump_sound,
        &audio.gem_sound,
    ]
    .iter()
    .any(|handle| assets.get(*handle).is_none())
    {
        return Ok(false);
    }

    let in_menu = *world.resource::<State<GameState>>().get() == GameState::MainMenu;
    let menu = world.resource::<MenuAssets>();
    let menu_ready = in_menu && menu.initialized && menu.signals_connected;
    let level_ready = *world.resource::<State<GameState>>().get() == GameState::InGame
        && world.resource::<CurrentLevel>().level_id == Some(LevelId::Level1)
        && world.resource::<PendingLevel>().level_id.is_none();
    let mut state = SystemState::<(
        SceneTreeRef,
        GodotAccess,
        Query<&GodotNodeHandle, With<Player>>,
        ResMut<StartAction>,
    )>::new(world);
    let (mut tree, mut godot, players, mut start) =
        state.get_mut(world).map_err(|error| error.to_string())?;
    let Some(scene) = tree.get().get_current_scene() else {
        return Ok(false);
    };

    if scene.get_scene_file_path() == TITLE_SCENE {
        if !menu_ready {
            return Ok(false);
        }
        let Some(mut button) = scene
            .get_node_or_null("Options/StartButton")
            .and_then(|node| node.try_cast::<Button>().ok())
        else {
            return Ok(false);
        };
        if !button.is_node_ready()
            || button.is_disabled()
            || button.get_signal_connection_list("pressed").is_empty()
        {
            return Ok(false);
        }
        if manifest.scenario == "title" {
            return Ok(true);
        }
        if *start == StartAction::Waiting {
            let event = start_key(true);
            if !InputMap::singleton().action_has_event("ui_accept", &event) {
                return Err("ui_accept must bind Enter by keycode as in project.godot".into());
            }
            button.grab_focus();
            Input::singleton().parse_input_event(&event);
            *start = StartAction::Pressed;
        }
        return Ok(false);
    }

    if manifest.scenario == "title"
        || scene.get_scene_file_path() != LEVEL_SCENE
        || !level_ready
        || *start != StartAction::Released
        || Input::singleton().is_action_pressed("ui_accept")
    {
        return Ok(false);
    }
    let Ok(handle) = players.single() else {
        return Ok(false);
    };
    let Some(mut player) = godot.try_get::<CharacterBody2D>(*handle) else {
        return Ok(false);
    };
    if !player.is_node_ready() || player.get_parent().as_ref() != Some(&scene) {
        return Ok(false);
    }
    let sprite = player.get_node_as::<AnimatedSprite2D>("AnimatedSprite2D");
    let Some(frames) = sprite.get_sprite_frames() else {
        return Ok(false);
    };
    for animation in ["idle", "jump", "run"] {
        if !frames.has_animation(animation) || frames.get_frame_count(animation) == 0 {
            return Ok(false);
        }
        for frame in 0..frames.get_frame_count(animation) {
            if !frames
                .get_frame_texture(animation, frame)
                .is_some_and(|texture| texture.get_width() > 0 && texture.get_height() > 0)
            {
                return Ok(false);
            }
        }
    }
    if player.get_name() != "Player2D" {
        player.set_name("Player2D");
        return Ok(false);
    }
    Ok(true)
}

fn reset(world: &mut World, manifest: &CaptureManifest) -> Result<(), String> {
    fastrand::seed(manifest.seed);
    if manifest.scenario == "title" {
        return Ok(());
    }

    let mut state = SystemState::<(
        Query<(&GodotNodeHandle, &mut Transform), With<Player>>,
        GodotAccess,
    )>::new(world);
    let (mut players, mut godot) = state.get_mut(world).map_err(|error| error.to_string())?;
    let (handle, mut transform) = players.single_mut().map_err(|error| error.to_string())?;
    let position = Vector2::new(PLAYER_SPAWN_POSITION[0], PLAYER_SPAWN_POSITION[1]);
    transform.translation.x = position.x;
    transform.translation.y = position.y;
    let mut player = godot.get::<CharacterBody2D>(*handle);
    player.set_position(position);
    player.set_velocity(Vector2::ZERO);
    player.set_visible(true);

    let mut sprite = player.get_node_as::<AnimatedSprite2D>("AnimatedSprite2D");
    sprite.stop();
    sprite.set_animation("idle");
    sprite.set_frame_and_progress(0, 0.0);
    sprite.set_flip_h(false);
    sprite.set_visible(true);

    let mut camera = player.get_node_as::<Camera2D>("Camera2D");
    camera.set_drag_horizontal_enabled(false);
    camera.set_drag_vertical_enabled(false);
    camera.set_position_smoothing_enabled(false);
    camera.set_process_callback(Camera2DProcessCallback::IDLE);
    camera.reset_smoothing();
    camera.force_update_scroll();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::PLAYER_SPAWN_POSITION;

    #[test]
    fn player_spawn_matches_level_scene() {
        let scene = include_str!("../../godot/scenes/levels/level_1.tscn");
        let player = scene
            .split("\n\n")
            .find(|section| section.starts_with("[node name=\"Player\" parent=\".\" "))
            .expect("authored Player instance");
        let position: Vec<f32> = player
            .lines()
            .find_map(|line| line.strip_prefix("position = Vector2("))
            .expect("authored spawn position")
            .trim_end_matches(')')
            .split(',')
            .map(|axis| axis.trim().parse().unwrap())
            .collect();

        assert_eq!(PLAYER_SPAWN_POSITION.as_slice(), position.as_slice());
    }
}
