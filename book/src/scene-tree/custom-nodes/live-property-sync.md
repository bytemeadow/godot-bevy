# Live property sync

Add `sync = two_way` or `sync = to_godot` to a mapped field to keep its Godot
property and Bevy value connected after creation. Both `GodotNode` and
`BevyComponents` support it. Fields without `sync` keep their creation-only
behavior; no extra plugin is needed beyond the scene-tree support in
`GodotCorePlugins`.

| Mode | At creation | During play |
| --- | --- | --- |
| No `sync` | Copy Godot into Bevy | No further synchronization |
| `sync = to_godot` | Copy Godot into Bevy | Publish changed Bevy values; never import later Godot edits |
| `sync = two_way` | Copy Godot into Bevy | Import changed Godot values and publish changed Bevy values; Bevy wins conflicts on the same field |

## Publish health to a Label

```rust
{{#include ../../../../book-tests/src/live_property_sync.rs:health}}
```

Run `take_damage` in `Update`. A Label script can read the generated property:

```gdscript
extends Label
@export var source: HealthView

func _process(_delta: float) -> void:
    text = str(source.current)
```

Configure the Label to process after `BevyApp` if it must display the current
publication. A Label that runs earlier sees the previous value. This mapping
publishes `HealthView.current`; it does not synchronize the Label's native
`text` property.

## Tune a field from Godot

Use `two_way` for values authored by GDScript, the remote Inspector, or animation:

```rust
{{#include ../../../../book-tests/src/live_property_sync.rs:tuning}}
```

Godot edits to `speed` reach `Movement.speed` at the next read. `acceleration`
is copied only at creation. A Bevy system can change either field without
replacing the component or losing its other runtime data.

## Convert in both directions

`with = path` takes an owned Godot value and returns the Bevy field value.
`into = path` takes an owned Bevy field value and returns the Godot value.
A live mapping with `with` must also provide `into`, including `to_godot`,
because creation still uses the inbound conversion.

```rust
{{#include ../../../../book-tests/src/live_property_sync.rs:conversion}}
```

An `AnimationPlayer` property track can animate `ChargeView.fraction` from 0 to
100. Bevy receives a fraction from 0 to 1. A Bevy edit publishes the percentage.
The bridge remembers the raw Godot value it last read or wrote, so a lossy
conversion does not feed its own output back into Bevy on the following frame.

For strings, keep the Bevy field as `String` and convert the Godot property:

```rust
{{#include ../../../../book-tests/src/live_property_sync.rs:strings}}
```

## Read and write boundaries

Each render frame has one read in `PreUpdate`, in `PropertySyncSet::Read`, and
one write in `PostUpdate`, in `PropertySyncSet::Write`. They run on frames with
zero or several physics ticks, and while Bevy virtual time is paused, provided
`BevyApp` still receives its Godot callbacks. These boundaries are separate from
[transform synchronization](../../transforms/sync-modes.md), which runs at
physics rate.

`FixedUpdate` and `Update` consumers see the result of that frame's property
read. Order a consumer in `PreUpdate` after `Read`; order an author in
`PostUpdate` before `Write`:

```rust
{{#include ../../../../book-tests/src/live_property_sync.rs:ordering}}
```

Before importing each field, the bridge checks whether Bevy changed it since
its last snapshot. A pending Bevy edit prevents an import and is published at
`Write`. This includes edits in `Startup`, `First`, signal observers, and the
ECS inspector, as well as gameplay systems. An edit later in `FixedUpdate` or
`Update` also publishes at `Write`. Normal Bevy system ordering decides the
final value when several Bevy systems write the same field.

Conflicts are per field. If Godot changes speed and Bevy changes health on the
same component, both changes survive. A Godot-only import does not cause an
outbound echo. A pending Bevy edit does publish even when its converted output
equals the previous Godot snapshot, so it can overwrite a competing late Godot
edit.

Godot edits after `Read` arrive at the next `Read`, unless a Bevy edit overwrites
them at `Write`. Edits after `Write` belong to the next cycle. Godot process
priority and an `AnimationPlayer`'s callback mode determine which side of these
boundaries an author runs on. Intermediate writes that change back before the
next sample cannot be observed.

## Supported values and mappings

Live fields must be independently accessible values. The plain-value path
requires `Clone + PartialEq + Send + Sync + 'static` on both the Bevy value and
the Godot value. The whole Bevy component does not need `Clone` or `Reflect`.
`GString` and `StringName` Godot properties have built-in owned `String`
snapshots; they do not need to satisfy the plain-value snapshot bounds.
For `f32` and `f64`, comparisons use bits, so unchanged NaNs stay unchanged and
positive and negative zero are distinct.

Snapshot cloning and comparison cost grows with the number and size of live
values. Two-way fields are polled each render frame. `to_godot` fields have no
recurring Godot reads. Arrays, dictionaries, `Variant`, and objects are unsupported
on either side, even with converters. Expose a separate plain stored property
for the value you want to synchronize.

Generated named fields, tuple fields, newtype companions, and struct companions
all accept `sync`. For Godot-first classes, annotate the source field of either
a newtype binding or an N→1 binding. A live source must have exactly one Bevy
destination, and live mappings cannot overlap on a Bevy target. Creation-only
fan-out remains supported. See the [grammar reference](property-mapping-with-bevy-bundle.md#live-mapping-grammar).

Godot-first live sources must be plain stored `#[export]` or `#[var]` fields.
Custom getters and setters are unsupported. Immutable Bevy components support
`to_godot` only. Native Godot properties such as `Node2D.position` and
`Label.text` are outside this feature. [Attachable components](attachable-components.md)
remain creation-only because their carrier nodes are consumed.

## Component and node lifetimes

Creation seeds the snapshots from the components that were constructed. An
unchanged new entity does not publish needlessly, and a `Startup` edit is
recognized before the first read. Pure Bevy entities have no property-sync
state; spawning a component alone does not create or bind a Godot node.

Removing a mapped component skips only its fields and does not recreate it.
Reinserting it makes its current Bevy values authoritative for the first
publication; subsequent Godot edits can import again. This also applies to a
remove/reinsert between the read and write passes.

A reparent within the mirrored tree preserves the entity and its sync state.
Changing the handle to another node of the mapped class reseeds from current
Bevy values and publishes them to the new node. Missing, freed, queued-for-deletion,
or wrong-class nodes are skipped. Excluded subtrees never acquire a binding.
Ordinary detach/despawn and protected detach follow the
[scene-tree lifecycle](../timing.md#runtime-scene-updates).
