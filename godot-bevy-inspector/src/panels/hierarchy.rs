//! Entity hierarchy tree panel.

use bevy_ecs::prelude::*;
use bevy_reflect::TypeRegistry;
use godot::classes::{
    Button, HBoxContainer, IPanelContainer, Label, LineEdit, PanelContainer, Tree, VBoxContainer,
    control, tree,
};
use godot::prelude::*;

/// A panel that displays the entity hierarchy as a tree.
#[derive(GodotClass)]
#[class(base=PanelContainer)]
pub struct HierarchyPanel {
    base: Base<PanelContainer>,
    tree: Option<Gd<Tree>>,
    /// The currently selected entity (as bits for FFI safety).
    selected_entity_bits: u64,
    /// Whether we have a valid selection.
    has_selection: bool,
}

#[godot_api]
impl IPanelContainer for HierarchyPanel {
    fn init(base: Base<PanelContainer>) -> Self {
        Self {
            base,
            tree: None,
            selected_entity_bits: 0,
            has_selection: false,
        }
    }

    fn ready(&mut self) {
        self.base_mut()
            .set_custom_minimum_size(Vector2::new(250.0, 0.0));

        // Create main layout
        let mut vbox = VBoxContainer::new_alloc();
        vbox.set_anchors_preset(control::LayoutPreset::FULL_RECT);

        // Header
        let mut header = HBoxContainer::new_alloc();
        let mut title = Label::new_alloc();
        title.set_text("Entities");
        header.add_child(&title);

        // Refresh button
        let mut refresh_btn = Button::new_alloc();
        refresh_btn.set_text("↻");
        refresh_btn.set_tooltip_text("Refresh entity list");
        let callable = self.base().callable("_on_refresh_pressed");
        refresh_btn.connect("pressed", &callable);
        header.add_child(&refresh_btn);

        vbox.add_child(&header);

        // Search filter
        let mut search = LineEdit::new_alloc();
        search.set_placeholder("Filter entities...");
        search.set_clear_button_enabled(true);
        let callable = self.base().callable("_on_filter_changed");
        search.connect("text_changed", &callable);
        vbox.add_child(&search);

        // Tree
        let mut tree_widget = Tree::new_alloc();
        tree_widget.set_v_size_flags(control::SizeFlags::EXPAND_FILL);
        tree_widget.set_hide_root(true);
        tree_widget.set_select_mode(tree::SelectMode::SINGLE);

        let callable = self.base().callable("_on_item_selected");
        tree_widget.connect("item_selected", &callable);

        vbox.add_child(&tree_widget.clone());
        self.tree = Some(tree_widget);

        self.base_mut().add_child(&vbox);
    }
}

#[godot_api]
impl HierarchyPanel {
    /// Signal emitted when an entity is selected.
    #[signal]
    fn entity_selected(entity_bits: u64);

    /// Create a new hierarchy panel.
    #[func]
    pub fn create() -> Gd<Self> {
        Gd::from_init_fn(|base| Self {
            base,
            tree: None,
            selected_entity_bits: 0,
            has_selection: false,
        })
    }

    /// Update the tree with entities from the world.
    /// This should be called from a Bevy system with world access.
    #[func]
    pub fn update_from_entity_data(&mut self, entity_data: VarDictionary) {
        let Some(ref mut tree_widget) = self.tree else {
            return;
        };

        tree_widget.clear();
        let root = tree_widget.create_item();

        // entity_data format: { entity_bits: { "name": String, "components": PackedStringArray } }
        for (key, value) in entity_data.iter_shared() {
            let entity_bits = key.to::<u64>();
            let info = value.to::<VarDictionary>();

            let name = info
                .get("name")
                .map(|v| v.to::<GString>())
                .unwrap_or_else(|| GString::from(&format!("Entity {:?}", entity_bits)[..]));

            let components = info
                .get("components")
                .map(|v| v.to::<PackedStringArray>())
                .unwrap_or_default();

            // Pass root as reference
            if let Some(mut item) = tree_widget.create_item_ex().parent(root.as_ref()).done() {
                item.set_text(0, &name);
                let variant = Variant::from(entity_bits);
                item.set_metadata(0, &variant);

                // Show component count as tooltip
                let component_list: Vec<String> = components
                    .as_slice()
                    .iter()
                    .map(|g| g.to_string())
                    .collect();
                let component_str = component_list.join(", ");
                item.set_tooltip_text(0, &format!("Components: {}", component_str));
            }
        }
    }

    /// Get the currently selected entity bits, or None if nothing selected.
    #[func]
    pub fn get_selected_entity(&self) -> u64 {
        self.selected_entity_bits
    }

    #[func]
    fn _on_item_selected(&mut self) {
        let Some(ref tree_widget) = self.tree else {
            return;
        };

        if let Some(item) = tree_widget.get_selected() {
            let metadata = item.get_metadata(0);
            if metadata.get_type() != VariantType::NIL {
                self.selected_entity_bits = metadata.to::<u64>();
                self.has_selection = true;
                let bits = self.selected_entity_bits;
                self.base_mut()
                    .emit_signal("entity_selected", &[Variant::from(bits)]);
            }
        }
    }

    #[func]
    fn _on_refresh_pressed(&mut self) {
        // This will be connected to trigger a refresh from Bevy
        // The actual refresh happens via update_from_entity_data
    }

    #[func]
    fn _on_filter_changed(&mut self, _text: GString) {
        // TODO: Implement filtering
    }
}

/// Helper struct for collecting entity data from Bevy world.
pub struct EntityDataCollector;

impl EntityDataCollector {
    /// Collect entity data from the world into a VarDictionary for the hierarchy panel.
    pub fn collect(world: &World, _type_registry: &TypeRegistry) -> VarDictionary {
        let mut dict = VarDictionary::new();

        // Iterate through all entities
        #[allow(deprecated)]
        for entity in world.iter_entities() {
            let entity_id = entity.id();
            let bits = entity_id.to_bits();

            let mut info = VarDictionary::new();

            // Get entity name if it has one
            let name = if let Some(name) = entity.get::<Name>() {
                GString::from(name.as_str())
            } else {
                GString::from(&format!("Entity {}", entity_id.index())[..])
            };
            info.set("name", name);

            // Collect component names
            let mut components = PackedStringArray::new();
            for component_id in entity.archetype().components() {
                if let Some(component_info) = world.components().get_info(*component_id) {
                    let name_string = component_info.name().to_string();
                    components.push(&GString::from(&name_string[..]));
                }
            }
            info.set("components", components);

            dict.set(bits, info);
        }

        dict
    }
}
