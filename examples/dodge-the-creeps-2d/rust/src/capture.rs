use bevy::ecs::system::SystemState;
use bevy::prelude::{
    Added, App, IntoScheduleConfigs, Last, OnExit, Query, Res, ResMut, Resource, State, Transform,
    With, World,
};
use godot::classes::{
    AnimatedSprite2D, Button, CheckButton, Input, Label, Node2D, RenderingServer, RigidBody2D,
};
use godot::prelude::*;
use godot_bevy::prelude::{GodotAccess, GodotNodeHandle, SceneTreeRef};
use godot_bevy_test::capture::{self, CaptureAdapter, CaptureManifest, Phase};

use crate::{
    GameState, Score,
    commands::{AnimationState, CachedScreenSize, UIHandles, VisibilityState},
    gameplay::{
        mob::{Mob, MobRng, MobSpawnTimer},
        player::Player,
    },
};

#[derive(Resource, Default)]
struct CapturedCreeps(u32);

pub(super) fn install(app: &mut App) {
    if !capture::is_enabled() {
        return;
    }
    app.insert_resource(MobRng(fastrand::Rng::with_seed(0)))
        .init_resource::<CapturedCreeps>()
        .add_systems(
            OnExit(GameState::MainMenu),
            hide_title.run_if(|| capture::phase() == Phase::Running),
        )
        .add_systems(
            Last,
            hold_creeps_at_spawn.run_if(|| capture::phase() == Phase::Running),
        );
    capture::install(
        app,
        CaptureAdapter {
            name: "dodge-the-creeps-2d",
            extension: |_, _, _| Ok(()),
            scenarios: &["title-and-first-creeps", "title-unseeded"],
            is_settled,
            reset,
            before_frame,
        },
    );
}

fn is_settled(world: &mut World, _: &CaptureManifest) -> Result<bool, String> {
    let mut state = SystemState::<(
        Res<State<GameState>>,
        Res<UIHandles>,
        Query<(&GodotNodeHandle, &CachedScreenSize), With<Player>>,
        SceneTreeRef,
        GodotAccess,
    )>::new(world);
    let (game_state, ui, players, mut tree, mut godot) =
        state.get_mut(world).map_err(|error| error.to_string())?;
    if *game_state.get() != GameState::MainMenu {
        return Ok(false);
    }
    let Some(scene) = tree.get().get_current_scene() else {
        return Ok(false);
    };
    let (Some(start), Some(message), Some(show_score), Some(score)) = (
        ui.start_button,
        ui.message_label,
        ui.show_score_button,
        ui.score_label,
    ) else {
        return Ok(false);
    };
    let start = godot.get::<Button>(start);
    let message = godot.get::<Label>(message);
    let show_score = godot.get::<CheckButton>(show_score);
    let score = godot.get::<Label>(score);
    if !start.is_node_ready()
        || start.is_disabled()
        || !start.is_visible_in_tree()
        || start.get_signal_connection_list("pressed").len() != 1
        || !message.is_node_ready()
        || !message.is_visible_in_tree()
        || message.get_text() != "Dodge the Creeps!"
        || !show_score.is_node_ready()
        || show_score.is_disabled()
        || !score.is_node_ready()
    {
        return Ok(false);
    }
    let Ok((handle, _)) = players.single() else {
        return Ok(false);
    };
    let mut player = godot.get::<Node2D>(*handle);
    if !player.is_node_ready() {
        return Ok(false);
    }
    let sprite = player.get_node_as::<AnimatedSprite2D>("AnimatedSprite2D");
    let Some(frames) = sprite.get_sprite_frames() else {
        return Ok(false);
    };
    for animation in frames.get_animation_names().as_slice() {
        let animation: StringName = animation.into();
        for frame in 0..frames.get_frame_count(&animation) {
            if !frames
                .get_frame_texture(&animation, frame)
                .is_some_and(|texture| texture.get_width() > 0 && texture.get_height() > 0)
            {
                return Ok(false);
            }
        }
    }
    if player.get_parent().as_ref() != Some(&scene) {
        player.reparent(&scene);
        player.set_name("Player");
        return Ok(false);
    }
    Ok(true)
}

fn reset_rng(rng: &mut MobRng, scenario: &str, seed: u64) {
    if scenario != "title-unseeded" {
        rng.0.seed(seed);
    }
}

