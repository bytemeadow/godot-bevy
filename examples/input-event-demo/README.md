# Input Event Demo

This example demonstrates the thread-safe input event system in godot-bevy, showing how to handle various types of input in a Bevy-style manner.

> 📖 **For detailed information about input systems**, see [docs/INPUT_SYSTEMS.md](../../docs/INPUT_SYSTEMS.md)

## What This Example Demonstrates

This example shows you how to use **Godot's bridged input system** for:

- **Handle keyboard input** with key press/release events, including echo detection
- **Process mouse button clicks** with position information and button type detection
- **Track mouse movement** with delta and absolute position data
- **Support touch input** for mobile/touchscreen devices
- **Use Godot action events** for input mapping integration

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

## Key Code Examples

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

## Key Features Demonstrated

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

This example is particularly useful for:
- Understanding the different input event types available
- Learning how to process input events in Bevy systems
- Testing cross-platform input handling
- Seeing how Godot's action system integrates with Bevy
