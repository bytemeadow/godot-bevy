use godot::{
    builtin::Side, 
    classes::{control::SizeFlags, Label, ScrollContainer, Tree, VBoxContainer, Window}, 
    obj::NewAlloc, 
    prelude::*
};

use crate::bridge::GodotNodeHandle;
use super::{utils::pretty_type_name, DEFAULT_WINDOW_SIZE};

/// Create the main world inspector window with tree view
pub fn create_world_inspector_window() -> Option<GodotNodeHandle> {
    // Get SceneTree directly from the engine
    let scene_tree = godot::classes::Engine::singleton()
        .get_main_loop()?
        .cast::<godot::classes::SceneTree>();

    // Create a simple window
    let mut window = Window::new_alloc();
    window.set_title("Bevy World Inspector");
    window.set_size(Vector2i::new(DEFAULT_WINDOW_SIZE.0, DEFAULT_WINDOW_SIZE.1));
    
    // Create main container with proper fill settings
    let mut main_container = VBoxContainer::new_alloc();
    main_container.set_name("MainContainer");
    
    // Set anchors to fill the window
    main_container.set_anchor_and_offset(Side::LEFT, 0.0, 0.0);
    main_container.set_anchor_and_offset(Side::TOP, 0.0, 0.0);
    main_container.set_anchor_and_offset(Side::RIGHT, 1.0, 0.0);
    main_container.set_anchor_and_offset(Side::BOTTOM, 1.0, 0.0);
    
    // Create title label
    let mut title_label = Label::new_alloc();
    title_label.set_text("🔍 Bevy World Inspector");
    title_label.set_name("TitleLabel");
    title_label.add_theme_font_size_override("font_size", 24);
    main_container.add_child(&title_label);
    
    // Create scrollable area that expands to fill remaining space
    let mut scroll_container = ScrollContainer::new_alloc();
    scroll_container.set_name("ScrollContainer");
    
    // Make the scroll container expand to fill available space
    scroll_container.set_h_size_flags(SizeFlags::EXPAND_FILL);
    scroll_container.set_v_size_flags(SizeFlags::EXPAND_FILL);
    
    // Use a Tree widget for expandable sections (like the original)
    let mut inspector_tree = Tree::new_alloc();
    inspector_tree.set_name("InspectorTree");
    inspector_tree.set_h_size_flags(SizeFlags::EXPAND_FILL);
    inspector_tree.set_v_size_flags(SizeFlags::EXPAND_FILL);
    inspector_tree.set_hide_root(true); // Hide the invisible root node
    inspector_tree.set_column_titles_visible(false);
    
    scroll_container.add_child(&inspector_tree);
    main_container.add_child(&scroll_container);
    
    // Add the container to the window
    window.add_child(&main_container);
    
    // Add window to scene tree
    let mut root = scene_tree.get_root()?;
    root.add_child(&window);
    
    // Show the window
    window.show();
    
    Some(GodotNodeHandle::new(window.upcast::<godot::classes::Node>()))
}

/// Create a simple inspector window for a specific resource type
pub fn create_resource_inspector_window<T>() -> Option<GodotNodeHandle> {
    let scene_tree = godot::classes::Engine::singleton()
        .get_main_loop()?
        .cast::<godot::classes::SceneTree>();

    let mut window = Window::new_alloc();
    window.set_title(&format!("Resource: {}", pretty_type_name::<T>()));
    window.set_size(Vector2i::new(400, 300));
    
    let mut label = Label::new_alloc();
    label.set_text(&format!("Resource Inspector for {}", pretty_type_name::<T>()));
    label.set_name("ResourceLabel");
    label.add_theme_font_size_override("font_size", 16);
    window.add_child(&label);
    
    let mut root = scene_tree.get_root()?;
    root.add_child(&window);
    window.show();
    
    Some(GodotNodeHandle::new(window.upcast::<godot::classes::Node>()))
}

/// Create a simple inspector window for asset types
pub fn create_asset_inspector_window<A>() -> Option<GodotNodeHandle> {
    let scene_tree = godot::classes::Engine::singleton()
        .get_main_loop()?
        .cast::<godot::classes::SceneTree>();

    let mut window = Window::new_alloc();
    window.set_title(&format!("Assets: {}", pretty_type_name::<A>()));
    window.set_size(Vector2i::new(400, 300));
    
    let mut label = Label::new_alloc();
    label.set_text(&format!("Asset Inspector for {}", pretty_type_name::<A>()));
    label.set_name("AssetLabel");
    label.add_theme_font_size_override("font_size", 16);
    window.add_child(&label);
    
    let mut root = scene_tree.get_root()?;
    root.add_child(&window);
    window.show();
    
    Some(GodotNodeHandle::new(window.upcast::<godot::classes::Node>()))
}

/// Create a simple inspector window for query filters
pub fn create_filter_query_inspector_window<F>() -> Option<GodotNodeHandle> {
    let scene_tree = godot::classes::Engine::singleton()
        .get_main_loop()?
        .cast::<godot::classes::SceneTree>();

    let mut window = Window::new_alloc();
    window.set_title(&format!("Query: {}", pretty_type_name::<F>()));
    window.set_size(Vector2i::new(400, 300));
    
    let mut label = Label::new_alloc();
    label.set_text(&format!("Filter Query Inspector for {}", pretty_type_name::<F>()));
    label.set_name("QueryLabel");
    label.add_theme_font_size_override("font_size", 16);
    window.add_child(&label);
    
    let mut root = scene_tree.get_root()?;
    root.add_child(&window);
    window.show();
    
    Some(GodotNodeHandle::new(window.upcast::<godot::classes::Node>()))
} 