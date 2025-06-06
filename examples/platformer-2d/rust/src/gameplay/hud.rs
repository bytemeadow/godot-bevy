use bevy::prelude::*;
use godot::classes::Label;
use godot_bevy::prelude::*;
use godot_bevy::utils::print_scene_tree;

use crate::gameplay::gem::GemsCollected;
use crate::level_manager::{CurrentLevel, LevelLoadedEvent};

#[derive(Resource, Default)]
pub struct HudHandles {
    pub current_level_label: Option<GodotNodeHandle>,
    pub gems_label: Option<GodotNodeHandle>,
}

#[derive(NodeTreeView)]
pub struct HudUi {
    #[node("/root/*/HUD/CurrentLevel")]
    pub current_level_label: GodotNodeHandle,
    #[node("/root/*/HUD/GemsLabel")]
    pub gems_label: GodotNodeHandle,
}

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HudHandles>()
            .add_systems(Update, (update_hud_on_level_loaded, update_gems_label));
    }
}

/// System to update HUD handles and set level label when a new level is loaded
fn update_hud_on_level_loaded(
    mut hud_handles: ResMut<HudHandles>,
    mut events: EventReader<LevelLoadedEvent>,
    current_level: Res<CurrentLevel>,
    mut scene_tree: SceneTreeRef,
) {
    print_scene_tree(&mut scene_tree);

    for _ in events.read() {
        // Try to get HUD node handles
        let root = scene_tree.get().get_root().unwrap();
        let mut hud_ui = HudUi::from_node(root);
        hud_handles.current_level_label = Some(hud_ui.current_level_label.clone());
        hud_handles.gems_label = Some(hud_ui.gems_label.clone());

        // Set the current level label
        if let Some(level_id) = current_level.level_id {
            hud_ui
                .current_level_label
                .get::<Label>()
                .set_text(level_id.display_name());
        }
    }
}

/// System to update the gems label when gems are collected
fn update_gems_label(gems_collected: Res<GemsCollected>, hud_handles: Res<HudHandles>) {
    if gems_collected.is_changed() {
        if let Some(gems_label) = &hud_handles.gems_label {
            let mut label_handle = gems_label.clone();
            label_handle
                .get::<Label>()
                .set_text(&format!("Gems: {}", gems_collected.0));
        }
    }
}
