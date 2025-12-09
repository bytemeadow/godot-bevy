//! Container widgets for organizing inspector UI.

use godot::classes::{
    Button, GridContainer, HBoxContainer, HSeparator, IVBoxContainer, MarginContainer,
    PanelContainer, ScrollContainer, VBoxContainer, control, scroll_container,
};
use godot::global::HorizontalAlignment;
use godot::prelude::*;

/// Create a VBoxContainer for vertical layouts.
pub fn vbox() -> Gd<VBoxContainer> {
    let mut vbox = VBoxContainer::new_alloc();
    vbox.set_h_size_flags(control::SizeFlags::EXPAND_FILL);
    vbox
}

/// Create an HBoxContainer for horizontal layouts.
pub fn hbox() -> Gd<HBoxContainer> {
    let mut hbox = HBoxContainer::new_alloc();
    hbox.set_h_size_flags(control::SizeFlags::EXPAND_FILL);
    hbox
}

/// Create a GridContainer for grid layouts.
pub fn grid(columns: i32) -> Gd<GridContainer> {
    let mut grid = GridContainer::new_alloc();
    grid.set_columns(columns);
    grid.set_h_size_flags(control::SizeFlags::EXPAND_FILL);
    grid
}

/// Create a ScrollContainer for scrollable content.
pub fn scroll_container() -> Gd<ScrollContainer> {
    let mut scroll = ScrollContainer::new_alloc();
    scroll.set_h_size_flags(control::SizeFlags::EXPAND_FILL);
    scroll.set_v_size_flags(control::SizeFlags::EXPAND_FILL);
    scroll.set_horizontal_scroll_mode(scroll_container::ScrollMode::DISABLED);
    scroll
}

/// Create a PanelContainer for visual grouping.
pub fn panel() -> Gd<PanelContainer> {
    let mut panel = PanelContainer::new_alloc();
    panel.set_h_size_flags(control::SizeFlags::EXPAND_FILL);
    panel
}

/// Create a MarginContainer for adding padding.
pub fn margin_container(margins: i32) -> Gd<MarginContainer> {
    let mut container = MarginContainer::new_alloc();
    container.add_theme_constant_override("margin_left", margins);
    container.add_theme_constant_override("margin_right", margins);
    container.add_theme_constant_override("margin_top", margins);
    container.add_theme_constant_override("margin_bottom", margins);
    container.set_h_size_flags(control::SizeFlags::EXPAND_FILL);
    container
}

/// Create a separator line.
pub fn separator() -> Gd<HSeparator> {
    HSeparator::new_alloc()
}

/// A collapsible section that can be expanded/collapsed.
#[derive(GodotClass)]
#[class(base=VBoxContainer)]
pub struct CollapsibleSection {
    base: Base<VBoxContainer>,
    is_expanded: bool,
    header: Option<Gd<Button>>,
    content: Option<Gd<VBoxContainer>>,
    title: GString,
}

#[godot_api]
impl IVBoxContainer for CollapsibleSection {
    fn init(base: Base<VBoxContainer>) -> Self {
        Self {
            base,
            is_expanded: true,
            header: None,
            content: None,
            title: GString::new(),
        }
    }

    fn ready(&mut self) {
        // Create header button
        let header_text = self.get_header_text();
        let mut header = Button::new_alloc();
        header.set_text(&header_text);
        header.set_flat(true);
        header.set_text_alignment(HorizontalAlignment::LEFT);

        // Connect pressed signal
        let callable = self.base().callable("_on_header_pressed");
        header.connect("pressed", &callable);

        self.base_mut().add_child(&header);
        self.header = Some(header);

        // Create content container
        let mut content = VBoxContainer::new_alloc();
        content.set_visible(self.is_expanded);
        content.set_h_size_flags(control::SizeFlags::EXPAND_FILL);

        // Add left margin for indentation
        let mut margin = MarginContainer::new_alloc();
        margin.add_theme_constant_override("margin_left", 16);
        margin.set_h_size_flags(control::SizeFlags::EXPAND_FILL);
        margin.add_child(&content.clone());

        self.base_mut().add_child(&margin);
        self.content = Some(content);
    }
}

#[godot_api]
impl CollapsibleSection {
    /// Create a new collapsible section with the given title.
    #[func]
    pub fn create(title: GString, expanded: bool) -> Gd<Self> {
        Gd::from_init_fn(|base| Self {
            base,
            is_expanded: expanded,
            header: None,
            content: None,
            title,
        })
    }

    /// Get the content container to add child widgets to.
    #[func]
    pub fn get_content(&self) -> Option<Gd<VBoxContainer>> {
        self.content.clone()
    }

    /// Check if the section is expanded.
    #[func]
    pub fn is_expanded(&self) -> bool {
        self.is_expanded
    }

    /// Set whether the section is expanded.
    #[func]
    pub fn set_expanded(&mut self, expanded: bool) {
        self.is_expanded = expanded;
        if let Some(ref mut content) = self.content {
            content.set_visible(expanded);
        }
        let header_text = self.get_header_text();
        if let Some(ref mut header) = self.header {
            header.set_text(&header_text);
        }
    }

    /// Toggle the expanded state.
    #[func]
    pub fn toggle(&mut self) {
        self.set_expanded(!self.is_expanded);
    }

    #[func]
    fn _on_header_pressed(&mut self) {
        self.toggle();
    }

    fn get_header_text(&self) -> GString {
        let arrow = if self.is_expanded { "▼" } else { "▶" };
        GString::from(&format!("{} {}", arrow, self.title)[..])
    }
}
