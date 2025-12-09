//! Bevy plugin for the inspector.

use bevy_app::{App, Plugin, Update};
use bevy_ecs::prelude::*;
use godot::classes::Input;
use godot::global::Key;
use godot::obj::InstanceId;
use godot::prelude::*;

// Use the AppTypeRegistry from godot-bevy
use godot_bevy::prelude::AppTypeRegistry;

use crate::panels::{ComponentDataSerializer, EntityDataCollector, WorldInspectorWindow};

/// Resource holding the inspector window reference via instance ID.
/// We store the instance ID rather than `Gd<T>` because `Gd<T>` is not Send+Sync.
#[derive(Resource)]
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
#[derive(Clone)]
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
            .insert_resource(InspectorConfig(config))
            .insert_resource(InspectorFrameCounter(0))
            .add_systems(
                Update,
                (
                    inspector_input_system,
                    inspector_refresh_system,
                    inspector_selection_system,
                )
                    .chain(),
            );
    }
}

#[derive(Resource)]
struct InspectorConfig(InspectorPluginConfig);

#[derive(Resource)]
struct InspectorFrameCounter(u32);

/// System that handles keyboard input for the inspector.
fn inspector_input_system(
    config: Res<InspectorConfig>,
    window_handle: Option<Res<InspectorWindowHandle>>,
) {
    // Check for toggle key press
    if let Some(ref key_str) = config.0.toggle_key {
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

        let action_name = format!("toggle_inspector_{}", key_str.to_lowercase());
        if input.is_action_just_pressed(&action_name) {
            if let Some(ref handle) = window_handle {
                if let Some(mut window) = handle.get_window() {
                    window.bind_mut().toggle_inspector();
                }
            }
        }

        // Suppress unused variable warning
        let _ = key;
    }
}

/// System that refreshes the inspector data.
fn inspector_refresh_system(
    config: Res<InspectorConfig>,
    mut counter: ResMut<InspectorFrameCounter>,
    window_handle: Option<Res<InspectorWindowHandle>>,
    world: &World,
    type_registry: Res<AppTypeRegistry>,
) {
    if !config.0.auto_refresh {
        return;
    }

    counter.0 += 1;
    if counter.0 < config.0.refresh_interval {
        return;
    }
    counter.0 = 0;

    // Update hierarchy data
    if let Some(ref handle) = window_handle {
        if let Some(mut window) = handle.get_window() {
            let registry = type_registry.read();
            let entity_data = EntityDataCollector::collect(world, &registry);

            if let Some(mut panel) = window.bind_mut().get_panel() {
                panel.bind_mut().update_hierarchy(entity_data);
            }
        }
    }
}

/// System that handles entity selection changes.
fn inspector_selection_system(
    selection: Res<InspectorSelection>,
    window_handle: Option<Res<InspectorWindowHandle>>,
    world: &World,
    type_registry: Res<AppTypeRegistry>,
) {
    if !selection.is_changed() {
        return;
    }

    let Some(ref handle) = window_handle else {
        return;
    };
    let Some(mut window) = handle.get_window() else {
        return;
    };
    let Some(entity) = selection.selected else {
        if let Some(mut panel) = window.bind_mut().get_panel() {
            panel.bind_mut().clear_inspector();
        }
        return;
    };

    // Get entity data
    let Ok(entity_ref) = world.get_entity(entity) else {
        return;
    };

    let registry = type_registry.read();

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
    if let Some(mut panel) = window.bind_mut().get_panel() {
        panel
            .bind_mut()
            .inspect_entity(entity.to_bits(), name, components);
    }
}
