//! Component inspector panel.

use bevy_reflect::PartialReflect;
use godot::classes::{
    Button, CheckBox, ColorPickerButton, Control, HBoxContainer, HSeparator, IPanelContainer,
    Label, LineEdit, MarginContainer, PanelContainer, ScrollContainer, SpinBox, VBoxContainer,
    control, scroll_container,
};
use godot::global::HorizontalAlignment;
use godot::prelude::*;

use crate::widgets::{self, CollapsibleSection};

/// A panel that displays and allows editing of an entity's components.
#[derive(GodotClass)]
#[class(base=PanelContainer)]
pub struct InspectorPanel {
    base: Base<PanelContainer>,
    content: Option<Gd<VBoxContainer>>,
    scroll: Option<Gd<ScrollContainer>>,
    /// The entity being inspected (as bits).
    current_entity_bits: u64,
    /// Title label showing entity info.
    title_label: Option<Gd<Label>>,
}

#[godot_api]
impl IPanelContainer for InspectorPanel {
    fn init(base: Base<PanelContainer>) -> Self {
        Self {
            base,
            content: None,
            scroll: None,
            current_entity_bits: 0,
            title_label: None,
        }
    }

    fn ready(&mut self) {
        self.base_mut()
            .set_custom_minimum_size(Vector2::new(300.0, 0.0));
        self.base_mut()
            .set_h_size_flags(control::SizeFlags::EXPAND_FILL);
        self.base_mut()
            .set_v_size_flags(control::SizeFlags::EXPAND_FILL);

        // Create main layout
        let mut vbox = VBoxContainer::new_alloc();
        vbox.set_anchors_preset(control::LayoutPreset::FULL_RECT);
        vbox.set_h_size_flags(control::SizeFlags::EXPAND_FILL);
        vbox.set_v_size_flags(control::SizeFlags::EXPAND_FILL);

        // Header
        let mut header = HBoxContainer::new_alloc();
        let mut title = Label::new_alloc();
        title.set_text("Inspector");
        header.add_child(&title.clone());
        self.title_label = Some(title);
        vbox.add_child(&header);

        // Separator
        vbox.add_child(&HSeparator::new_alloc());

        // Scroll container for components
        let mut scroll = ScrollContainer::new_alloc();
        scroll.set_h_size_flags(control::SizeFlags::EXPAND_FILL);
        scroll.set_v_size_flags(control::SizeFlags::EXPAND_FILL);
        scroll.set_horizontal_scroll_mode(scroll_container::ScrollMode::DISABLED);

        // Content container
        let mut content = VBoxContainer::new_alloc();
        content.set_h_size_flags(control::SizeFlags::EXPAND_FILL);

        // Placeholder text
        let mut placeholder = Label::new_alloc();
        placeholder.set_text("Select an entity to inspect");
        placeholder.add_theme_color_override("font_color", Color::from_rgb(0.6, 0.6, 0.6));
        content.add_child(&placeholder);

        scroll.add_child(&content.clone());
        vbox.add_child(&scroll.clone());

        self.scroll = Some(scroll);
        self.content = Some(content);
        self.base_mut().add_child(&vbox);
    }
}

#[godot_api]
impl InspectorPanel {
    /// Signal emitted when a component value is changed.
    #[signal]
    fn component_changed(entity_bits: u64, component_name: GString);

    /// Create a new inspector panel.
    #[func]
    pub fn create() -> Gd<Self> {
        Gd::from_init_fn(|base| Self {
            base,
            content: None,
            scroll: None,
            current_entity_bits: 0,
            title_label: None,
        })
    }

    /// Clear the inspector content.
    #[func]
    pub fn clear(&mut self) {
        if let Some(ref mut content) = self.content {
            // Remove all children
            for mut child in content.get_children().iter_shared() {
                child.queue_free();
            }

            // Add placeholder
            let mut placeholder = Label::new_alloc();
            placeholder.set_text("Select an entity to inspect");
            placeholder.add_theme_color_override("font_color", Color::from_rgb(0.6, 0.6, 0.6));
            content.add_child(&placeholder);
        }

        if let Some(ref mut title) = self.title_label {
            title.set_text("Inspector");
        }

        self.current_entity_bits = 0;
    }

