//! Godot Control-based widgets for the inspector.
//!
//! This module provides widget implementations that use native Godot UI controls
//! to display and edit values.

mod compound;
mod containers;
mod primitives;

pub use compound::*;
pub use containers::*;
pub use primitives::*;

use godot::classes::{Button, Control, HBoxContainer, Label, VBoxContainer, control};
use godot::global::HorizontalAlignment;
use godot::prelude::*;

/// Result type for widget value changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetChange {
    /// The value was not changed.
    Unchanged,
    /// The value was changed.
    Changed,
}

impl WidgetChange {
    /// Returns true if the value changed.
    pub fn is_changed(self) -> bool {
        matches!(self, WidgetChange::Changed)
    }
}

impl From<bool> for WidgetChange {
    fn from(changed: bool) -> Self {
        if changed {
            WidgetChange::Changed
        } else {
            WidgetChange::Unchanged
        }
    }
}

/// Trait for widgets that can be created to edit values.
pub trait InspectorWidget {
    /// The Godot control type for this widget.
    type Control: Inherits<Control>;

    /// Create a new widget instance.
    fn create() -> Gd<Self::Control>;
}

/// Helper to create a horizontal container with a label and widget.
pub fn labeled_widget<C: Inherits<Control>>(label: &str, widget: Gd<C>) -> Gd<HBoxContainer> {
    let mut hbox = HBoxContainer::new_alloc();

    let mut label_node = Label::new_alloc();
    label_node.set_text(label);
    label_node.set_h_size_flags(control::SizeFlags::EXPAND_FILL);
    label_node.set_custom_minimum_size(Vector2::new(120.0, 0.0));

    hbox.add_child(&label_node);
    hbox.add_child(&widget.upcast::<Control>());

    hbox
}

/// Helper to create a section with a collapsible header.
pub fn collapsible_section(
    title: &str,
    initially_open: bool,
) -> (Gd<VBoxContainer>, Gd<VBoxContainer>) {
    let mut outer = VBoxContainer::new_alloc();

    // Create header button
    let mut header = Button::new_alloc();
    header.set_text(&format!(
        "{} {}",
        if initially_open { "▼" } else { "▶" },
        title
    ));
    header.set_flat(true);
    header.set_text_alignment(HorizontalAlignment::LEFT);
    outer.add_child(&header);

    // Create content container
    let mut content = VBoxContainer::new_alloc();
    content.set_visible(initially_open);
    outer.add_child(&content.clone());

    // We'll connect the signal in the calling code since we need the content reference
    // The caller should connect header.pressed to toggle content.visible

    (outer, content)
}
