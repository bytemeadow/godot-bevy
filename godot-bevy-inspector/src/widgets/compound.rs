//! Compound type widgets (vectors, colors, quaternions).

use bevy_math::{Quat, Vec2, Vec3, Vec4};
use godot::classes::{ColorPickerButton, Control, HBoxContainer, Label, VBoxContainer, control};
use godot::prelude::*;

use crate::options::{NumberOptions, QuatDisplay, QuatOptions};

use super::primitives::number_spinbox;

/// Create a widget for editing Vec2 values.
pub fn vec2_widget(value: Vec2, options: &NumberOptions<f32>) -> Gd<HBoxContainer> {
    let mut hbox = HBoxContainer::new_alloc();
    hbox.set_h_size_flags(control::SizeFlags::EXPAND_FILL);

    // X component
    let mut x_label = Label::new_alloc();
    x_label.set_text("X");
    x_label.add_theme_color_override("font_color", Color::from_rgb(1.0, 0.4, 0.4));
    hbox.add_child(&x_label);

    let x_spin = number_spinbox(value.x, options);
    hbox.add_child(&x_spin);

    // Y component
    let mut y_label = Label::new_alloc();
    y_label.set_text("Y");
    y_label.add_theme_color_override("font_color", Color::from_rgb(0.4, 1.0, 0.4));
    hbox.add_child(&y_label);

    let y_spin = number_spinbox(value.y, options);
    hbox.add_child(&y_spin);

    hbox
}

/// Create a widget for editing Vec3 values.
pub fn vec3_widget(value: Vec3, options: &NumberOptions<f32>) -> Gd<HBoxContainer> {
    let mut hbox = HBoxContainer::new_alloc();
    hbox.set_h_size_flags(control::SizeFlags::EXPAND_FILL);

    // X component
    let mut x_label = Label::new_alloc();
    x_label.set_text("X");
    x_label.add_theme_color_override("font_color", Color::from_rgb(1.0, 0.4, 0.4));
    hbox.add_child(&x_label);

    let x_spin = number_spinbox(value.x, options);
    hbox.add_child(&x_spin);

    // Y component
    let mut y_label = Label::new_alloc();
    y_label.set_text("Y");
    y_label.add_theme_color_override("font_color", Color::from_rgb(0.4, 1.0, 0.4));
    hbox.add_child(&y_label);

    let y_spin = number_spinbox(value.y, options);
    hbox.add_child(&y_spin);

    // Z component
    let mut z_label = Label::new_alloc();
    z_label.set_text("Z");
    z_label.add_theme_color_override("font_color", Color::from_rgb(0.4, 0.4, 1.0));
    hbox.add_child(&z_label);

    let z_spin = number_spinbox(value.z, options);
    hbox.add_child(&z_spin);

    hbox
}

/// Create a widget for editing Vec4 values.
pub fn vec4_widget(value: Vec4, options: &NumberOptions<f32>) -> Gd<HBoxContainer> {
    let mut hbox = HBoxContainer::new_alloc();
    hbox.set_h_size_flags(control::SizeFlags::EXPAND_FILL);

    let labels = ["X", "Y", "Z", "W"];
    let colors = [
        Color::from_rgb(1.0, 0.4, 0.4),
        Color::from_rgb(0.4, 1.0, 0.4),
        Color::from_rgb(0.4, 0.4, 1.0),
        Color::from_rgb(1.0, 1.0, 0.4),
    ];
    let values = [value.x, value.y, value.z, value.w];

    for i in 0..4 {
        let mut label = Label::new_alloc();
        label.set_text(labels[i]);
        label.add_theme_color_override("font_color", colors[i]);
        hbox.add_child(&label);

        let spin = number_spinbox(values[i], options);
        hbox.add_child(&spin);
    }

    hbox
}

/// Create a widget for editing Quat values based on display options.
pub fn quat_widget(value: Quat, options: &QuatOptions) -> Gd<Control> {
    match options.display {
        QuatDisplay::Raw => quat_raw_widget(value).upcast(),
        QuatDisplay::Euler => quat_euler_widget(value).upcast(),
        QuatDisplay::YawPitchRoll => quat_ypr_widget(value).upcast(),
        QuatDisplay::AxisAngle => quat_axis_angle_widget(value).upcast(),
    }
}

