//! Bevy plugin for the inspector.

use bevy_app::{App, Plugin, Update};
use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemState;
use godot::classes::Input;
use godot::global::Key;
use godot::obj::InstanceId;
use godot::prelude::*;

// Use the AppTypeRegistry from godot-bevy
use godot_bevy::prelude::AppTypeRegistry;

use crate::panels::{ComponentDataSerializer, EntityDataCollector, WorldInspectorWindow};

/// Resource holding the inspector window reference via instance ID.
/// We store the instance ID rather than `Gd<T>` because `Gd<T>` is not Send+Sync.
#[derive(Resource, Clone)]
pub struct InspectorWindowHandle {
    instance_id: InstanceId,
}

impl InspectorWindowHandle {
    /// Create a new handle from a window instance.
    #[allow(dead_code)]
    pub fn new(window: &Gd<WorldInspectorWindow>) -> Self {
        Self {
            instance_id: window.instance_id(),
        }
    }

    /// Get the window if it still exists.
    pub fn get_window(&self) -> Option<Gd<WorldInspectorWindow>> {
        Gd::try_from_instance_id(self.instance_id).ok()
    }
}

/// Resource tracking the currently selected entity.
#[derive(Resource, Default)]
pub struct InspectorSelection {
    /// The selected entity, if any.
    pub selected: Option<Entity>,
}

/// Event sent when an entity is selected in the inspector.
#[derive(Event, Clone)]
#[allow(dead_code)]
pub struct EntitySelectedEvent {
    pub entity: Entity,
}

/// Configuration for the inspector plugin.
#[derive(Clone, Resource)]
pub struct InspectorPluginConfig {
    /// Whether to show the inspector window on startup.
    pub show_on_startup: bool,
    /// The keyboard shortcut to toggle the inspector (as a Godot key string).
    pub toggle_key: Option<String>,
    /// Whether to update the inspector every frame.
    pub auto_refresh: bool,
    /// How often to refresh (in frames) when auto_refresh is true.
    pub refresh_interval: u32,
}

impl Default for InspectorPluginConfig {
    fn default() -> Self {
        Self {
            show_on_startup: false,
            toggle_key: Some("F12".to_string()),
            auto_refresh: true,
            refresh_interval: 10, // Every 10 frames
        }
    }
}

/// Plugin that adds the inspector functionality to your godot-bevy app.
///
/// # Example
///
/// ```rust,ignore
/// use godot_bevy_inspector::InspectorPlugin;
///
/// #[bevy_app]
/// fn build_app(app: &mut App) {
///     app.add_plugins(InspectorPlugin::default());
/// }
/// ```
pub struct InspectorPlugin {
    config: InspectorPluginConfig,
}

impl Default for InspectorPlugin {
    fn default() -> Self {
        Self {
            config: InspectorPluginConfig::default(),
        }
    }
}

impl InspectorPlugin {
    /// Create a new inspector plugin with custom configuration.
    pub fn new(config: InspectorPluginConfig) -> Self {
        Self { config }
    }

    /// Show the inspector window on startup.
    pub fn show_on_startup(mut self) -> Self {
        self.config.show_on_startup = true;
        self
    }

    /// Set the toggle key.
    pub fn with_toggle_key(mut self, key: impl Into<String>) -> Self {
        self.config.toggle_key = Some(key.into());
        self
    }

    /// Disable auto-refresh.
    pub fn without_auto_refresh(mut self) -> Self {
        self.config.auto_refresh = false;
        self
    }

    /// Set the refresh interval in frames.
    pub fn with_refresh_interval(mut self, frames: u32) -> Self {
        self.config.refresh_interval = frames;
        self
    }
}

impl Plugin for InspectorPlugin {
    fn build(&self, app: &mut App) {
        let config = self.config.clone();

        app.insert_resource(InspectorSelection::default())
            .insert_resource(config)
            .insert_resource(InspectorFrameCounter(0))
            .insert_resource(InspectorKeyState::default())
            .add_systems(Update, inspector_input_system)
            // Use exclusive systems for world access
            .add_systems(Update, inspector_refresh_exclusive_system)
            .add_systems(Update, inspector_selection_exclusive_system);
    }
}

#[derive(Resource)]
struct InspectorFrameCounter(u32);

#[derive(Resource, Default)]
struct InspectorKeyState {
    was_pressed: bool,
}

