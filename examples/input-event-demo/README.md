# Input Event Demo

This example demonstrates the thread-safe input event system in godot-bevy, showing how to handle various types of input in a Bevy-style manner.

## ⚠️ **Important: When to Use This vs Bevy's Built-in Input**

### **Use Bevy's Built-in Input For:**
```rust
// Simple, desktop-focused games
fn movement(keys: Res<ButtonInput<KeyCode>>) {
    if keys.pressed(KeyCode::ArrowLeft) {
        // Move left
    }
}
```

**✅ Pros:**
- **Simpler**: No setup required, works immediately
- **State-based**: Easy to check "is key currently held?"
- **Performant**: Direct state queries
- **Rich API**: `just_pressed()`, `pressed()`, `just_released()`

**❌ Cons:**
- **Desktop only**: Limited mobile/touch support in Godot context
- **No Godot integration**: Can't use Godot's input map system
- **Hardcoded keys**: Users can't remap controls

### **Use Godot's Bridged Input For:**
```rust
// Cross-platform games with input mapping
fn movement(mut action_events: EventReader<ActionInput>) {
    for event in action_events.read() {
        if event.action == "move_left" && event.pressed {
            // Move left (configurable by users!)
        }
    }
}
```

**✅ Pros:**
- **Cross-platform**: Desktop, mobile, console support
- **Input mapping**: Users can remap controls in Godot
- **Touch support**: Native mobile input handling
- **Action system**: Semantic actions like "jump", "attack"
- **Godot ecosystem**: Consistent with Godot's input handling

**❌ Cons:**
- **More complex**: Event-based processing
- **Setup overhead**: Requires bridging systems
- **State queries harder**: Need to track state manually for "is held?" queries

## **Recommendation**

### 🚀 **For Rapid Prototyping & Simple Desktop Games**
```rust
// Use Bevy's built-in input - it's simpler!
fn player_movement(
    keys: Res<ButtonInput<KeyCode>>,
    mut player_query: Query<&mut Transform, With<Player>>,
) {
    let mut transform = player_query.single_mut();
    
    if keys.pressed(KeyCode::ArrowLeft) {
        transform.translation.x -= 200.0 * time.delta_seconds();
    }
    if keys.pressed(KeyCode::ArrowRight) {
        transform.translation.x += 200.0 * time.delta_seconds();
    }
}
```

### 🎮 **For Production Games & Cross-Platform**
```rust
// Use Godot's bridged input for flexibility
fn player_movement(
    mut action_events: EventReader<ActionInput>,
    mut player_query: Query<&mut Transform, With<Player>>,
) {
    for event in action_events.read() {
        match event.action.as_str() {
            "move_left" if event.pressed => {
                // Handle move left (user-configurable!)
            }
            "move_right" if event.pressed => {
                // Handle move right
            }
            _ => {}
        }
    }
}
```

---

## What This Example Demonstrates

This example shows you how to use **Godot's bridged input system** for:

- **Handle keyboard input** with key press/release events, including echo detection
- **Process mouse button clicks** with position information and button type detection
- **Track mouse movement** with delta and absolute position data
- **Support touch input** for mobile/touchscreen devices
- **Use Godot action events** for input mapping integration

## Input Event Types

### 🎹 Keyboard Input (`KeyboardInput`)
- **Keycode**: The logical key that was pressed
- **Physical Keycode**: The physical key location (useful for different keyboard layouts)
- **Pressed State**: Whether the key was pressed or released
- **Echo**: Whether this is a repeated key event (when holding a key down)

### 🖱️ Mouse Button Input (`MouseButtonInput`)
- **Button Type**: Left, Right, Middle, WheelUp, WheelDown, WheelLeft, WheelRight, Extra1, Extra2
- **Pressed State**: Whether the button was pressed or released
- **Position**: Mouse cursor position when the event occurred

### 🖱️ Mouse Motion (`MouseMotion`)
- **Delta**: How much the mouse moved since the last event
- **Position**: Current absolute mouse position

