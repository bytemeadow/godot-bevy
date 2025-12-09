//! Core inspector UI generation from Bevy reflection.

use std::any::{Any, TypeId};

use bevy_ecs::world::World;
use bevy_reflect::{
    Array, Enum, List, Map, PartialReflect, ReflectRef, Set, Struct, Tuple, TupleStruct,
    TypeRegistry,
};
use godot::classes::{Control, Label, OptionButton, control};
use godot::prelude::*;

use crate::options::{NumberOptions, QuatOptions, ReflectInspectorOptions, options_for_field};
use crate::widgets::{self, CollapsibleSection};

/// Context for inspector UI generation.
///
/// Provides access to the Bevy world and other resources needed during UI generation.
#[derive(Default)]
pub struct Context<'a> {
    /// Optional reference to the Bevy world for looking up related data.
    pub world: Option<&'a World>,
}

/// Main struct for generating inspector UI from reflected values.
pub struct InspectorUi<'a, 'c> {
    /// Reference to the type registry for looking up type information.
    pub type_registry: &'a TypeRegistry,
    /// Context with world access.
    pub context: &'a mut Context<'c>,
    /// Counter for generating unique IDs.
    #[allow(dead_code)]
    id_counter: u32,
}

impl<'a, 'c> InspectorUi<'a, 'c> {
    /// Create a new InspectorUi.
    pub fn new(type_registry: &'a TypeRegistry, context: &'a mut Context<'c>) -> Self {
        Self {
            type_registry,
            context,
            id_counter: 0,
        }
    }

    /// Generate a unique ID for a widget.
    #[allow(dead_code)]
    fn next_id(&mut self) -> u32 {
        self.id_counter += 1;
        self.id_counter
    }

    /// Create UI for a reflected value, returning the root control.
    pub fn ui_for_reflect(&mut self, value: &dyn PartialReflect) -> Gd<Control> {
        self.ui_for_reflect_with_options(value, &())
    }

    /// Create UI for a reflected value with options.
    pub fn ui_for_reflect_with_options(
        &mut self,
        value: &dyn PartialReflect,
        options: &dyn Any,
    ) -> Gd<Control> {
        // Check if we have registered options in the type registry
        let options = self.get_options_or_default(value, options);

        // Try to handle as a known type first
        if let Some(widget) = self.try_primitive_widget(value, options) {
            return widget;
        }

        // Fall back to reflection-based traversal
        match value.reflect_ref() {
            ReflectRef::Struct(s) => self.ui_for_struct(s, options),
            ReflectRef::TupleStruct(ts) => self.ui_for_tuple_struct(ts, options),
            ReflectRef::Tuple(t) => self.ui_for_tuple(t, options),
            ReflectRef::List(l) => self.ui_for_list(l, options),
            ReflectRef::Array(a) => self.ui_for_array(a, options),
            ReflectRef::Map(m) => self.ui_for_map(m, options),
            ReflectRef::Enum(e) => self.ui_for_enum(e, options),
            ReflectRef::Set(s) => self.ui_for_set(s, options),
            ReflectRef::Opaque(_) => self.ui_for_opaque(value),
        }
    }

