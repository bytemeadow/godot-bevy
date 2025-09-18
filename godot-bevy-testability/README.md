# Godot-Bevy Testability

**Bevy-specific testing utilities for Godot-Bevy integration projects**

This crate provides specialized testing utilities for projects using the [godot-bevy](../godot-bevy) integration. It builds upon the general-purpose [`godot-testability-runtime`](../godot-testability-runtime) to offer Bevy-specific testing tools.

## Overview

While `godot-testability-runtime` provides the core embedded Godot testing infrastructure, this crate adds:

- **Transform Synchronization Testing** - Utilities for testing coordinate system conversions between Godot and Bevy
- **Bevy ECS Testing Helpers** - Simplified testing of Bevy components, entities, and systems
- **Godot-Bevy Bridge Testing** - Tools for testing the integration points between the two engines
- **Specialized Test Hosts** - Bevy-aware test environments for comprehensive integration testing

## Quick Start

Add to your `Cargo.toml`:

```toml
[dev-dependencies]
godot-bevy-testability = { path = "path/to/godot-bevy-testability" }
tokio = { version = "1.0", features = ["full"] }
```

Write a Bevy-Godot integration test:

```rust
use godot_bevy_testability::prelude::*;

#[derive(Default)]
struct TransformSyncTest;

#[async_trait]
impl EmbeddedTestCase for TransformSyncTest {
    async fn run_test(&mut self) -> TestResult<()> {
        // Test transform conversion between Godot and Bevy
        let godot_transform = Transform2D::from_angle_origin(PI/4.0, Vector2::new(10.0, 20.0));
        let bevy_transform = transforms::godot_transform2d_to_bevy(godot_transform);
        let converted_back = transforms::bevy_transform_to_godot2d(bevy_transform);
        
        transforms::assert_transform2d_approx_eq(godot_transform, converted_back, 0.001)?;
        
        // Test Bevy ECS functionality
        let mut app = bevy_ecs::create_test_app();
        let entity = app.world_mut().spawn(MyComponent { value: 42 }).id();
        bevy_ecs::assert_entity_has_component::<MyComponent>(app.world(), entity)?;
        
        Ok(())
    }
    
    fn name(&self) -> &str { "TransformSyncTest" }
}

#[derive(Component)]
struct MyComponent { value: i32 }

#[tokio::main]
async fn main() -> TestResult<()> {
    let mut runner = BevyGodotTestRunner::for_development();
    let result = runner.run_test(TransformSyncTest::default()).await?;
    
    if result.passed {
        println!("✅ Bevy-Godot integration test passed!");
    }
    
    Ok(())
}
```

## Key Features

### Transform Testing Utilities

```rust
use godot_bevy_testability::prelude::*;

// Test 2D transform round-trip conversion
let godot_2d = Transform2D::from_angle_origin(PI/4.0, Vector2::new(10.0, 20.0));
let bevy_transform = transforms::godot_transform2d_to_bevy(godot_2d);
let back_to_godot = transforms::bevy_transform_to_godot2d(bevy_transform);
transforms::assert_transform2d_approx_eq(godot_2d, back_to_godot, 0.001)?;

// Test 3D transforms
let godot_3d = Transform3D::new(Basis::from_euler(...), Vector3::new(1.0, 2.0, 3.0));
let bevy_3d = transforms::godot_transform3d_to_bevy(godot_3d);
transforms::assert_vector3_approx_eq(bevy_3d.translation, Vec3::new(1.0, 2.0, 3.0), 0.001)?;
```

### Bevy ECS Testing

```rust
use godot_bevy_testability::prelude::*;

// Create test apps with plugins
let mut app = bevy_ecs::create_test_app_with_plugins(vec![MyPlugin]);

// Test entity creation and components
let entity = app.world_mut().spawn(TestComponent { value: 42 }).id();
bevy_ecs::assert_entity_has_component::<TestComponent>(app.world(), entity)?;
bevy_ecs::assert_entity_count_with_component::<TestComponent>(app.world(), 1)?;

// Test component absence
let empty_entity = app.world_mut().spawn_empty().id();
bevy_ecs::assert_entity_lacks_component::<TestComponent>(app.world(), empty_entity)?;
```

### Specialized Test Hosts

```rust
use godot_bevy_testability::prelude::*;

// Create Bevy-aware test runners
let mut runner = BevyGodotTestRunner::for_development()  // Visual debugging enabled
// or
let mut runner = BevyGodotTestRunner::for_ci()           // Headless, timeouts enabled

// Runs tests in embedded Godot with Bevy-specific setup
let result = runner.run_test(MyBevyGodotTest::default()).await?;
```

## Architecture

This crate is designed as a thin layer over `godot-testability-runtime`:

```
┌─────────────────────────────────────────┐
│      godot-bevy-testability             │
│   (Bevy-specific utilities)             │
├─────────────────────────────────────────┤
│      godot-testability-runtime          │
│   (General embedded testing)            │  
├─────────────────────────────────────────┤
│           libgodot.dylib                │
│      (Embedded Godot Engine)            │
└─────────────────────────────────────────┘
```

**Dependencies:**
- **Runtime**: All core functionality comes from `godot-testability-runtime`
- **Bevy Integration**: Adds ECS testing helpers and transform utilities
- **Godot-Bevy Bridge**: Utilities specific to the `godot-bevy` integration

## Examples

Run the examples to see the utilities in action:

```bash
# Simple Bevy-specific testing demonstration
cargo run --package godot-bevy-testability --example simple_bevy_test

# Comprehensive Bevy-Godot integration testing
cargo run --package godot-bevy-testability --example bevy_godot_integration_test
```

## Relationship to General Framework

This crate **extends** rather than **replaces** the general framework:

- **Use `godot-testability-runtime`** for general Godot testing needs
- **Use `godot-bevy-testability`** when you need Bevy-specific testing utilities
- **Both** provide embedded Godot runtime testing capabilities
- **This crate** adds transform conversion, ECS helpers, and Bevy-aware test hosts

## Contributing

Since this crate focuses on Bevy-specific utilities, contributions should:

1. **Enhance Bevy Integration** - Better ECS testing helpers, transform utilities
2. **Improve Test Ergonomics** - More convenient APIs for common Bevy-Godot testing patterns  
3. **Add Specialized Helpers** - Testing utilities for specific godot-bevy features
4. **Maintain Focus** - General testing improvements should go to `godot-testability-runtime`

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../LICENSE-APACHE))
- MIT License ([LICENSE-MIT](../LICENSE-MIT))

at your option.

---

**Focus on what matters:** Test your Bevy-Godot integration with confidence! 🦀🎮