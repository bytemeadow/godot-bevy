# Boids Performance Benchmark

This example demonstrates the performance benefits of using **godot-bevy** (Rust + ECS) compared to pure Godot (GDScript) for computationally intensive tasks like boids simulation.

> 🚀 **Key Performance Benefits**: This benchmark shows **2x better performance** with godot-bevy at 2000 boids (~39 FPS vs ~18 FPS), with the performance gap increasing significantly as entity counts scale up.

## What This Benchmark Tests

### Pure Godot Implementation (GDScript)
- **Language**: GDScript
- **Architecture**: Traditional object-oriented approach with Node2D instances
- **Neighbor Finding**: Spatial grid optimization
- **Behaviors**: Separation, alignment, cohesion, boundary avoidance
- **Limitations**: Single-threaded, interpreted language overhead

### godot-bevy Implementation (Rust + ECS)
- **Language**: Rust (compiled, zero-cost abstractions)
- **Architecture**: Entity Component System with Bevy
- **Neighbor Finding**: **bevy_spatial KDTree2** with k_nearest_neighbour optimization
- **Transform Sync**: **Hybrid batching** for efficient Transform2D synchronization
- **Behaviors**: Separation, alignment, cohesion, boundary avoidance
- **Visual Effects**: **Random color generation** matching GDScript variety
- **Advantages**: Compiled performance, memory efficiency, CPU cache-friendly data layout

## Boids Algorithm

Both implementations use the classic boids algorithm with four behaviors:

1. **Separation**: Avoid crowding neighbors
2. **Alignment**: Steer towards average heading of neighbors  
3. **Cohesion**: Move towards center of mass of neighbors
4. **Boundary Avoidance**: Steer away from world edges (both use 100px margin with 2x force strength)

**Boundary Handling**: Both implementations also use wraparound boundaries as a fallback - if a boid still reaches an edge despite avoidance forces, it wraps to the opposite side (toroidal world).

### Performance-Critical Operations

- **Neighbor Finding**: Spatial grid (75x75px cells) vs **bevy_spatial KDTree2** with k_nearest_neighbour
- **Transform Synchronization**: Batch vs individual Godot scene updates
- **Vector Math**: Hundreds of vector calculations per frame
- **Memory Access**: Cache efficiency becomes critical with many entities
- **Update Loops**: Processing thousands of entities each frame

## Running the Benchmark

### Prerequisites

1. **Rust toolchain** (for compiling the godot-bevy implementation)
2. **Godot 4.2+** 
3. **Build the project**:
   ```bash
   cd rust
   cargo build --release
   ```

### Launch Options

**Option 1: From Godot Editor**
1. Open `godot/project.godot` in Godot
2. Run the main scene
3. Use the UI controls to switch implementations and adjust settings

**Option 2: From Command Line**
```bash
# From the rust directory
cargo run
```

## Using the Benchmark

### UI Controls

- **Implementation Selector**: Switch between "Godot (GDScript)" and "godot-bevy (Rust + ECS)"
- **Boid Count Slider**: Adjust from 50 to 2000+ boids
- **Start/Stop**: Control benchmark execution
- **Reset Metrics**: Clear performance measurements

### Performance Metrics

The benchmark tracks:
- **Current FPS**: Real-time frame rate
- **Average FPS**: Rolling average over 5 seconds
- **Min/Max FPS**: Performance extremes
- **Active Boids**: Current entity count

## Expected Results

### Performance Characteristics

| Boid Count | Godot (GDScript) | godot-bevy (Rust) | Improvement |
|------------|------------------|-------------------|-------------|
| 100        | ~60 FPS          | ~60 FPS           | Minimal     |
| 500        | ~50 FPS          | ~60 FPS           | 1.2x        |
| 1000       | ~35 FPS          | ~55 FPS           | 1.6x        |
| **2000**   | **~18 FPS**      | **~39 FPS**       | **2.2x**    |

> **Note**: Actual results measured on M1 MacBook Pro. The **Rust implementation is 13.4x faster** in pure algorithm execution (0.38ms vs 22.4ms force calculation), with the remaining time spent on transform synchronization and rendering.

### Why godot-bevy Performs Better

1. **Different Spatial Structures**: **bevy_spatial KDTree2** with k_nearest_neighbour (50-entity cap) vs spatial grid (75x75px cells)
2. **Compiled vs Interpreted**: Rust compiles to native machine code, GDScript is interpreted  
3. **Memory Layout**: ECS components are stored contiguously in memory (cache-friendly)
4. **Efficient Transform Sync**: **Hybrid batching** reduces Godot API call overhead
5. **Zero-Cost Abstractions**: Rust's ownership system eliminates garbage collection overhead
6. **SIMD Optimizations**: Rust compiler can auto-vectorize mathematical operations

## Architecture Comparison

### Godot (Traditional OOP)
```
Node2D (Boid)
├── Polygon2D (Visual)
├── Meta: velocity
├── Meta: acceleration
└── Manual neighbor queries
```

### godot-bevy (ECS)
```
Entity
├── Transform2D Component (hybrid batched sync)
├── Boid Component (marker for spatial tracking)
├── Velocity Component
├── NeedsColorization Component (temporary)
└── bevy_spatial KDTree2 for neighbor queries
```

## Benchmark Methodology

### Fair Comparison Principles

