//! Command system for thread-safe Godot API access
//!
//! This module provides a command/event pattern that allows most game logic
//! to run multi-threaded while keeping Godot API access on the main thread.

use bevy::prelude::*;
use godot::builtin::{StringName, Vector2};
use godot::classes::AnimatedSprite2D;
use godot::obj::Gd;
use godot_bevy::prelude::*;

/// Commands for UI operations
#[derive(Event, Debug, Clone)]
pub enum UICommand {
    /// Set text on a UI element
    SetText { target: UIElement, text: String },
    /// Set visibility of a UI element
    SetVisible { target: UIElement, visible: bool },
    /// Show a temporary message
    ShowMessage { text: String },
}

/// Commands for node operations
#[derive(Event, Debug, Clone)]
pub enum NodeCommand {
    /// Set visibility of any node
    #[allow(dead_code)]
    SetVisible { entity: Entity, visible: bool },
    /// Destroy a node
    Destroy { entity: Entity },
    /// Set position of a node
    #[allow(dead_code)]
    SetPosition { entity: Entity, position: Vector2 },
}

/// Commands for animation operations
#[derive(Event, Debug, Clone)]
pub enum AnimationCommand {
    /// Play an animation on a sprite
    #[allow(dead_code)]
    Play {
        entity: Entity,
        animation: Option<StringName>,
    },
    /// Stop animation on a sprite
    #[allow(dead_code)]
    Stop { entity: Entity },
    /// Set sprite flip properties
    #[allow(dead_code)]
    SetFlip {
        entity: Entity,
        flip_h: bool,
        flip_v: bool,
    },
}

/// UI element identifiers
#[derive(Debug, Clone, PartialEq)]
pub enum UIElement {
    StartButton,
    ScoreLabel,
    MessageLabel,
}

/// Resource to hold UI element handles
#[derive(Resource, Default)]
pub struct UIHandles {
    pub start_button: Option<GodotNodeHandle>,
    pub score_label: Option<GodotNodeHandle>,
    pub message_label: Option<GodotNodeHandle>,
}

impl UIHandles {
    pub fn get_handle(&self, element: &UIElement) -> Option<&GodotNodeHandle> {
        match element {
            UIElement::StartButton => self.start_button.as_ref(),
            UIElement::ScoreLabel => self.score_label.as_ref(),
            UIElement::MessageLabel => self.message_label.as_ref(),
        }
    }
}

/// Component to cache commonly accessed data to avoid Godot API calls
#[derive(Component, Debug)]
pub struct CachedScreenSize {
    pub size: Vector2,
}

/// Component to track node visibility state
#[derive(Component, Debug)]
pub struct VisibilityState {
    pub visible: bool,
    pub dirty: bool,
}

impl Default for VisibilityState {
    fn default() -> Self {
        Self {
            visible: true,
            dirty: false,
        }
    }
}

impl VisibilityState {
    pub fn set_visible(&mut self, visible: bool) {
        if self.visible != visible {
            self.visible = visible;
            self.dirty = true;
        }
    }
}

/// Component for animation state
#[derive(Component, Debug, Default)]
pub struct AnimationState {
    pub current_animation: Option<StringName>,
    pub playing: bool,
    pub flip_h: bool,
    pub flip_v: bool,
    pub dirty: bool,
}

impl AnimationState {
    pub fn play(&mut self, animation: Option<StringName>) {
        self.current_animation = animation;
        self.playing = true;
        self.dirty = true;
    }

    pub fn stop(&mut self) {
        self.playing = false;
        self.dirty = true;
    }

    pub fn set_flip(&mut self, flip_h: bool, flip_v: bool) {
        if self.flip_h != flip_h || self.flip_v != flip_v {
            self.flip_h = flip_h;
            self.flip_v = flip_v;
            self.dirty = true;
        }
    }
}

/// Plugin that sets up the command system
pub struct CommandSystemPlugin;

impl Plugin for CommandSystemPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UIHandles>()
            .add_event::<UICommand>()
            .add_event::<NodeCommand>()
            .add_event::<AnimationCommand>()
            .add_systems(
                Update,
                (
                    // Main thread systems that process commands
                    process_ui_commands,
                    process_node_commands,
                    process_animation_commands,
                    sync_visibility_state,
                    sync_animation_state,
                ),
            );
    }
}

/// Main thread system that processes UI commands
#[main_thread_system]
fn process_ui_commands(mut ui_commands: EventReader<UICommand>, ui_handles: Res<UIHandles>) {
    use godot::classes::{Button, Label};

    for command in ui_commands.read() {
        match command {
            UICommand::SetText { target, text } => {
                if let Some(handle) = ui_handles.get_handle(target)
                    && let Some(mut label) = handle.clone().try_get::<Label>()
                {
                    label.set_text(text);
                }
            }
            UICommand::SetVisible { target, visible } => {
                if let Some(handle) = ui_handles.get_handle(target)
                    && let Some(mut button) = handle.clone().try_get::<Button>()
                {
                    button.set_visible(*visible);
                }
            }
            UICommand::ShowMessage { text } => {
                if let Some(handle) = ui_handles.get_handle(&UIElement::MessageLabel)
                    && let Some(mut label) = handle.clone().try_get::<Label>()
                {
                    label.set_text(text);
                }
            }
        }
    }
}

