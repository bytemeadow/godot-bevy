//! Inspector options for customizing how values are displayed and edited.
//!
//! This module provides the [`InspectorOptions`] type which allows associating
//! display/edit options with struct fields, enum variants, etc.
//!
//! The design is adapted from bevy-inspector-egui's options system to be UI-agnostic.

use bevy_reflect::{FromType, TypeData};
use std::{any::Any, collections::HashMap};

mod number;
mod quat;

pub use number::{NumberDisplay, NumberOptions};
pub use quat::{QuatDisplay, QuatOptions};

/// Descriptor of a path into a struct/enum.
///
/// Used as a key in [`InspectorOptions`] to associate options with specific fields.
#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug)]
#[non_exhaustive]
pub enum Target {
    /// A field in a struct or tuple struct, by index
    Field(usize),
    /// A field within a specific enum variant
    VariantField {
        variant_index: usize,
        field_index: usize,
    },
}

/// Map of [`Target`]s to arbitrary [`TypeData`] used to control how values are displayed.
///
/// # Example
///
/// ```rust,ignore
/// use bevy_reflect::Reflect;
/// use godot_bevy_inspector::prelude::*;
///
/// #[derive(Reflect, Default)]
/// struct Config {
///     // Use InspectorOptions to customize display
///     font_size: f32,  // Could use NumberOptions { min: 10.0, max: 70.0, .. }
/// }
/// ```
#[derive(Default)]
pub struct InspectorOptions {
    options: HashMap<Target, Box<dyn TypeData>>,
}

impl std::fmt::Debug for InspectorOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut options = f.debug_struct("InspectorOptions");
        for entry in self.options.keys() {
            options.field(&format!("{entry:?}"), &"..");
        }
        options.finish()
    }
}

impl Clone for InspectorOptions {
    fn clone(&self) -> Self {
        Self {
            options: self
                .options
                .iter()
                .map(|(target, data)| (*target, TypeData::clone_type_data(&**data)))
                .collect(),
        }
    }
}

impl InspectorOptions {
    /// Create a new empty options map.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert options for a specific target.
    pub fn insert<T: TypeData>(&mut self, target: Target, options: T) {
        self.options.insert(target, Box::new(options));
    }

    /// Insert boxed options for a specific target.
    pub fn insert_boxed(&mut self, target: Target, options: Box<dyn TypeData>) {
        self.options.insert(target, options);
    }

    /// Get options for a specific target.
    pub fn get(&self, target: Target) -> Option<&dyn Any> {
        self.options.get(&target).map(|value| value.as_any())
    }

    /// Iterate over all target-options pairs.
    pub fn iter(&self) -> impl Iterator<Item = (Target, &dyn TypeData)> + '_ {
        self.options.iter().map(|(target, data)| (*target, &**data))
    }
}

/// Wrapper of [`InspectorOptions`] to be stored in the [`TypeRegistry`](bevy_reflect::TypeRegistry).
#[derive(Clone)]
pub struct ReflectInspectorOptions(pub InspectorOptions);

impl<T> FromType<T> for ReflectInspectorOptions
where
    InspectorOptions: FromType<T>,
{
    fn from_type() -> Self {
        ReflectInspectorOptions(InspectorOptions::from_type())
    }
}

/// Helper trait for deriving inspector options.
///
/// Types implementing this trait can have their options automatically generated
/// from derive macro attributes.
pub trait InspectorOptionsType {
    /// The options type used during derive macro expansion.
    type DeriveOptions: Default;
    /// The actual options type stored in the registry.
    type Options: TypeData + Clone;

    /// Convert derive-time options to runtime options.
    fn options_from_derive(options: Self::DeriveOptions) -> Self::Options;
}

/// Get options for a struct field from the parent options.
pub fn options_for_field(options: &dyn Any, field_index: usize) -> &dyn Any {
    options
        .downcast_ref::<InspectorOptions>()
        .and_then(|opts| opts.get(Target::Field(field_index)))
        .unwrap_or(&())
}

/// Get options for an enum variant field from the parent options.
pub fn options_for_variant_field(
    options: &dyn Any,
    variant_index: usize,
    field_index: usize,
) -> &dyn Any {
    options
        .downcast_ref::<InspectorOptions>()
        .and_then(|opts| {
            opts.get(Target::VariantField {
                variant_index,
                field_index,
            })
        })
        .unwrap_or(&())
}
