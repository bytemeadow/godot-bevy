//! Primitive type widgets (numbers, strings, bools).

use godot::classes::{CheckBox, HSlider, Label, LineEdit, SpinBox, TextEdit, control};
use godot::prelude::*;

use crate::options::{NumberDisplay, NumberOptions};

/// Create a SpinBox for editing numeric values.
pub fn number_spinbox<T: NumericValue>(value: T, options: &NumberOptions<T>) -> Gd<SpinBox> {
    let mut spinbox = SpinBox::new_alloc();

    // Set range
    if let Some(min) = &options.min {
        spinbox.set_min(min.to_f64());
    } else {
        spinbox.set_min(T::MIN_VALUE);
    }

    if let Some(max) = &options.max {
        spinbox.set_max(max.to_f64());
    } else {
        spinbox.set_max(T::MAX_VALUE);
    }

    // Set step
    spinbox.set_step(options.step);

    // Set current value
    spinbox.set_value(value.to_f64());

    // Set prefix/suffix
    if !options.prefix.is_empty() {
        spinbox.set_prefix(&options.prefix);
    }
    if !options.suffix.is_empty() {
        spinbox.set_suffix(&options.suffix);
    }

    // Size settings
    spinbox.set_h_size_flags(control::SizeFlags::EXPAND_FILL);
    spinbox.set_custom_minimum_size(Vector2::new(80.0, 0.0));

    spinbox
}

/// Create an HSlider for editing numeric values.
pub fn number_slider<T: NumericValue>(value: T, options: &NumberOptions<T>) -> Gd<HSlider> {
    let mut slider = HSlider::new_alloc();

    // Sliders require both min and max
    let min = options.min.as_ref().map(|v| v.to_f64()).unwrap_or(0.0);
    let max = options.max.as_ref().map(|v| v.to_f64()).unwrap_or(100.0);

    slider.set_min(min);
    slider.set_max(max);
    slider.set_step(options.step);
    slider.set_value(value.to_f64());

    // Size settings
    slider.set_h_size_flags(control::SizeFlags::EXPAND_FILL);
    slider.set_custom_minimum_size(Vector2::new(100.0, 0.0));

    slider
}

/// Create a widget for editing a numeric value based on options.
pub fn number_widget<T: NumericValue>(
    value: T,
    options: &NumberOptions<T>,
) -> Gd<godot::classes::Control> {
    match options.display {
        NumberDisplay::SpinBox => number_spinbox(value, options).upcast(),
        NumberDisplay::Slider => number_slider(value, options).upcast(),
    }
}

/// Create a CheckBox for editing bool values.
pub fn bool_checkbox(value: bool) -> Gd<CheckBox> {
    let mut checkbox = CheckBox::new_alloc();
    checkbox.set_pressed(value);
    checkbox
}

/// Create a LineEdit for editing string values.
pub fn string_lineedit(value: &str) -> Gd<LineEdit> {
    let mut lineedit = LineEdit::new_alloc();
    lineedit.set_text(value);
    lineedit.set_h_size_flags(control::SizeFlags::EXPAND_FILL);
    lineedit
}

/// Create a TextEdit for editing multi-line string values.
pub fn string_textedit(value: &str) -> Gd<TextEdit> {
    let mut textedit = TextEdit::new_alloc();
    textedit.set_text(value);
    textedit.set_h_size_flags(control::SizeFlags::EXPAND_FILL);
    textedit.set_custom_minimum_size(Vector2::new(0.0, 60.0));
    textedit
}

/// Create a read-only label for displaying values.
pub fn readonly_label(text: &str) -> Gd<Label> {
    let mut label = Label::new_alloc();
    label.set_text(text);
    label.set_h_size_flags(control::SizeFlags::EXPAND_FILL);
    label
}

/// Trait for numeric values that can be displayed in widgets.
pub trait NumericValue: Clone + Send + Sync + 'static {
    /// The minimum representable value.
    const MIN_VALUE: f64;
    /// The maximum representable value.
    const MAX_VALUE: f64;

    /// Convert to f64 for widget display.
    fn to_f64(&self) -> f64;
    /// Convert from f64 after widget edit.
    fn from_f64(value: f64) -> Self;
}

macro_rules! impl_numeric_value {
    ($($ty:ty),*) => {
        $(
            impl NumericValue for $ty {
                const MIN_VALUE: f64 = <$ty>::MIN as f64;
                const MAX_VALUE: f64 = <$ty>::MAX as f64;

                fn to_f64(&self) -> f64 {
                    *self as f64
                }

                fn from_f64(value: f64) -> Self {
                    value as $ty
                }
            }
        )*
    };
}

impl_numeric_value!(f32, f64, i8, i16, i32, i64, isize, u8, u16, u32, u64, usize);

// Special handling for i128/u128 since they can overflow f64
impl NumericValue for i128 {
    const MIN_VALUE: f64 = i64::MIN as f64; // Clamp to f64 representable range
    const MAX_VALUE: f64 = i64::MAX as f64;

    fn to_f64(&self) -> f64 {
        *self as f64
    }

    fn from_f64(value: f64) -> Self {
        value as i128
    }
}

impl NumericValue for u128 {
    const MIN_VALUE: f64 = 0.0;
    const MAX_VALUE: f64 = u64::MAX as f64;

    fn to_f64(&self) -> f64 {
        *self as f64
    }

    fn from_f64(value: f64) -> Self {
        value as u128
    }
}