### 👆 Touch Input (`TouchInput`)
- **Finger ID**: Unique identifier for multi-touch support
- **Position**: Touch position on screen
- **Pressed State**: Whether the touch started or ended

### 🎮 Action Input (`ActionInput`) - **The Key Advantage!**
- **Action Name**: The Godot input map action name (user-configurable!)
- **Pressed State**: Whether the action was pressed or released
- **Strength**: Action strength (useful for analog inputs like gamepad triggers)

## How It Works

### Thread-Safe Design
Unlike Godot's raw input events (which contain non-`Send`/`Sync` pointers), our input system:

1. **Extracts essential data** from Godot input events on the main thread
2. **Creates thread-safe events** with only the necessary information
3. **Sends them through Bevy's event system** for processing in any system

### Event Processing
```rust
fn handle_keyboard_input(mut keyboard_events: EventReader<KeyboardInput>) {
    for event in keyboard_events.read() {
        if event.pressed && event.keycode == Key::SPACE {
            println!("Space key pressed!");
        }
    }
}

fn handle_actions(mut action_events: EventReader<ActionInput>) {
    for event in action_events.read() {
        if event.action == "jump" && event.pressed {
            println!("Jump action triggered!"); // User can remap this!
        }
    }
}
```

## What You'll See

When you run this example and interact with it, you'll see console output like:

```
🎹 Keyboard: SPACE pressed (physical: Some(SPACE))
🚀 Space bar pressed - Jump!
🖱️  Mouse: Left pressed at (245.0, 180.0)
👆 Left click - Select/Attack!
🖱️  Mouse moved: delta(15.0, -8.0) position(260.0, 172.0)
👆 Touch: finger 0 touched at (300.0, 200.0)
📱 Touch started - finger 0
🎮 Action: 'move_left' pressed (strength: 1.00)
🏃 Movement action: move_left
```

## Key Features

### 🔄 **Automatic Integration**
The input event system is automatically included when you use godot-bevy - no manual plugin setup required!

### 🎯 **Bevy-Style Events**
All events follow Bevy's event patterns and can be used with standard Bevy systems.

### 🧵 **Thread-Safe**
Events can be processed in parallel systems without synchronization issues.

### 📱 **Cross-Platform**
Supports desktop (keyboard/mouse) and mobile (touch) input seamlessly.

### 🎮 **Input Mapping Integration**
Works with Godot's input map system - users can remap controls!

## Running This Example

1. **Build**: `cargo build`
2. **Run**: Open the Godot project and run the scene
3. **Interact**: Try different input methods:
   - Press keys on your keyboard (Space, Escape, Enter)
   - Click mouse buttons and move the mouse
   - Use arrow keys (mapped to move_left, move_right, etc. actions)
   - Use touch input (if on a touchscreen device)

## Usage in Your Projects

### For Cross-Platform Games:
```rust
use bevy::prelude::*;
use godot_bevy::prelude::*;

fn build_app(app: &mut App) {
    app.add_systems(Update, (
        handle_keyboard_input,
        handle_mouse_input,
        handle_action_input, // The key advantage!
        // ... other systems
    ));
}
```

### For Simple Desktop Games:
```rust
use bevy::prelude::*;

fn build_app(app: &mut App) {
    app.add_systems(Update, (
        simple_movement,
        handle_jump
          .run_if(input_just_pressed(KeyCode::Space)),
    ))
    
}

fn simple_movement(keys: Res<ButtonInput<KeyCode>>) {
    // Use Bevy's built-in input - much simpler!
    if keys.pressed(KeyCode::Left) {
        // Move left
    }
    if keys.pressed(KeyCode::Right) {
        // Move right 
    }
    if keys.pressed(KeyCode::Up) {
        // Move up
    }
    if keys.pressed(KeyCode::Down) {
        // Move down
    }
}
```

The input events are automatically available - no additional setup required!