    /// Set the entity being inspected.
    /// Takes entity bits and a VarDictionary of component data.
    #[func]
    pub fn inspect_entity(
        &mut self,
        entity_bits: u64,
        entity_name: GString,
        components: VarDictionary,
    ) {
        self.current_entity_bits = entity_bits;

        // Update title
        if let Some(ref mut title) = self.title_label {
            title.set_text(&format!("Inspector - {}", entity_name));
        }

        // Collect component data first
        let component_list: Vec<(GString, VarDictionary)> = components
            .iter_shared()
            .map(|(name, data)| (name.to::<GString>(), data.to::<VarDictionary>()))
            .collect();

        // Clear current content
        if let Some(ref mut content) = self.content {
            for mut child in content.get_children().iter_shared() {
                child.queue_free();
            }

            // Add UI for each component
            for (component_name, component_data) in component_list {
                // Create collapsible section for each component
                let section = CollapsibleSection::create(component_name.clone(), true);
                content.add_child(&section);

                // Add component fields
                Self::add_component_ui_static(content, &component_name, &component_data);
            }
        }
    }

    /// Add UI for a single component (static version to avoid borrow issues).
    fn add_component_ui_static(
        parent: &mut Gd<VBoxContainer>,
        name: &GString,
        data: &VarDictionary,
    ) {
        let mut section_vbox = widgets::vbox();

        // Header with component name
        let mut header = Button::new_alloc();
        header.set_text(&format!("▼ {}", name));
        header.set_flat(true);
        header.set_text_alignment(HorizontalAlignment::LEFT);
        section_vbox.add_child(&header);

        // Content with fields
        let mut fields_container = widgets::vbox();
        let mut margin = MarginContainer::new_alloc();
        margin.add_theme_constant_override("margin_left", 16);
        margin.set_h_size_flags(control::SizeFlags::EXPAND_FILL);

        for (field_name, field_value) in data.iter_shared() {
            let mut row = widgets::hbox();

            // Field label
            let mut label = Label::new_alloc();
            label.set_text(&field_name.to::<GString>());
            label.set_custom_minimum_size(Vector2::new(120.0, 0.0));
            row.add_child(&label);

            // Field value widget based on type
            let widget = Self::create_widget_for_variant_static(&field_value);
            row.add_child(&widget);

            fields_container.add_child(&row);
        }

        margin.add_child(&fields_container);
        section_vbox.add_child(&margin);
        section_vbox.add_child(&HSeparator::new_alloc());

        parent.add_child(&section_vbox);
    }

    /// Create an appropriate widget for a Variant value (static version).
    fn create_widget_for_variant_static(value: &Variant) -> Gd<Control> {
        match value.get_type() {
            VariantType::BOOL => {
                let mut checkbox = CheckBox::new_alloc();
                checkbox.set_pressed(value.to::<bool>());
                checkbox.upcast()
            }
            VariantType::INT => {
                let mut spinbox = SpinBox::new_alloc();
                spinbox.set_value(value.to::<i64>() as f64);
                spinbox.set_h_size_flags(control::SizeFlags::EXPAND_FILL);
                spinbox.upcast()
            }
            VariantType::FLOAT => {
                let mut spinbox = SpinBox::new_alloc();
                spinbox.set_value(value.to::<f64>());
                spinbox.set_step(0.01);
                spinbox.set_h_size_flags(control::SizeFlags::EXPAND_FILL);
                spinbox.upcast()
            }
            VariantType::STRING => {
                let mut lineedit = LineEdit::new_alloc();
                lineedit.set_text(&value.to::<GString>());
                lineedit.set_h_size_flags(control::SizeFlags::EXPAND_FILL);
                lineedit.upcast()
            }
            VariantType::VECTOR2 => {
                let v = value.to::<Vector2>();
                Self::create_vector2_widget_static(v)
            }
            VariantType::VECTOR3 => {
                let v = value.to::<Vector3>();
                Self::create_vector3_widget_static(v)
            }
            VariantType::COLOR => {
                let c = value.to::<Color>();
                let mut picker = ColorPickerButton::new_alloc();
                picker.set_pick_color(c);
                picker.set_h_size_flags(control::SizeFlags::EXPAND_FILL);
                picker.upcast()
            }
            VariantType::DICTIONARY => {
                // Nested dictionary - show as expandable
                let dict = value.to::<VarDictionary>();
                let mut vbox = widgets::vbox();
                for (k, v) in dict.iter_shared() {
                    let mut row = widgets::hbox();
                    let mut label = Label::new_alloc();
                    label.set_text(&k.to::<GString>());
                    label.set_custom_minimum_size(Vector2::new(80.0, 0.0));
                    row.add_child(&label);
                    let widget = Self::create_widget_for_variant_static(&v);
                    row.add_child(&widget);
                    vbox.add_child(&row);
                }
                vbox.upcast()
            }
            _ => {
                // Fallback: display as string
                let mut label = Label::new_alloc();
                label.set_text(&format!("{:?}", value));
                label.add_theme_color_override("font_color", Color::from_rgb(0.7, 0.7, 0.7));
                label.upcast()
            }
        }
    }