fn reset(world: &mut World, manifest: &CaptureManifest) -> Result<(), String> {
    reset_rng(
        &mut world.resource_mut::<MobRng>(),
        &manifest.scenario,
        manifest.seed,
    );
    world.resource_mut::<MobSpawnTimer>().0.reset();
    world.resource_mut::<CapturedCreeps>().0 = 0;
    world.resource_mut::<Score>().0 = 0;
    RenderingServer::singleton().set_default_clear_color(Color::BLACK);
    let mut state = SystemState::<(
        Res<UIHandles>,
        Query<
            (
                &GodotNodeHandle,
                &mut Transform,
                &mut VisibilityState,
                &mut AnimationState,
            ),
            With<Player>,
        >,
        SceneTreeRef,
        GodotAccess,
    )>::new(world);
    let (ui, mut players, mut tree, mut godot) =
        state.get_mut(world).map_err(|error| error.to_string())?;
    let scene = tree
        .get()
        .get_current_scene()
        .ok_or("Main is missing at reset")?;
    let spawn = scene.get_node_as::<Node2D>("StartPosition").get_position();
    let (handle, mut transform, mut visibility, mut animation) =
        players.single_mut().map_err(|error| error.to_string())?;
    transform.translation.x = spawn.x;
    transform.translation.y = spawn.y;
    visibility.set_visible(false);
    *animation = AnimationState::default();
    let mut player = godot.get::<Node2D>(*handle);
    player.set_position(spawn);
    player.hide();
    let mut sprite = player.get_node_as::<AnimatedSprite2D>("AnimatedSprite2D");
    sprite.stop();
    sprite.set_frame(0);
    let mut title = godot.get::<Label>(ui.message_label.ok_or("Message is missing")?);
    title.set_text("Dodge the Creeps!");
    title.show();
    godot
        .get::<Button>(ui.start_button.ok_or("Start is missing")?)
        .show();
    let mut show_score =
        godot.get::<CheckButton>(ui.show_score_button.ok_or("Show score is missing")?);
    show_score.set_pressed_no_signal(true);
    show_score.show();
    let mut score = godot.get::<Label>(ui.score_label.ok_or("Score is missing")?);
    score.set_text("0");
    score.show();
    let mut input = godot.singleton::<Input>();
    for action in ["move_left", "move_right", "move_up", "move_down"] {
        input.action_release(action);
    }
    Ok(())
}

fn before_frame(world: &mut World, _: &CaptureManifest, frame: u32) -> Result<(), String> {
    if frame != 1 {
        return Ok(());
    }
    let mut state = SystemState::<(Res<UIHandles>, GodotAccess)>::new(world);
    let (ui, mut godot) = state.get_mut(world).map_err(|error| error.to_string())?;
    let handle = ui.start_button.ok_or("Start is missing at frame 1")?;
    godot.get::<Button>(handle).emit_signal("pressed", &[]);
    Ok(())
}

fn hide_title(ui: Res<UIHandles>, mut godot: GodotAccess) {
    godot.get::<Label>(ui.message_label.unwrap()).hide();
}

fn hold_creeps_at_spawn(
    creeps: Query<&GodotNodeHandle, (With<Mob>, Added<GodotNodeHandle>)>,
    mut count: ResMut<CapturedCreeps>,
    mut tree: SceneTreeRef,
    mut godot: GodotAccess,
) {
    let scene = tree.get().get_current_scene().unwrap();
    for handle in &creeps {
        count.0 += 1;
        let mut creep = godot.get::<RigidBody2D>(*handle);
        creep.set_freeze_enabled(true);
        creep.reparent(&scene);
        creep.set_name(&format!("CaptureCreep{}", count.0));
    }
}

#[cfg(test)]
mod tests {
    use super::reset_rng;
    use crate::gameplay::mob::MobRng;

    fn first_spawn_position(rng: &mut MobRng) -> [f32; 2] {
        let distance = rng.0.f32() * 2400.0;
        if distance < 480.0 {
            [distance, 0.0]
        } else if distance < 1200.0 {
            [480.0, distance - 480.0]
        } else if distance < 1680.0 {
            [1680.0 - distance, 720.0]
        } else {
            [0.0, 2400.0 - distance]
        }
    }

    #[test]
    fn seed_five_spawns_at_the_stated_position() {
        let mut rng = MobRng(fastrand::Rng::with_seed(0));
        reset_rng(&mut rng, "title-and-first-creeps", 5);
        assert_eq!(first_spawn_position(&mut rng), [416.2791, 0.0]);
    }

    #[test]
    fn unseeded_scenario_retains_the_wrong_spawn_position() {
        let mut rng = MobRng(fastrand::Rng::with_seed(0));
        reset_rng(&mut rng, "title-unseeded", 5);
        let position = first_spawn_position(&mut rng);
        assert_eq!(position, [233.69373, 720.0]);
        assert_ne!(position, [416.2791, 0.0]);
    }
}