/// System that handles keyboard input for the inspector.
fn inspector_input_system(
    config: Res<InspectorPluginConfig>,
    window_handle: Option<Res<InspectorWindowHandle>>,
    mut key_state: ResMut<InspectorKeyState>,
) {
    // Check for toggle key press
    if let Some(ref key_str) = config.toggle_key {
        let input = Input::singleton();

        // Parse key string to Godot key
        let key = match key_str.as_str() {
            "F1" => Key::F1,
            "F2" => Key::F2,
            "F3" => Key::F3,
            "F4" => Key::F4,
            "F5" => Key::F5,
            "F6" => Key::F6,
            "F7" => Key::F7,
            "F8" => Key::F8,
            "F9" => Key::F9,
            "F10" => Key::F10,
            "F11" => Key::F11,
            "F12" => Key::F12,
            _ => return,
        };

        // Check for key press directly instead of using InputMap action
        let is_pressed = input.is_key_pressed(key);

        if is_pressed && !key_state.was_pressed {
            // Key just pressed - toggle the inspector
            if let Some(ref handle) = window_handle {
                if let Some(mut window) = handle.get_window() {
                    window.bind_mut().toggle_inspector();
                }
            }
        }

        key_state.was_pressed = is_pressed;
    }
}

/// Exclusive system that refreshes the inspector data.
fn inspector_refresh_exclusive_system(world: &mut World) {
    // First, get what we need from resources
    let (should_refresh, window_instance_id) = {
        let mut state: SystemState<(
            Res<InspectorPluginConfig>,
            ResMut<InspectorFrameCounter>,
            Option<Res<InspectorWindowHandle>>,
        )> = SystemState::new(world);

        let (config, mut counter, window_handle) = state.get_mut(world);

        if !config.auto_refresh {
            state.apply(world);
            return;
        }

        counter.0 += 1;
        if counter.0 < config.refresh_interval {
            state.apply(world);
            return;
        }
        counter.0 = 0;

        let instance_id = window_handle.as_ref().map(|h| h.instance_id);
        state.apply(world);

        (true, instance_id)
    };

    if !should_refresh {
        return;
    }

    let Some(instance_id) = window_instance_id else {
        return;
    };

    // Get the type registry
    let registry = {
        let type_registry = world.resource::<AppTypeRegistry>();
        type_registry.read()
    };

    // Now collect entity data with full world access
    let entity_data = EntityDataCollector::collect(world, &registry);

    // Update hierarchy in the window
    if let Ok(mut window) = Gd::<WorldInspectorWindow>::try_from_instance_id(instance_id) {
        if let Some(mut panel) = window.bind_mut().get_panel() {
            panel.bind_mut().update_hierarchy(entity_data);
        }
    }
}

/// Exclusive system that handles entity selection changes.
fn inspector_selection_exclusive_system(world: &mut World) {
    // First, check if selection changed and get necessary data
    let (selected_entity, window_instance_id, selection_changed) = {
        let mut state: SystemState<(Res<InspectorSelection>, Option<Res<InspectorWindowHandle>>)> =
            SystemState::new(world);

        let (selection, window_handle) = state.get(world);

        let changed = selection.is_changed();
        let entity = selection.selected;
        let instance_id = window_handle.as_ref().map(|h| h.instance_id);

        state.apply(world);

        (entity, instance_id, changed)
    };

    if !selection_changed {
        return;
    }

    let Some(instance_id) = window_instance_id else {
        return;
    };

    let Some(entity) = selected_entity else {
        if let Ok(mut window) = Gd::<WorldInspectorWindow>::try_from_instance_id(instance_id) {
            if let Some(mut panel) = window.bind_mut().get_panel() {
                panel.bind_mut().clear_inspector();
            }
        }
        return;
    };

    // Get the type registry
    let registry = {
        let type_registry = world.resource::<AppTypeRegistry>();
        type_registry.read()
    };

    // Get entity data with full world access
    let Ok(entity_ref) = world.get_entity(entity) else {
        return;
    };

    // Get entity name
    let name = entity_ref
        .get::<Name>()
        .map(|n| GString::from(n.as_str()))
        .unwrap_or_else(|| GString::from(&format!("Entity {}", entity.index())[..]));

    // Collect component data
    let mut components = VarDictionary::new();

    for component_id in entity_ref.archetype().components() {
        if let Some(component_info) = world.components().get_info(*component_id) {
            let component_name = component_info.name();

            // Try to get reflect data for this component
            if let Some(type_id) = component_info.type_id() {
                if let Some(registration) = registry.get(type_id) {
                    if let Some(reflect_component) = registration.data::<ReflectComponent>() {
                        if let Some(component) = reflect_component.reflect(entity_ref) {
                            let data = ComponentDataSerializer::serialize(component);
                            let name_str: &str = component_name.as_ref();
                            components.set(GString::from(name_str), data);
                        }
                    }
                }
            }
        }
    }

    // Update inspector
    if let Ok(mut window) = Gd::<WorldInspectorWindow>::try_from_instance_id(instance_id) {
        if let Some(mut panel) = window.bind_mut().get_panel() {
            panel
                .bind_mut()
                .inspect_entity(entity.to_bits(), name, components);
        }
    }
}