/// Main thread system that processes node commands
#[main_thread_system]
fn process_node_commands(
    mut node_commands: EventReader<NodeCommand>,
    mut nodes: Query<&mut GodotNodeHandle>,
    mut commands: Commands,
) {
    use godot::classes::{CanvasItem, Node};

    for command in node_commands.read() {
        match command {
            NodeCommand::SetVisible { entity, visible } => {
                if let Ok(mut handle) = nodes.get_mut(*entity)
                    && let Some(mut canvas_item) = handle.try_get::<CanvasItem>()
                {
                    canvas_item.set_visible(*visible);
                }
            }
            NodeCommand::Destroy { entity } => {
                if let Ok(mut handle) = nodes.get_mut(*entity)
                    && let Some(mut node) = handle.try_get::<Node>()
                {
                    node.queue_free();
                }
                commands.entity(*entity).despawn();
            }
            NodeCommand::SetPosition { entity, position } => {
                if let Ok(mut handle) = nodes.get_mut(*entity)
                    && let Some(mut node) = handle.try_get::<godot::classes::Node2D>()
                {
                    node.set_position(*position);
                }
            }
        }
    }
}

/// Main thread system that processes animation commands
#[main_thread_system]
fn process_animation_commands(
    mut animation_commands: EventReader<AnimationCommand>,
    mut nodes: Query<&mut GodotNodeHandle>,
) {
    use godot::classes::AnimatedSprite2D;

    for command in animation_commands.read() {
        if let Ok(mut handle) = nodes.get_mut(command.entity())
            && let Some(mut sprite) = handle.try_get::<AnimatedSprite2D>()
        {
            match command {
                AnimationCommand::Play { animation, .. } => {
                    if let Some(anim) = animation {
                        sprite.set_animation(anim);
                    }
                    sprite.play();
                }
                AnimationCommand::Stop { .. } => {
                    sprite.stop();
                }
                AnimationCommand::SetFlip { flip_h, flip_v, .. } => {
                    sprite.set_flip_h(*flip_h);
                    sprite.set_flip_v(*flip_v);
                }
            }
        }
    }
}

/// Main thread system that syncs visibility state to Godot nodes
#[main_thread_system]
fn sync_visibility_state(
    mut nodes: Query<(&mut GodotNodeHandle, &mut VisibilityState), Changed<VisibilityState>>,
) {
    use godot::classes::CanvasItem;

    for (mut handle, mut visibility) in nodes.iter_mut() {
        if visibility.dirty {
            if let Some(mut canvas_item) = handle.try_get::<CanvasItem>() {
                canvas_item.set_visible(visibility.visible);
            }
            visibility.dirty = false;
        }
    }
}

/// Main thread system that syncs animation state to Godot sprites
#[main_thread_system]
fn sync_animation_state(
    mut nodes: Query<(&mut GodotNodeHandle, &mut AnimationState), Changed<AnimationState>>,
) {
    use godot::classes::AnimatedSprite2D;

    for (mut handle, mut anim_state) in nodes.iter_mut() {
        if anim_state.dirty {
            // First try to get the node directly as AnimatedSprite2D
            if let Some(mut sprite) = handle.try_get::<AnimatedSprite2D>() {
                apply_animation_state(&mut sprite, &anim_state);
            }
            // If that fails, try to find AnimatedSprite2D as a child
            else if let Some(node) = handle.try_get::<godot::classes::Node>() {
                let mut sprite = node.get_node_as::<AnimatedSprite2D>("AnimatedSprite2D");
                apply_animation_state(&mut sprite, &anim_state);
            }
            anim_state.dirty = false;
        }
    }
}

/// Helper function to apply animation state to a sprite
fn apply_animation_state(sprite: &mut Gd<AnimatedSprite2D>, anim_state: &AnimationState) {
    if anim_state.playing {
        if let Some(ref animation) = anim_state.current_animation {
            sprite.set_animation(animation);
        }
        sprite.play();
    } else {
        sprite.stop();
    }

    sprite.set_flip_h(anim_state.flip_h);
    sprite.set_flip_v(anim_state.flip_v);
}

impl AnimationCommand {
    fn entity(&self) -> Entity {
        match self {
            AnimationCommand::Play { entity, .. } => *entity,
            AnimationCommand::Stop { entity } => *entity,
            AnimationCommand::SetFlip { entity, .. } => *entity,
        }
    }
}
