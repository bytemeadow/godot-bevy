# Transitional Mailbox Bridge (Godot -> Bevy Messages)

This pattern is for teams migrating existing Godot/GDScript gameplay into `godot-bevy`.

It provides a safe, typed bridge from temporary script-side "pending" fields to Bevy messages,
so gameplay logic can stay ECS-native while legacy scripts are ported incrementally.

> This is a **transition aid**, not the ideal long-term architecture.

## When to use this

Use it when:

- Some gameplay producers are still in GDScript (for example, enemy contact damage or collectible callbacks)
- You want ECS systems to remain the source of truth
- You need a low-risk, iterative migration path

## When not to use this

Avoid it for greenfield systems.

For new gameplay, prefer:

- direct Bevy messages/events from Rust systems
- typed signal routing via `GodotSignals`
- component/resource-driven state in ECS

## API overview

`godot-bevy` provides:

- `GodotMailboxMessage` trait
- `GodotMailboxPlugin<T, Marker>`
- `GodotMailboxSet::Drain`

The plugin runs in `PhysicsUpdate`, queries entities with `Marker` + `GodotNodeHandle`,
calls `T::drain_from_node(...)`, and writes `T` when present.

## Example

```rust,ignore
use bevy::prelude::*;
use godot::builtin::{Variant, Vector3};
use godot::classes::Node;
use godot::prelude::ToGodot;
use godot_bevy::prelude::*;

const PENDING_DAMAGE: &str = "rust_pending_damage";
const PENDING_DAMAGE_FORCE: &str = "rust_pending_damage_force";

#[derive(Message, Debug, Clone, Copy)]
pub struct PlayerDamageRequest {
    pub target: GodotNodeHandle,
    pub force: Vector3,
}

impl GodotMailboxMessage for PlayerDamageRequest {
    fn drain_from_node(node: &mut Node, source: GodotNodeHandle) -> Option<Self> {
        let pending = node.get(PENDING_DAMAGE).try_to::<bool>().unwrap_or(false);
        let force = node
            .get(PENDING_DAMAGE_FORCE)
            .try_to::<Vector3>()
            .unwrap_or(Vector3::ZERO);

        // Important: clear pending fields as part of draining.
        node.set(PENDING_DAMAGE, &false.to_variant());
        node.set(PENDING_DAMAGE_FORCE, &Vector3::ZERO.to_variant());

        pending.then_some(Self {
            target: source,
            force,
        })
    }
}

#[derive(Component)]
struct PlayerMarker;

app.add_plugins(GodotMailboxPlugin::<PlayerDamageRequest, PlayerMarker>::default())
    .add_systems(
        PhysicsUpdate,
        apply_damage_requests.after(GodotMailboxSet::Drain),
    );
```

## Migration guidance

1. Keep mailbox fields small and explicit (for example, `pending_damage`, `pending_force`).
2. Drain once per tick and clear immediately.
3. Consume typed messages in ECS systems.
4. Port producers from GDScript to Rust over time.
5. Remove mailbox fields/plugin once all producers are Rust-native.

The end goal is to delete the mailbox layer entirely.
