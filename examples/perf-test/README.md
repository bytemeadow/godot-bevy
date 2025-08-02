# Particle Rain Performance Benchmark

This example demonstrates the performance characteristics of **godot-bevy** (Rust + ECS) compared to pure Godot (GDScript) for simple entity management and transform synchronization tasks.

## What This Benchmark Tests

This benchmark focuses on measuring the overhead of transform synchronization between Bevy's ECS and Godot's scene tree by using a simple particle rain simulation that maximizes transform updates while minimizing computational complexity.

### Pure Godot Implementation (GDScript)
- **Language**: GDScript
- **Architecture**: Traditional approach with Node2D instances stored in arrays
- **Physics**: Simple gravity + velocity updates
- **Transform Updates**: Direct Node2D.position assignments
- **Characteristics**: Single-threaded, interpreted language, direct scene tree access

### godot-bevy Implementation (Rust + ECS)
- **Language**: Rust (compiled)
- **Architecture**: Entity Component System with Bevy
- **Physics**: Same gravity + velocity logic as GDScript version
- **Transform Updates**: Transform2D → Transform sync → Godot scene tree
- **Characteristics**: Compiled performance, but additional sync overhead

## Particle Rain Algorithm

Both implementations use identical physics simulation:

1. **Gravity**: Constant downward acceleration (200 px/s²)
2. **Velocity Bounds**: Fall speed clamped between 50-300 px/s
3. **Horizontal Drift**: Random horizontal movement (±50 px/s)
4. **Wraparound**: Particles reset to top when they fall off bottom
5. **Randomization**: New particles get random colors and positions

### Performance-Critical Operations

- **Transform Synchronization**: Every particle needs position update every frame
- **Entity Lifecycle**: Spawning/despawning particles dynamically
- **Memory Access**: Iterating through thousands of entities
- **Scene Tree Updates**: Updating Node2D positions in Godot

## Using the Benchmark

### UI Controls

- **Implementation Selector**: Switch between "Godot (GDScript)" and "godot-bevy (Rust + ECS)"
- **Particle Count Slider**: Adjust from 50 to 50,000+ particles
- **Start/Stop**: Control benchmark execution
- **Reset Metrics**: Clear performance measurements

### Performance Metrics

The benchmark tracks:
- **Current FPS**: Real-time frame rate
- **Average FPS**: Rolling average over 5 seconds
- **Min/Max FPS**: Performance extremes
- **Active Particles**: Current entity count

## Expected Results

### Performance Characteristics

This benchmark specifically tests **transform synchronization overhead**. Expected results:

| Particle Count | Godot (GDScript) | godot-bevy (Rust) | Analysis |
|----------------|------------------|-------------------|----------|
| 1,000         | ~145 FPS         | ~145 FPS          | Similar - low sync overhead |
| 5,000         | ~145 FPS         | ~120 FPS          | Sync overhead becomes visible |
| 10,000        | ~120 FPS         | ~95 FPS           | Sync overhead significant |
| 20,000        | ~60 FPS          | ~45 FPS           | Both limited by transform updates |

### Why This Test Is Important

Unlike complex algorithms (like boids), this test isolates the **pure overhead** of using godot-bevy for simple tasks:

1. **Transform Sync Cost**: Shows the price of the Transform2D → Transform → Godot pipeline
2. **ECS Overhead**: Measures if ECS adds overhead for simple operations
3. **Baseline Comparison**: Establishes when godot-bevy is worth the complexity

## Benchmark Methodology

### Fair Comparison Principles

1. **Identical Logic**: Both implementations use exactly the same physics calculations
2. **Same Visual Effects**: Both generate random colors and use identical scenes
3. **Same Update Patterns**: Both spawn/despawn entities at the same rate
4. **Same Memory Patterns**: Both use arrays for bulk operations
5. **Clean Measurement**: No debug logging or unnecessary overhead

### Key Difference: Transform Updates

- **GDScript**: Direct `node.position = new_pos` assignment
- **godot-bevy**: `Transform2D` → `Transform` sync → `node.position` update

## Implementation Details

### Godot Implementation (`scripts/godot_boids.gd`)
- Uses `Node2D` instances with direct position updates
- Arrays store positions and velocities for cache efficiency  
- Simple physics loop with immediate visual updates
- Spawns/despawns up to 50 particles per frame

### godot-bevy Implementation (`rust/src/particle_rain.rs`)
- ECS entities with `Particle`, `Velocity`, and `Transform2D` components
- **No spatial data structures** - just simple physics
- Transform synchronization through godot-bevy's sync systems
- Same spawn/despawn logic as GDScript version

## Conclusion

This benchmark demonstrates the **baseline cost** of using godot-bevy for simple entity management tasks. It helps answer the question: "When is the complexity of godot-bevy worth it?"

**Key Insights:**
- **Simple Tasks**: GDScript may be faster for basic entity management
- **Transform Overhead**: godot-bevy has measurable sync costs for high entity counts
- **Complexity Threshold**: More complex logic (AI, physics, algorithms) will favor godot-bevy
- **Design Decision**: Choose based on computational complexity, not just entity count

This serves as a foundation for understanding godot-bevy performance characteristics and making informed architectural decisions.