1. **Identical Algorithms**: Both implementations use the four boids behaviors (separation, alignment, cohesion, boundary avoidance)
2. **Same Visual Effects**: Both generate random colors for boids and use identical scene structure
3. **Clean Logging**: Removed debug logging and timing overhead for accurate measurements
4. **Same Update Rate**: Both update at consistent intervals using native scheduling
5. **Identical Parameters**: Same max_speed, max_force, perception_radius, separation_radius, boundary_weight
6. **Same Boundary Behavior**: Both use boundary avoidance forces (100px margin, 2x strength) plus wraparound fallback

### Measurements

- Performance measured over 5-second rolling windows
- Excludes startup/initialization time
- Tests run at various boid counts to show scaling behavior
- Multiple runs recommended for statistical significance

## Implementation Details

### Godot Implementation (`scripts/godot_boids.gd`)
- Uses `Node2D` instances for each boid with random color modulation
- Spatial grid hash map for neighbor optimization
- Vector math using Godot's built-in `Vector2`
- Single-threaded update loop in `_process()`
- Optimized with pre-allocated PackedVector2Array data structures

### godot-bevy Implementation (`rust/src/bevy_boids.rs`)
- ECS entities with `Boid`, `Velocity`, and `Transform2D` components  
- **bevy_spatial AutomaticUpdate** plugin with KDTree2 for spatial queries
- **k_nearest_neighbour** with 50-entity cap for optimized neighbor finding
- **Hybrid transform batching** for efficient Godot scene synchronization
- **Deferred colorization** system matching GDScript visual variety
- Systems run in Bevy's `Update` schedule with proper ordering

## Key Optimizations Implemented

### bevy_spatial Integration
- **KDTree2 spatial data structure** for O(log n) neighbor queries
- **AutomaticUpdate plugin** maintains spatial tree automatically  
- **k_nearest_neighbour** with 50-entity cap prevents performance spikes
- **16ms update frequency** (roughly 60 FPS) for spatial tree refresh

### Transform Synchronization Batching
- **Hybrid batching system** for both 2D and 3D transforms
- **Automatic threshold detection**: Batches when ≥10 entities need updates
- **Individual updates** for low-frequency changes to minimize overhead
- **Performance metrics tracking** with configurable batching parameters
- **Fallback support** with option to disable batching entirely

### Clean Performance Measurement  
- **Removed debug logging** that was affecting performance measurements
- **Eliminated timing overhead** from microsecond-level measurements
- **Simplified performance tracking** to essential FPS reporting only
- **Fixed UI synchronization** so boid count displays correctly
- **Fixed restart behavior** ensuring simulation can be stopped and restarted reliably

### Visual Parity
- **Random color generation** matching GDScript behavior exactly
- **Deferred colorization** using marker components for proper timing
- **Scene structure compatibility** supporting Sprite, Triangle, or direct Node2D modulation
- **Identical boundary behavior** (both use avoidance forces + wraparound fallback)

## Extending the Benchmark

### Adding More Implementations
You could extend this benchmark to compare:
- **C# with Godot**: Compiled language vs GDScript
- **GDExtension C++**: Native performance comparison
- **WebAssembly**: Browser performance testing

### Advanced Optimizations
- **Parallel Processing**: Utilize Bevy's `par_iter()` for system parallelization
- **Compute Shaders**: GPU-accelerated boids on graphics hardware
- **SIMD Instructions**: Hand-optimized vector operations
- **Memory Pooling**: Reduce allocation overhead

### Profiling Integration
- **Bevy Diagnostic Plugin**: Built-in performance tracking
- **Godot Profiler**: Memory and CPU usage analysis
- **Custom Metrics**: Algorithm-specific measurements

## Performance Tips

### For Godot Users
1. Use spatial partitioning for neighbor queries
2. Minimize `get_meta()` calls (cache values)
3. Prefer packed arrays for bulk operations
4. Consider GDScript compilation flags

### For godot-bevy Users
1. Use `Query` filters to reduce iteration
2. Leverage Bevy's change detection
3. Group related components for cache efficiency
4. Profile with `bevy/dynamic_linking` for faster iteration

## Troubleshooting

### Low Performance Issues
- **Debug vs Release**: Ensure `cargo build --release` for Rust
- **V-Sync**: Disable for accurate FPS measurement
- **Background Processes**: Close other applications during testing
- **Hardware Limits**: GPU-bound rendering vs CPU-bound simulation


### Build Issues
- **Missing Dependencies**: Check Rust toolchain installation
- **Godot Version**: Requires Godot 4.2+ for best compatibility
- **Extension Loading**: Verify `rust.gdextension` is properly configured

## Conclusion

This benchmark demonstrates that **godot-bevy provides significant performance benefits** for CPU-intensive game logic, particularly as complexity scales. While Godot excels at game editor features and rapid prototyping, godot-bevy offers the performance characteristics needed for demanding simulations, large-scale multiplayer games, and complex AI systems.

The performance gap becomes most apparent with:
- **High entity counts** (1000+ objects)
- **Complex per-entity calculations** (physics, AI, pathfinding)
- **Frequent data access patterns** (neighbor queries, spatial partitioning)

For games requiring maximum performance in these areas, godot-bevy provides a compelling solution that combines Godot's excellent tooling with Rust's performance characteristics.