    /// Get options from type registry or use provided options.
    fn get_options_or_default<'b>(
        &self,
        value: &dyn PartialReflect,
        options: &'b dyn Any,
    ) -> &'b dyn Any {
        if !options.is::<()>() {
            return options;
        }

        // Try to get options from type registry
        if let Some(reflect) = value.try_as_reflect() {
            if let Some(_data) = self
                .type_registry
                .get_type_data::<ReflectInspectorOptions>(reflect.type_id())
            {
                // Note: We can't return a reference to data here due to lifetimes,
                // so we just return the unit type and handle this case specially
            }
        }

        options
    }

    /// Try to create a widget for a primitive type.
    fn try_primitive_widget(
        &mut self,
        value: &dyn PartialReflect,
        options: &dyn Any,
    ) -> Option<Gd<Control>> {
        let reflect = value.try_as_reflect()?;
        let type_id = reflect.type_id();

        // Numbers
        if type_id == TypeId::of::<f32>() {
            let v = reflect.downcast_ref::<f32>().unwrap();
            let opts = options
                .downcast_ref::<NumberOptions<f32>>()
                .cloned()
                .unwrap_or_default();
            return Some(widgets::number_spinbox(*v, &opts).upcast());
        }
        if type_id == TypeId::of::<f64>() {
            let v = reflect.downcast_ref::<f64>().unwrap();
            let opts = options
                .downcast_ref::<NumberOptions<f64>>()
                .cloned()
                .unwrap_or_default();
            return Some(widgets::number_spinbox(*v, &opts).upcast());
        }
        if type_id == TypeId::of::<i32>() {
            let v = reflect.downcast_ref::<i32>().unwrap();
            let opts = options
                .downcast_ref::<NumberOptions<i32>>()
                .cloned()
                .unwrap_or_default();
            return Some(widgets::number_spinbox(*v, &opts).upcast());
        }
        if type_id == TypeId::of::<i64>() {
            let v = reflect.downcast_ref::<i64>().unwrap();
            let opts = options
                .downcast_ref::<NumberOptions<i64>>()
                .cloned()
                .unwrap_or_default();
            return Some(widgets::number_spinbox(*v, &opts).upcast());
        }
        if type_id == TypeId::of::<u32>() {
            let v = reflect.downcast_ref::<u32>().unwrap();
            let opts = options
                .downcast_ref::<NumberOptions<u32>>()
                .cloned()
                .unwrap_or_default();
            return Some(widgets::number_spinbox(*v, &opts).upcast());
        }
        if type_id == TypeId::of::<u64>() {
            let v = reflect.downcast_ref::<u64>().unwrap();
            let opts = options
                .downcast_ref::<NumberOptions<u64>>()
                .cloned()
                .unwrap_or_default();
            return Some(widgets::number_spinbox(*v, &opts).upcast());
        }
        if type_id == TypeId::of::<usize>() {
            let v = reflect.downcast_ref::<usize>().unwrap();
            let opts = options
                .downcast_ref::<NumberOptions<usize>>()
                .cloned()
                .unwrap_or_default();
            return Some(widgets::number_spinbox(*v, &opts).upcast());
        }

        // Bool
        if type_id == TypeId::of::<bool>() {
            let v = reflect.downcast_ref::<bool>().unwrap();
            return Some(widgets::bool_checkbox(*v).upcast());
        }

        // String
        if type_id == TypeId::of::<String>() {
            let v = reflect.downcast_ref::<String>().unwrap();
            return Some(widgets::string_lineedit(v).upcast());
        }

        // Vectors
        if type_id == TypeId::of::<bevy_math::Vec2>() {
            let v = reflect.downcast_ref::<bevy_math::Vec2>().unwrap();
            let opts = options
                .downcast_ref::<NumberOptions<f32>>()
                .cloned()
                .unwrap_or_default();
            return Some(widgets::vec2_widget(*v, &opts).upcast());
        }
        if type_id == TypeId::of::<bevy_math::Vec3>() {
            let v = reflect.downcast_ref::<bevy_math::Vec3>().unwrap();
            let opts = options
                .downcast_ref::<NumberOptions<f32>>()
                .cloned()
                .unwrap_or_default();
            return Some(widgets::vec3_widget(*v, &opts).upcast());
        }
        if type_id == TypeId::of::<bevy_math::Vec4>() {
            let v = reflect.downcast_ref::<bevy_math::Vec4>().unwrap();
            let opts = options
                .downcast_ref::<NumberOptions<f32>>()
                .cloned()
                .unwrap_or_default();
            return Some(widgets::vec4_widget(*v, &opts).upcast());
        }

        // Quaternion
        if type_id == TypeId::of::<bevy_math::Quat>() {
            let v = reflect.downcast_ref::<bevy_math::Quat>().unwrap();
            let opts = options
                .downcast_ref::<QuatOptions>()
                .cloned()
                .unwrap_or_default();
            return Some(widgets::quat_widget(*v, &opts));
        }

        None
    }

    /// Create UI for a struct.
    fn ui_for_struct(&mut self, value: &dyn Struct, options: &dyn Any) -> Gd<Control> {
        let mut vbox = widgets::vbox();

        let type_name = value.reflect_short_type_path();
        let mut _section = CollapsibleSection::create(GString::from(type_name), true);

        // We need to defer adding children until after ready() is called
        // For now, just create a simple vbox
        let mut content = widgets::vbox();

        for i in 0..value.field_len() {
            if let Some(field) = value.field_at(i) {
                let field_name = value.name_at(i).unwrap_or("?");
                let field_options = options_for_field(options, i);

                let mut row = widgets::hbox();

                // Field label
                let mut label = Label::new_alloc();
                label.set_text(field_name);
                label.set_custom_minimum_size(Vector2::new(120.0, 0.0));
                row.add_child(&label);

                // Field widget
                let widget = self.ui_for_reflect_with_options(field, field_options);
                row.add_child(&widget);

                content.add_child(&row);
            }
        }

        vbox.add_child(&content);
        vbox.upcast()
    }

    /// Create UI for a tuple struct.
    fn ui_for_tuple_struct(&mut self, value: &dyn TupleStruct, options: &dyn Any) -> Gd<Control> {
        let mut vbox = widgets::vbox();

        for i in 0..value.field_len() {
            if let Some(field) = value.field(i) {
                let field_options = options_for_field(options, i);

                let mut row = widgets::hbox();

                // Index label
                let mut label = Label::new_alloc();
                label.set_text(&format!("{}", i));
                label.set_custom_minimum_size(Vector2::new(30.0, 0.0));
                row.add_child(&label);

                // Field widget
                let widget = self.ui_for_reflect_with_options(field, field_options);
                row.add_child(&widget);

                vbox.add_child(&row);
            }
        }

        vbox.upcast()
    }

    /// Create UI for a tuple.
    fn ui_for_tuple(&mut self, value: &dyn Tuple, options: &dyn Any) -> Gd<Control> {
        let mut hbox = widgets::hbox();

        for i in 0..value.field_len() {
            if let Some(field) = value.field(i) {
                let field_options = options_for_field(options, i);
                let widget = self.ui_for_reflect_with_options(field, field_options);
                hbox.add_child(&widget);
            }
        }

        hbox.upcast()
    }

    /// Create UI for a list.
    fn ui_for_list(&mut self, value: &dyn List, options: &dyn Any) -> Gd<Control> {
        let mut vbox = widgets::vbox();

        if value.len() == 0 {
            let mut label = Label::new_alloc();
            label.set_text("(empty list)");
            label.add_theme_color_override("font_color", Color::from_rgb(0.6, 0.6, 0.6));
            vbox.add_child(&label);
        } else {
            for i in 0..value.len() {
                if let Some(item) = value.get(i) {
                    let mut row = widgets::hbox();

                    // Index label
                    let mut label = Label::new_alloc();
                    label.set_text(&format!("[{}]", i));
                    label.set_custom_minimum_size(Vector2::new(40.0, 0.0));
                    row.add_child(&label);

                    // Item widget
                    let widget = self.ui_for_reflect_with_options(item, options);
                    row.add_child(&widget);

                    vbox.add_child(&row);
                }
            }
        }

        vbox.upcast()
    }

    /// Create UI for an array.
    fn ui_for_array(&mut self, value: &dyn Array, options: &dyn Any) -> Gd<Control> {
        let mut vbox = widgets::vbox();

        for i in 0..value.len() {
            if let Some(item) = value.get(i) {
                let mut row = widgets::hbox();

                // Index label
                let mut label = Label::new_alloc();
                label.set_text(&format!("[{}]", i));
                label.set_custom_minimum_size(Vector2::new(40.0, 0.0));
                row.add_child(&label);

                // Item widget
                let widget = self.ui_for_reflect_with_options(item, options);
                row.add_child(&widget);

                vbox.add_child(&row);
            }
        }

        vbox.upcast()
    }

    /// Create UI for a map.
    fn ui_for_map(&mut self, value: &dyn Map, options: &dyn Any) -> Gd<Control> {
        let mut vbox = widgets::vbox();

        if value.len() == 0 {
            let mut label = Label::new_alloc();
            label.set_text("(empty map)");
            label.add_theme_color_override("font_color", Color::from_rgb(0.6, 0.6, 0.6));
            vbox.add_child(&label);
        } else {
            for (key, val) in value.iter() {
                let mut row = widgets::hbox();

                // Key widget
                let key_widget = self.ui_for_reflect(key);
                row.add_child(&key_widget);

                // Arrow separator
                let mut arrow = Label::new_alloc();
                arrow.set_text("→");
                row.add_child(&arrow);

                // Value widget
                let val_widget = self.ui_for_reflect_with_options(val, options);
                row.add_child(&val_widget);

                vbox.add_child(&row);
            }
        }

        vbox.upcast()
    }

    /// Create UI for an enum.
    fn ui_for_enum(&mut self, value: &dyn Enum, _options: &dyn Any) -> Gd<Control> {
        let mut vbox = widgets::vbox();

        // Variant selector
        let mut hbox = widgets::hbox();
        let mut label = Label::new_alloc();
        label.set_text("Variant");
        label.set_custom_minimum_size(Vector2::new(80.0, 0.0));
        hbox.add_child(&label);

        let mut option_button = OptionButton::new_alloc();
        option_button.set_h_size_flags(control::SizeFlags::EXPAND_FILL);

        // Add all variants (we'd need TypeInfo for this, for now just show current)
        let variant_name = value.variant_name();
        option_button.add_item(variant_name);
        option_button.select(0);

        hbox.add_child(&option_button);
        vbox.add_child(&hbox);

        // Variant fields
        let field_len = value.field_len();
        if field_len > 0 {
            let mut fields_container = widgets::vbox();

            for i in 0..field_len {
                if let Some(field) = value.field_at(i) {
                    let mut row = widgets::hbox();

                    // Field name/index
                    let field_label = if let Some(name) = value.name_at(i) {
                        name.to_string()
                    } else {
                        format!("{}", i)
                    };
                    let mut label = Label::new_alloc();
                    label.set_text(&field_label);
                    label.set_custom_minimum_size(Vector2::new(80.0, 0.0));
                    row.add_child(&label);

                    // Field widget
                    let widget = self.ui_for_reflect(field);
                    row.add_child(&widget);

                    fields_container.add_child(&row);
                }
            }

            vbox.add_child(&fields_container);
        }

        vbox.upcast()
    }

    /// Create UI for a set.
    fn ui_for_set(&mut self, value: &dyn Set, _options: &dyn Any) -> Gd<Control> {
        let mut vbox = widgets::vbox();

        if value.len() == 0 {
            let mut label = Label::new_alloc();
            label.set_text("(empty set)");
            label.add_theme_color_override("font_color", Color::from_rgb(0.6, 0.6, 0.6));
            vbox.add_child(&label);
        } else {
            for item in value.iter() {
                let widget = self.ui_for_reflect(item);
                vbox.add_child(&widget);
            }
        }

        vbox.upcast()
    }

    /// Create UI for an opaque value (no reflection available).
    fn ui_for_opaque(&mut self, value: &dyn PartialReflect) -> Gd<Control> {
        let mut label = Label::new_alloc();
        let type_name = value.reflect_short_type_path();
        label.set_text(&format!("<{}>", type_name));
        label.add_theme_color_override("font_color", Color::from_rgb(0.8, 0.5, 0.5));
        label.upcast()
    }
}
