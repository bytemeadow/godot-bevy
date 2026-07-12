# Excluding Nodes

By default every Godot node becomes a Bevy entity. To keep a node -- and everything
under it -- out of the ECS, give it the `_bevy_exclude` metadata. Excluded nodes never
cross into Bevy: the GDScript watcher filters them out before they reach Rust.

The typical use is a UI or editor-only subtree you never query from ECS. Set the metadata
in the editor (select the node, **Inspector > Add Metadata**, name it `_bevy_exclude`,
type `bool`, value `true`), or from GDScript:

```gdscript
$UI.set_meta("_bevy_exclude", true)
```

You can also set it from Rust on a node you have a handle to:

```rust
node.set_meta("_bevy_exclude", &true.to_variant());
```

Set it *before* the node enters the tree -- exclusion is evaluated when the node is added.

## Exclusion is subtree-wide

Excluding a node also excludes all of its descendants. Mark one UI or editor-only root
and the whole branch stays out of the ECS; you don't tag each child.

## What you give up

An excluded node has no entity, so nothing that flows through the ECS applies to it:

- **Collision events.** Excluded bodies produce no collision events, and a still-mirrored
  node that collides with an excluded one gets no entity for its excluded partner.
- **Autosync components.** `#[derive(GodotNode)]` / `BevyComponents` components and any
  registered bundles do not materialize for excluded nodes.
- **Transforms, markers, groups** -- none of the usual decoration happens.

If you want a node in the ECS but want to skip only its *transform reads*, don't exclude
it -- use [`DisableGodotTransformRead`](../transforms/sync-modes.md#opting-out-of-reads)
instead.

## Timing

Exclusion is decided once, when the node is added. Adding or removing the metadata at
runtime does not retroactively mirror or unmirror a node. The one exception:
**reparenting an already-mirrored node into an excluded subtree tears its entity down**,
so a node that moves out of the ECS does not linger with a stale parent.
