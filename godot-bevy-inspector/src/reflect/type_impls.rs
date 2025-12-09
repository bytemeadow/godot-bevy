//! Type-specific implementations for the inspector.
//!
//! This module contains registration logic for known types.

use bevy_reflect::TypeRegistry;

/// Register default inspector implementations for common types.
#[allow(dead_code)]
pub fn register_default_types(_registry: &mut TypeRegistry) {
    // Type registrations for inspector options will be added here
    // For now, the primitive handling is done inline in InspectorUi
}