    fn create_vector2_widget_static(v: Vector2) -> Gd<Control> {
        let mut hbox = widgets::hbox();

        for (label_text, value, color) in [
            ("X", v.x, Color::from_rgb(1.0, 0.4, 0.4)),
            ("Y", v.y, Color::from_rgb(0.4, 1.0, 0.4)),
        ] {
            let mut label = Label::new_alloc();
            label.set_text(label_text);
            label.add_theme_color_override("font_color", color);
            hbox.add_child(&label);

            let mut spin = SpinBox::new_alloc();
            spin.set_value(value as f64);
            spin.set_step(0.01);
            spin.set_h_size_flags(control::SizeFlags::EXPAND_FILL);
            spin.set_custom_minimum_size(Vector2::new(60.0, 0.0));
            hbox.add_child(&spin);
        }

        hbox.upcast()
    }

    fn create_vector3_widget_static(v: Vector3) -> Gd<Control> {
        let mut hbox = widgets::hbox();

        for (label_text, value, color) in [
            ("X", v.x, Color::from_rgb(1.0, 0.4, 0.4)),
            ("Y", v.y, Color::from_rgb(0.4, 1.0, 0.4)),
            ("Z", v.z, Color::from_rgb(0.4, 0.4, 1.0)),
        ] {
            let mut label = Label::new_alloc();
            label.set_text(label_text);
            label.add_theme_color_override("font_color", color);
            hbox.add_child(&label);

            let mut spin = SpinBox::new_alloc();
            spin.set_value(value as f64);
            spin.set_step(0.01);
            spin.set_h_size_flags(control::SizeFlags::EXPAND_FILL);
            spin.set_custom_minimum_size(Vector2::new(60.0, 0.0));
            hbox.add_child(&spin);
        }

        hbox.upcast()
    }
}

/// Helper for serializing component data to VarDictionary format.
pub struct ComponentDataSerializer;

impl ComponentDataSerializer {
    /// Serialize a reflected component to a VarDictionary.
    pub fn serialize(value: &dyn PartialReflect) -> VarDictionary {
        let mut dict = VarDictionary::new();

        match value.reflect_ref() {
            bevy_reflect::ReflectRef::Struct(s) => {
                for i in 0..s.field_len() {
                    if let (Some(name), Some(field)) = (s.name_at(i), s.field_at(i)) {
                        dict.set(GString::from(name), Self::value_to_variant(field));
                    }
                }
            }
            bevy_reflect::ReflectRef::TupleStruct(ts) => {
                for i in 0..ts.field_len() {
                    if let Some(field) = ts.field(i) {
                        dict.set(
                            GString::from(&format!("{}", i)[..]),
                            Self::value_to_variant(field),
                        );
                    }
                }
            }
            _ => {
                // For other types, just store a string representation
                dict.set("value", GString::from(&format!("{:?}", value)[..]));
            }
        }

        dict
    }

    /// Convert a reflected value to a Godot Variant.
    fn value_to_variant(value: &dyn PartialReflect) -> Variant {
        // Try to extract known types
        if let Some(reflect) = value.try_as_reflect() {
            use std::any::TypeId;
            let type_id = reflect.type_id();

            if type_id == TypeId::of::<bool>() {
                return Variant::from(*reflect.downcast_ref::<bool>().unwrap());
            }
            if type_id == TypeId::of::<f32>() {
                return Variant::from(*reflect.downcast_ref::<f32>().unwrap() as f64);
            }
            if type_id == TypeId::of::<f64>() {
                return Variant::from(*reflect.downcast_ref::<f64>().unwrap());
            }
            if type_id == TypeId::of::<i32>() {
                return Variant::from(*reflect.downcast_ref::<i32>().unwrap() as i64);
            }
            if type_id == TypeId::of::<i64>() {
                return Variant::from(*reflect.downcast_ref::<i64>().unwrap());
            }
            if type_id == TypeId::of::<u32>() {
                return Variant::from(*reflect.downcast_ref::<u32>().unwrap() as i64);
            }
            if type_id == TypeId::of::<u64>() {
                return Variant::from(*reflect.downcast_ref::<u64>().unwrap() as i64);
            }
            if type_id == TypeId::of::<String>() {
                return Variant::from(GString::from(
                    reflect.downcast_ref::<String>().unwrap().as_str(),
                ));
            }
            if type_id == TypeId::of::<bevy_math::Vec2>() {
                let v = reflect.downcast_ref::<bevy_math::Vec2>().unwrap();
                return Variant::from(Vector2::new(v.x, v.y));
            }
            if type_id == TypeId::of::<bevy_math::Vec3>() {
                let v = reflect.downcast_ref::<bevy_math::Vec3>().unwrap();
                return Variant::from(Vector3::new(v.x, v.y, v.z));
            }
        }

        // Fallback: string representation
        Variant::from(GString::from(&format!("{:?}", value)[..]))
    }
}
