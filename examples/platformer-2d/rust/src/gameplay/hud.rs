use bevy::prelude::*;
use godot::classes::Label;
use godot_bevy::prelude::*;

use crate::gameplay::gem::{GemCollectedEvent, GemsCollected};
use crate::level_manager::LevelLoadedEvent;

/// System sets for HUD operations that can run in parallel when possible
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum HudSystemSet {
    /// Handle setup and major updates (level changes, etc.)
    Setup,
    /// Handle incremental updates (gem counts, etc.)
    IncrementalUpdates,
}

/// Event to request HUD updates
///
/// This decouples HUD updates from direct resource access,
/// allowing better parallelization with other game systems.
#[derive(Event, Debug)]
pub enum HudUpdateEvent {
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
            .add_event::<HudUpdateEvent>()
            .add_systems(
                Update,
                (
                    // Setup systems - handle major UI changes
                    setup_hud_on_level_loaded.in_set(HudSystemSet::Setup),
                    // Incremental update systems - can run in parallel
                    (generate_hud_update_events, handle_hud_update_events)
                        .in_set(HudSystemSet::IncrementalUpdates),
                ),
            );
    }
}

/// System to set up HUD handles and update displays when a new level is loaded
///
/// Simplified approach that still reduces SceneTreeRef conflicts by batching operations.
#[main_thread_system]
fn setup_hud_on_level_loaded(
    mut hud_handles: ResMut<HudHandles>,
    mut events: EventReader<LevelLoadedEvent>,
    mut scene_tree: SceneTreeRef,
    mut hud_update_events: EventWriter<HudUpdateEvent>,
    gems_collected: Res<GemsCollected>,
) {
    for event in events.read() {
        // Try to get HUD node handles - this is the only SceneTreeRef access in HUD
        let root = scene_tree.get().get_root().unwrap();
        let mut hud_ui = HudUi::from_node(root);
        hud_handles.current_level_label = Some(hud_ui.current_level_label.clone());
        hud_handles.gems_label = Some(hud_ui.gems_label.clone());

        // Set the current level label immediately
        hud_ui
            .current_level_label
            .get::<Label>()
            .set_text(event.level_id.display_name());

        // Request HUD gem update via events
        hud_update_events.write(HudUpdateEvent::GemsChanged(gems_collected.0));
    }
}

/// System to generate HUD update events based on game state changes
///
/// This system runs in parallel with other incremental update systems
/// and converts state changes to events for loose coupling.
fn generate_hud_update_events(
    gems_collected: Res<GemsCollected>,
    mut gem_events: EventReader<GemCollectedEvent>,
    mut hud_update_events: EventWriter<HudUpdateEvent>,
) {
    // Generate gem update events when gems are collected
    for _event in gem_events.read() {
        hud_update_events.write(HudUpdateEvent::GemsChanged(gems_collected.0));
    }
}

/// System to handle HUD update events
///
/// This system can run in parallel with other incremental update systems
/// since it only responds to events and updates UI elements.
#[main_thread_system]
fn handle_hud_update_events(
    mut hud_events: EventReader<HudUpdateEvent>,
    hud_handles: Res<HudHandles>,
) {
    for event in hud_events.read() {
        match event {
            HudUpdateEvent::GemsChanged(gem_count) => {
                if let Some(gems_label) = &hud_handles.gems_label {
                    let mut label_handle = gems_label.clone();
                    label_handle
                        .get::<Label>()
                        .set_text(&format!("Gems: {gem_count}"));
                }
            }
        }
    }
}
