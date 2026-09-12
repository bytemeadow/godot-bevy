use bevy::{prelude::*, state::app::StatesPlugin};
use godot::classes::{CheckButton, Label, PackedScene};
use godot::prelude::*;
use godot_bevy_test::prelude::*;

use crate::{GameState, commands::CommandSystemPlugin, main_menu::MainMenuPlugin};

#[itest]
async fn menu_score_setting_reaches_observer(ctx: TestContext) {
    let mut main = load::<PackedScene>("res://scenes/main.tscn")
        .instantiate()
        .unwrap();
    let mut root = ctx.scene_tree.get_tree().get_root().unwrap();
    root.add_child(&main);

    let mut app = TestApp::new(&ctx, |app| {
        app.add_plugins(StatesPlugin)
            .init_state::<GameState>()
            .add_plugins((CommandSystemPlugin, MainMenuPlugin));
        app.world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(GameState::MainMenu);
    })
    .await;

    assert!(main.has_node("HUD/ShowScoreButton"));
    let mut toggle = main.get_node_as::<CheckButton>("HUD/ShowScoreButton");
    let score = main.get_node_as::<Label>("HUD/ScoreLabel");
    assert!(!toggle.is_disabled());
    assert!(score.is_visible());

    toggle.set_pressed(false);
    app.updates(2).await;
    assert!(
        !score.is_visible(),
        "the menu event must reach the Bevy observer and hide the score"
    );

    toggle.set_pressed(true);
    app.updates(2).await;
    assert!(score.is_visible(), "the observer must show the score again");

    app.cleanup().await;
    main.queue_free();
}
