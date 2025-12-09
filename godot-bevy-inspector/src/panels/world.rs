//! Combined world inspector panel.

use godot::classes::{Control, HSplitContainer, IHSplitContainer, IWindow, Window, control};
use godot::prelude::*;

use super::{HierarchyPanel, InspectorPanel};

/// A combined panel that shows entity hierarchy and component inspector side by side.
#[derive(GodotClass)]
#[class(base=HSplitContainer)]
pub struct WorldInspectorPanel {
    base: Base<HSplitContainer>,
    hierarchy: Option<Gd<HierarchyPanel>>,
    inspector: Option<Gd<InspectorPanel>>,
}

#[godot_api]
impl IHSplitContainer for WorldInspectorPanel {
    fn init(base: Base<HSplitContainer>) -> Self {
        Self {
            base,
            hierarchy: None,
            inspector: None,
        }
    }

    fn ready(&mut self) {
        // Configure split container
        self.base_mut().set_split_offset(250);
        self.base_mut()
            .set_h_size_flags(control::SizeFlags::EXPAND_FILL);
        self.base_mut()
            .set_v_size_flags(control::SizeFlags::EXPAND_FILL);

        // Create hierarchy panel
        let hierarchy = HierarchyPanel::create();
        self.base_mut()
            .add_child(&hierarchy.clone().upcast::<Node>());

        // Connect hierarchy selection to inspector
        let callable = self.base().callable("_on_entity_selected");
        hierarchy
            .clone()
            .upcast::<Node>()
            .connect("entity_selected", &callable);

        self.hierarchy = Some(hierarchy);

        // Create inspector panel
        let inspector = InspectorPanel::create();
        self.base_mut()
            .add_child(&inspector.clone().upcast::<Node>());
        self.inspector = Some(inspector);
    }
}

#[godot_api]
impl WorldInspectorPanel {
    /// Signal emitted when an entity is selected for inspection.
    #[signal]
    fn entity_selected(entity_bits: u64);

    /// Create a new world inspector panel.
    #[func]
    pub fn create() -> Gd<Self> {
        Gd::from_init_fn(|base| Self {
            base,
            hierarchy: None,
            inspector: None,
        })
    }

    /// Get the hierarchy panel.
    #[func]
    pub fn get_hierarchy(&self) -> Option<Gd<HierarchyPanel>> {
        self.hierarchy.clone()
    }

    /// Get the inspector panel.
    #[func]
    pub fn get_inspector(&self) -> Option<Gd<InspectorPanel>> {
        self.inspector.clone()
    }

    /// Update the hierarchy with entity data.
    #[func]
    pub fn update_hierarchy(&mut self, entity_data: VarDictionary) {
        if let Some(ref mut hierarchy) = self.hierarchy {
            hierarchy.bind_mut().update_from_entity_data(entity_data);
        }
    }

    /// Inspect an entity.
    #[func]
    pub fn inspect_entity(
        &mut self,
        entity_bits: u64,
        entity_name: GString,
        components: VarDictionary,
    ) {
        if let Some(ref mut inspector) = self.inspector {
            inspector
                .bind_mut()
                .inspect_entity(entity_bits, entity_name, components);
        }
    }

    /// Clear the inspector.
    #[func]
    pub fn clear_inspector(&mut self) {
        if let Some(ref mut inspector) = self.inspector {
            inspector.bind_mut().clear();
        }
    }

    #[func]
    fn _on_entity_selected(&mut self, entity_bits: u64) {
        // Re-emit for external listeners
        self.base_mut()
            .emit_signal("entity_selected", &[Variant::from(entity_bits)]);
    }
}

/// A floating window version of the world inspector.
#[derive(GodotClass)]
#[class(base=Window)]
pub struct WorldInspectorWindow {
    base: Base<Window>,
    panel: Option<Gd<WorldInspectorPanel>>,
}

#[godot_api]
impl IWindow for WorldInspectorWindow {
    fn init(base: Base<Window>) -> Self {
        Self { base, panel: None }
    }

    fn ready(&mut self) {
        self.base_mut().set_title("Bevy Inspector");
        self.base_mut().set_size(Vector2i::new(800, 600));
        self.base_mut().set_min_size(Vector2i::new(400, 300));

        // Create the panel
        let panel = WorldInspectorPanel::create();
        panel
            .clone()
            .upcast::<Control>()
            .set_anchors_preset(control::LayoutPreset::FULL_RECT);
        self.base_mut().add_child(&panel.clone().upcast::<Node>());
        self.panel = Some(panel);

        // Connect close request
        let callable = self.base().callable("_on_close_requested");
        self.base_mut().connect("close_requested", &callable);
    }
}

#[godot_api]
impl WorldInspectorWindow {
    /// Signal emitted when an entity is selected.
    #[signal]
    fn entity_selected(entity_bits: u64);

    /// Create a new inspector window.
    #[func]
    pub fn create() -> Gd<Self> {
        Gd::from_init_fn(|base| Self { base, panel: None })
    }

    /// Get the world inspector panel.
    #[func]
    pub fn get_panel(&self) -> Option<Gd<WorldInspectorPanel>> {
        self.panel.clone()
    }

    /// Show the inspector window.
    #[func]
    pub fn show_inspector(&mut self) {
        self.base_mut().show();
        self.base_mut().grab_focus();
    }

    /// Hide the inspector window.
    #[func]
    pub fn hide_inspector(&mut self) {
        self.base_mut().hide();
    }

    /// Toggle the inspector window visibility.
    #[func]
    pub fn toggle_inspector(&mut self) {
        if self.base().is_visible() {
            self.hide_inspector();
        } else {
            self.show_inspector();
        }
    }

    #[func]
    fn _on_close_requested(&mut self) {
        self.hide_inspector();
    }
}