/// Raw quaternion display (x, y, z, w).
fn quat_raw_widget(value: Quat) -> Gd<HBoxContainer> {
    let vec4 = Vec4::new(value.x, value.y, value.z, value.w);
    vec4_widget(vec4, &NumberOptions::default())
}

/// Euler angles display (pitch, yaw, roll in degrees).
fn quat_euler_widget(value: Quat) -> Gd<HBoxContainer> {
    let euler = value.to_euler(bevy_math::EulerRot::XYZ);
    let degrees = Vec3::new(
        euler.0.to_degrees(),
        euler.1.to_degrees(),
        euler.2.to_degrees(),
    );

    let mut hbox = HBoxContainer::new_alloc();
    hbox.set_h_size_flags(control::SizeFlags::EXPAND_FILL);

    let labels = ["Pitch", "Yaw", "Roll"];
    let values = [degrees.x, degrees.y, degrees.z];
    let options = NumberOptions::<f32>::between(-180.0, 180.0).with_step(0.1);

    for i in 0..3 {
        let mut label = Label::new_alloc();
        label.set_text(labels[i]);
        hbox.add_child(&label);

        let spin = number_spinbox(values[i], &options);
        hbox.add_child(&spin);
    }

    hbox
}

/// Yaw-Pitch-Roll display (more intuitive ordering).
fn quat_ypr_widget(value: Quat) -> Gd<HBoxContainer> {
    let euler = value.to_euler(bevy_math::EulerRot::YXZ);
    let degrees = Vec3::new(
        euler.0.to_degrees(), // Yaw
        euler.1.to_degrees(), // Pitch
        euler.2.to_degrees(), // Roll
    );

    let mut hbox = HBoxContainer::new_alloc();
    hbox.set_h_size_flags(control::SizeFlags::EXPAND_FILL);

    let labels = ["Yaw", "Pitch", "Roll"];
    let values = [degrees.x, degrees.y, degrees.z];
    let options = NumberOptions::<f32>::between(-180.0, 180.0).with_step(0.1);

    for i in 0..3 {
        let mut label = Label::new_alloc();
        label.set_text(labels[i]);
        hbox.add_child(&label);

        let spin = number_spinbox(values[i], &options);
        hbox.add_child(&spin);
    }

    hbox
}

/// Axis-Angle display.
fn quat_axis_angle_widget(value: Quat) -> Gd<VBoxContainer> {
    let (axis, angle) = value.to_axis_angle();

    let mut vbox = VBoxContainer::new_alloc();
    vbox.set_h_size_flags(control::SizeFlags::EXPAND_FILL);

    // Axis row
    let mut axis_hbox = HBoxContainer::new_alloc();
    let mut axis_label = Label::new_alloc();
    axis_label.set_text("Axis");
    axis_label.set_custom_minimum_size(Vector2::new(50.0, 0.0));
    axis_hbox.add_child(&axis_label);

    let axis_widget = vec3_widget(
        axis,
        &NumberOptions::<f32>::between(-1.0, 1.0).with_step(0.01),
    );
    axis_hbox.add_child(&axis_widget);
    vbox.add_child(&axis_hbox);

    // Angle row
    let mut angle_hbox = HBoxContainer::new_alloc();
    let mut angle_label = Label::new_alloc();
    angle_label.set_text("Angle");
    angle_label.set_custom_minimum_size(Vector2::new(50.0, 0.0));
    angle_hbox.add_child(&angle_label);

    let angle_spin = number_spinbox(
        angle.to_degrees(),
        &NumberOptions::<f32>::between(-180.0, 180.0)
            .with_step(0.1)
            .with_suffix("°"),
    );
    angle_hbox.add_child(&angle_spin);
    vbox.add_child(&angle_hbox);

    vbox
}

/// Create a color picker button for editing Color values.
pub fn color_picker(value: godot::prelude::Color) -> Gd<ColorPickerButton> {
    let mut picker = ColorPickerButton::new_alloc();
    picker.set_pick_color(value);
    picker.set_h_size_flags(control::SizeFlags::EXPAND_FILL);
    picker.set_custom_minimum_size(Vector2::new(60.0, 0.0));
    picker
}

/// Create a color picker for Bevy's Color type.
/// Note: This converts to/from Godot's Color format.
pub fn bevy_color_picker(r: f32, g: f32, b: f32, a: f32) -> Gd<ColorPickerButton> {
    let godot_color = Color::from_rgba(r, g, b, a);
    color_picker(godot_color)
}
