use bevy::prelude::*;
use godot::classes::Label;
use godot_bevy::prelude::*;

use crate::gameplay::gem::GemsCollected;
use crate::level_manager::LevelLoadedMessage;

/// Event to request HUD updates
///
/// This decouples HUD updates from direct resource access,
/// allowing better parallelization with other game systems.
#[derive(Event, Debug, Clone)]
pub enum HudUpdateMessage {
    GemsChanged(i64),
}

#[derive(Resource, Default)]
pub struct HudHandles {
    pub current_level_label: Option<GodotNodeHandle>,
    pub gems_label: Option<GodotNodeHandle>,
}

impl HudHandles {
    /// Clear all HUD handles (useful when scene changes invalidate them)
    pub fn clear(&mut self) {
        self.current_level_label = None;
        self.gems_label = None;
    }
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
            .add_observer(on_level_loaded_setup_hud)
            .add_observer(on_hud_update);
    }
}

/// Observer to set up HUD handles and update displays when a new level is loaded
fn on_level_loaded_setup_hud(
    trigger: On<LevelLoadedMessage>,
    mut hud_handles: ResMut<HudHandles>,
    mut scene_tree: SceneTreeRef,
    gems_collected: Res<GemsCollected>,
    mut commands: Commands,
    mut godot: GodotAccess,
) {
    let event = trigger.event();

    // Try to get HUD node handles - this is the only SceneTreeRef access in HUD
    let root = scene_tree.get().get_root().unwrap();
    let hud_ui = HudUi::from_node(root).unwrap();
    hud_handles.current_level_label = Some(hud_ui.current_level_label);
    hud_handles.gems_label = Some(hud_ui.gems_label);

    // Set the current level label immediately
    let mut label = godot.get::<Label>(hud_ui.current_level_label);
    label.set_text(event.level_id.display_name());

    // Request HUD gem update via events
    commands.trigger(HudUpdateMessage::GemsChanged(gems_collected.0));
}

/// Observer that handles HUD update events
fn on_hud_update(
    trigger: On<HudUpdateMessage>,
    hud_handles: Res<HudHandles>,
    mut godot: GodotAccess,
) {
    match trigger.event() {
        HudUpdateMessage::GemsChanged(gem_count) => {
            if let Some(handle) = hud_handles.gems_label
                && let Some(mut label) = godot.try_get::<Label>(handle)
            {
                label.set_text(&format!("Gems: {gem_count}"));
            }
        }
    }
}
