# The Event Bridge

Bevy systems react to Godot through observers. The event bridge is how something *outside* a Bevy system — a Godot node's Rust method, or a plain GDScript script — gets a Bevy [`Event`](https://docs.rs/bevy/latest/bevy/ecs/event/trait.Event.html) into that observer loop. The event lands as an `On<T>` trigger, the same place a Godot signal lands.

Every fire enqueues onto a per-app channel and is delivered on the next `First` drain — it never triggers synchronously. From inside a Bevy system you'd use `Commands::trigger`; the bridge is for callers who aren't in the ECS yet.

There are two ways in, depending on who's sending.

## From Rust

If you have a `Gd<BevyApp>`, you send a typed event straight in — no `Variant`, no string name, no registration:

```rust,ignore
#[derive(Event, Clone)]
struct Damage { amount: i32 }

#[godot_api]
impl Enemy {
    #[func]
    fn take_hit(&mut self, amount: i32) {
        if let Some(app) = BevyApp::try_singleton() {
            app.bind().send_event(Damage { amount });
        }
    }
}
```

`try_singleton()` resolves the `/root/BevyAppSingleton` autoload. If you run more than one `BevyApp`, skip it and send to the one you mean: `godot_bevy::send_event(&that_app, Damage { amount })` — the same call, spelled as a free function.

Handle it like any observer:

```rust,ignore
app.add_plugins(GodotEventBridgePlugin)
   .add_observer(|trigger: On<Damage>, mut hp: ResMut<Health>| {
       hp.0 -= trigger.event().amount;
   });
```

Fire these from the main thread, from a node callback *between* frames — they bind the app to reach its world, so don't fire from inside a running Bevy frame (a signal a system emitted synchronously). Off the main thread, hold a cloned `GodotEventSender` (a `Res<GodotEventSender>` you cloned on the main thread) and `.send()` through that — it's a plain channel send, safe from anywhere.

## From GDScript

GDScript can't name a Rust type, so you register the name once on the Rust side with a mapper from the payload `Variant`:

```rust,ignore
#[derive(Event, Clone)]
struct Damage { amount: i64 }   // GDScript ints arrive as i64

app.add_plugins(GodotEventBridgePlugin)
   .add_godot_event::<Damage>("damage", |payload| {
       let dict = payload.try_to::<VarDictionary>().ok()?;
       Some(Damage { amount: dict.get("amount")?.try_to::<i64>().ok()? })
   });
```

Then any script fires it through the autoload:

```gdscript
get_node("/root/BevyAppSingleton").send_event("damage", { "amount": 10 })
```

For a transparent newtype you can drop the mapper — `GodotConvert` gives you the decode for free (gdext derives `FromGodot` from it):

```rust,ignore
#[derive(Event, Clone, GodotConvert)]
#[godot(transparent)]
struct Volume(f64);

app.add_godot_event_from::<Volume>("volume");
```

A unit event takes a `null` payload: `send_event("game_over", null)`.

The decode is strict on purpose: a float where you asked for an `i64`, or anything out of range, is dropped with a warning rather than quietly coerced. An unknown name is dropped with a warning that lists the names that *are* registered, so a typo is quick to spot. Your mapper returns `None` to reject a payload — it must never panic.

One limit is inherent to the GDScript path: `send_event` is a `#[func]` on `BevyApp`, so a script handler that fires it *in response to a signal a running Bevy system just emitted* re-enters a node that's already borrowed for the frame, and gdext panics before the bridge runs. Fire GDScript events between frames, not from inside the bridge's own frame.

## Timing

The channel drains once a frame, in `First`. `First` runs in the per-frame prefix — in `_physics_process` when a physics step runs, or in `_process` on a frame with no step — so:

- Enqueue *before* this frame's `physics_process` (from `_input`, for instance) and it's drained at this frame's `First`, visible to `FixedUpdate` and `Update` the same frame.
- Enqueue *mid-frame* (from a running system, or a re-entered signal handler) and the drain has already passed — it lands on the next frame's `First`.

Delivery is quantized to that one render-frame drain, not per fixed step: in a frame with several physics steps, the frame's events all arrive on the first `FixedUpdate` step. If you need a system ordered around delivery, the drain runs in the public `EventBridgeSet::Drain`, so `.after(EventBridgeSet::Drain)` does what you'd expect.

## Signals or `send_event`?

Both arrive at the same `On<T>` observers, so pick by where the event comes from:

- A built-in node signal — a `Button`'s `pressed`, an `Area2D`'s `body_entered` → `GodotSignalsPlugin` + `GodotSignals::connect`.
- A GDScript script raising its own event → `add_godot_event` + `send_event`.
- A Rust node method raising its own event → `send_event(&app, event)`.

## Migrating from the mailbox

Earlier versions shipped a poll-based `GodotMailboxPlugin`: a `FixedFirst` system scanned marked nodes every step, read script-side "pending" fields off each, and wrote a `Message<T>`. The bridge replaces it — a push instead of a poll, decoded once per fire instead of scanned per entity per step.

| Mailbox (poll) | Bridge (push) |
|---|---|
| `impl GodotMailboxMessage` + `drain_from_node` reads node fields | `add_godot_event::<T>("name", \|v\| ...)` decoder; GDScript fires `send_event("name", payload)` |
| `drain_from_node`'s `source: GodotNodeHandle` | pass `{"source": self}` in the payload; the mapper extracts it, the observer resolves node → `Entity` via `NodeEntityIndex` |
| `GodotMailboxPlugin<T, Marker>` per message type | `GodotEventBridgePlugin` once |
| `MessageReader<T>` batch loop | an observer accumulates into a `Resource`, an ordered system drains it (below) |
| O(entities) FFI scan every fixed step | one decode per fire, deferred to the next `First` |

Observers fire per event, not as one batched read. If a consumer wants the mailbox's batch shape — drain everything in an ordered fixed-step system — accumulate in the observer and drain in `FixedUpdate`:

```rust,ignore
#[derive(Resource, Default)]
struct PendingDamage(Vec<PlayerDamageRequest>);

app.add_godot_event::<PlayerDamageRequest>("damage", |v| {
    let dict = v.try_to::<VarDictionary>().ok()?;
    let node = dict.get("source")?.try_to::<Gd<Node>>().ok()?;   // the old `source`
    Some(PlayerDamageRequest {
        target: GodotNodeHandle::from_instance_id(node.instance_id()),
        force: dict.get("amount")?.try_to::<f32>().ok()?,
    })
})
.init_resource::<PendingDamage>()
.add_observer(|ev: On<PlayerDamageRequest>, mut q: ResMut<PendingDamage>| q.0.push(ev.event().clone()))
.add_systems(FixedUpdate, |mut q: ResMut<PendingDamage>, idx: Res<NodeEntityIndex>| {
    for req in q.0.drain(..) {
        let Some(_entity) = idx.get(req.target.instance_id()) else { continue };
        // apply req.force to _entity
    }
});
```

```gdscript
# Was: each enemy set a mailbox field on itself every tick; the drain scanned every Enemy node.
BevyAppSingleton.send_event("damage", {"source": self, "amount": 10.0})
```
