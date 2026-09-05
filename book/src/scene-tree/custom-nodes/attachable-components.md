# Attachable Components

`AttachableComponent` turns an editor-authored child node into a component on
its parent's Bevy entity. The child is a one-shot carrier: successful attachment
queues it for deletion. Its node and path disappear. It never gets a Bevy entity.

Use this derive for configuration authored as child nodes. Use
[`BevyComponents`](property-mapping-with-bevy-bundle.md) when the authored node
should survive and receive components on its own entity. Neither derive provides
live Inspector synchronization.

## Define a carrier

`GodotCorePlugins` registers carriers automatically. Derive `GodotClass` and
`AttachableComponent`, name the target component, and implement the conversion.
The derive is available through `godot_bevy::prelude::*`.

```rust
use bevy::prelude::Component;
use godot::prelude::*;
use godot_bevy::prelude::*;

#[derive(Component)]
struct Movement {
    max_speed: f32,
}

#[derive(AttachableComponent, GodotClass)]
#[class(init, base=Node)]
#[gdbevy(target = Movement)]
struct MovementComponent {
    #[export]
    max_speed: f32,
}

impl From<&MovementComponent> for Movement {
    fn from(value: &MovementComponent) -> Self {
        Self {
            max_speed: value.max_speed,
        }
    }
}
```

In Godot, add `MovementComponent` as a child of the node to configure and set
`max_speed` in the Inspector. Keep the carrier empty. Ordinary and internal
children both cause rejection before the conversion runs. Nested carriers are
also rejected. Rejection preserves the authored subtree; ordinary descendants
may mirror without a Bevy relationship to the unmirrored carrier.

## Placement and lifetime

Attachment runs after ordinary scene-tree messages, so a parent added in the
same batch is available. The carrier's live parent is the destination, even if
it was reparented before the messages were processed. Duplicate adds within a
drain convert once. A carrier already queued for deletion does not convert again.

The parent must be a live, mirrored node. It cannot be the root viewport
(`/root`), another carrier, excluded, or queued for deletion. A carrier directly
under the mirrored current scene root is valid. Exclusion applies to the whole
subtree and normally prevents add messages from reaching the mirror.

An invalid placement leaves the carrier alive and unmirrored. The warning names
its class, path, parent, and the rejection reason. A missing parent mapping is
also rejected. Fix the placement and remove and re-add the carrier to trigger a
new attempt. Later empty drains do not retry. Freed, detached, or queued carriers
do not convert.

Startup scanning and `GodotScene` instantiation use the same rules. Detaching and
re-adding a surviving scene node cannot restore consumed carriers or replay their
configuration onto its new entity. Instantiate the saved `PackedScene` again for
fresh carriers.

## Conversion references

`From` may copy values and retain owned resources. Handles may refer to nodes
that survive independently, such as the parent or a sibling outside the carrier.
Do not capture the carrier, its descendants, other carriers, or paths that pass
through consumed nodes. Do not mutate the tree during conversion.

Captured nodes can still be freed later. Resolve their handles with
`GodotAccess::try_get` when their lifetime is uncertain.
