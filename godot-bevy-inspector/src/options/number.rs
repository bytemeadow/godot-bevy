//! Options for numeric value display and editing.

use super::InspectorOptionsType;

/// Options for how numeric values are displayed and edited.
#[derive(Clone)]
#[non_exhaustive]
pub struct NumberOptions<T> {
    /// Minimum allowed value.
    pub min: Option<T>,
    /// Maximum allowed value.
    pub max: Option<T>,
    /// Step size for drag/scroll operations.
    pub step: f64,
    /// Prefix to display before the value (e.g., "$").
    pub prefix: String,
    /// Suffix to display after the value (e.g., "px").
    pub suffix: String,
    /// How to display the number input.
    pub display: NumberDisplay,
}

impl<T> Default for NumberOptions<T> {
    fn default() -> Self {
        Self {
            min: None,
            max: None,
            step: 1.0,
            prefix: String::new(),
            suffix: String::new(),
            display: NumberDisplay::default(),
        }
    }
}

/// How to display a numeric input.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum NumberDisplay {
    /// A draggable number field (SpinBox in Godot).
    #[default]
    SpinBox,
    /// A slider control (HSlider in Godot).
    Slider,
}

impl<T> NumberOptions<T> {
    /// Create options with min and max bounds.
    pub fn between(min: T, max: T) -> Self {
        Self {
            min: Some(min),
            max: Some(max),
            ..Default::default()
        }
    }

    /// Create options with only a minimum bound.
    pub fn at_least(min: T) -> Self {
        Self {
            min: Some(min),
            max: None,
            ..Default::default()
        }
    }

    /// Set the step size.
    pub fn with_step(mut self, step: f64) -> Self {
        self.step = step;
        self
    }

    /// Set the display mode.
    pub fn with_display(mut self, display: NumberDisplay) -> Self {
        self.display = display;
        self
    }

    /// Set a suffix.
    pub fn with_suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = suffix.into();
        self
    }

    /// Set a prefix.
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
        self
    }

    /// Map the bounds to a different type.
    pub fn map<U>(&self, f: impl Fn(&T) -> U) -> NumberOptions<U> {
        NumberOptions {
            min: self.min.as_ref().map(&f),
            max: self.max.as_ref().map(&f),
            step: self.step,
            prefix: self.prefix.clone(),
            suffix: self.suffix.clone(),
            display: self.display,
        }
    }
}

impl NumberOptions<f32> {
    /// Options for a normalized value (0.0 to 1.0).
    pub fn normalized() -> Self {
        Self {
            min: Some(0.0),
            max: Some(1.0),
            step: 0.01,
            ..Default::default()
        }
    }

    /// Options for positive values only.
    pub fn positive() -> Self {
        Self {
            min: Some(0.0),
            max: None,
            ..Default::default()
        }
    }
}

impl NumberOptions<f64> {
    /// Options for a normalized value (0.0 to 1.0).
    pub fn normalized() -> Self {
        Self {
            min: Some(0.0),
            max: Some(1.0),
            step: 0.01,
            ..Default::default()
        }
    }

    /// Options for positive values only.
    pub fn positive() -> Self {
        Self {
            min: Some(0.0),
            max: None,
            ..Default::default()
        }
    }
}

// Implement InspectorOptionsType for numeric types
macro_rules! impl_number_options {
    ($($ty:ty),*) => {
        $(
            impl InspectorOptionsType for $ty {
                type DeriveOptions = NumberOptions<$ty>;
                type Options = NumberOptions<$ty>;

                fn options_from_derive(options: Self::DeriveOptions) -> Self::Options {
                    options
                }
            }
        )*
    };
}

impl_number_options!(
    f32, f64, i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize
);
