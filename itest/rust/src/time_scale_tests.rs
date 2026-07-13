//! Engine.time_scale scales Time<Virtual> (the Update clock) but not Time<Real>. It's
//! global process state, so every test resets it via a drop guard. Assertions use tolerant
//! ratios over multi-frame windows to survive the harness's +/-1 frame slop.

use bevy::prelude::*;
use godot::obj::Singleton;
use godot_bevy_test::prelude::*;

/// Restores Engine.time_scale to 1.0 when dropped -- runs even if an assertion
/// unwinds, keeping the global scale from leaking across tests.
struct ResetTimeScale;

impl Drop for ResetTimeScale {
    fn drop(&mut self) {
        godot::classes::Engine::singleton().set_time_scale(1.0);
    }
}

fn set_time_scale(scale: f64) {
    godot::classes::Engine::singleton().set_time_scale(scale);
}

#[derive(Resource, Default)]
struct RatioProbe {
    virt_sum: f64,
    real_sum: f64,
    frames: u32,
}

/// Accumulate per-frame Virtual/Real deltas, skipping frames near the Time<Virtual>
/// max_delta clamp (0.25s) that would distort the ratio. Below the clamp Virtual.delta ==
/// Real.delta * relative_speed, so the ratio of sums is the scale despite wall-clock jitter.
fn sample_ratio(virt: Res<Time<Virtual>>, real: Res<Time<Real>>, mut probe: ResMut<RatioProbe>) {
    let r = real.delta_secs_f64();
    if r > 0.0 && r < 0.1 {
        probe.real_sum += r;
        probe.virt_sum += virt.delta_secs_f64();
        probe.frames += 1;
    }
}

/// Drive `frames` render frames at `scale` and return (sum Virtual.delta,
/// sum Real.delta, clean-frame count). Resets time_scale to 1.0 before returning.
async fn measure_ratio(ctx: &TestContext, scale: f64, frames: u32) -> (f64, f64, u32) {
    let mut app = TestApp::new(ctx, |app| {
        app.init_resource::<RatioProbe>();
        app.add_systems(Update, sample_ratio);
    })
    .await;

    let _reset = ResetTimeScale;
    set_time_scale(scale);
    app.updates(3).await; // let the new scale settle
    app.with_world_mut(|w| *w.resource_mut::<RatioProbe>() = RatioProbe::default());
    app.updates(frames).await;

    let out = app.with_world(|w| {
        let p = w.resource::<RatioProbe>();
        (p.virt_sum, p.real_sum, p.frames)
    });
    app.cleanup().await;
    out
}

/// time_scale = 0.5 halves the Update clock: sum(Virtual.delta)/sum(Real.delta) ~= 0.5.
/// Real stays truthful -- its elapsed keeps pace with wall time, not the halved scale.
#[itest(async)]
fn test_time_scale_half_scales_virtual_not_real(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx = ctx.clone();
    godot::task::spawn(async move {
        let mut app = TestApp::new(&ctx, |app| {
            app.init_resource::<RatioProbe>();
            app.add_systems(Update, sample_ratio);
        })
        .await;

        let _reset = ResetTimeScale;
        set_time_scale(0.5);
        app.updates(3).await;
        app.with_world_mut(|w| *w.resource_mut::<RatioProbe>() = RatioProbe::default());

        let real0 = app.with_world(|w| w.resource::<Time<Real>>().elapsed_secs_f64());
        let wall = std::time::Instant::now();
        app.updates(30).await;
        let wall_elapsed = wall.elapsed().as_secs_f64();

        let (virt_sum, real_sum, frames, real1) = app.with_world(|w| {
            let p = w.resource::<RatioProbe>();
            (
                p.virt_sum,
                p.real_sum,
                p.frames,
                w.resource::<Time<Real>>().elapsed_secs_f64(),
            )
        });

        assert!(frames >= 10, "need clean frames to average, got {frames}");
        let ratio = virt_sum / real_sum;
        assert!(
            (ratio - 0.5).abs() < 0.06,
            "Virtual/Real delta ratio {ratio} should track time_scale 0.5"
        );

        // Real is unscaled: its elapsed keeps pace with the test's own wall clock.
        let real_elapsed = real1 - real0;
        assert!(
            real_elapsed > 0.6 * wall_elapsed,
            "Real.elapsed grew {real_elapsed}s over {wall_elapsed}s wall -- Real must not be scaled"
        );

        app.cleanup().await;
    })
}

/// time_scale = 0 freezes the Update clock -- Virtual.delta is 0 every frame -- while
/// Real.elapsed keeps climbing, proving Real ignores time_scale entirely.
#[itest(async)]
fn test_time_scale_zero_freezes_virtual_real_climbs(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx = ctx.clone();
    godot::task::spawn(async move {
        #[derive(Resource, Default)]
        struct MaxVirtDelta(f64);

        let mut app = TestApp::new(&ctx, |app| {
            app.init_resource::<MaxVirtDelta>();
            app.add_systems(
                Update,
                |virt: Res<Time<Virtual>>, mut m: ResMut<MaxVirtDelta>| {
                    m.0 = m.0.max(virt.delta_secs_f64());
                },
            );
        })
        .await;

        let _reset = ResetTimeScale;
        set_time_scale(0.0);
        app.updates(3).await;
        app.with_world_mut(|w| w.resource_mut::<MaxVirtDelta>().0 = 0.0);

        let real0 = app.with_world(|w| w.resource::<Time<Real>>().elapsed_secs_f64());
        app.updates(30).await;
        let (max_virt, real1) = app.with_world(|w| {
            (
                w.resource::<MaxVirtDelta>().0,
                w.resource::<Time<Real>>().elapsed_secs_f64(),
            )
        });

        assert_eq!(
            max_virt, 0.0,
            "Virtual.delta must be 0 on every frame at time_scale 0"
        );
        assert!(
            real1 > real0,
            "Real.elapsed must keep climbing while Virtual is frozen ({real0} -> {real1})"
        );

        app.cleanup().await;
    })
}

/// time_scale = 4.0 fast-forwards the Update clock: ratio ~= 4.0.
#[itest(async)]
fn test_time_scale_four_fast_forwards_virtual(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx = ctx.clone();
    godot::task::spawn(async move {
        let (virt_sum, real_sum, frames) = measure_ratio(&ctx, 4.0, 30).await;
        assert!(frames >= 10, "need clean frames to average, got {frames}");
        let ratio = virt_sum / real_sum;
        assert!(
            (ratio - 4.0).abs() < 0.4,
            "Virtual/Real delta ratio {ratio} should track time_scale 4.0"
        );
    })
}

/// time_scale = 1.0 is a no-op: Virtual.delta ~= Real.delta.
#[itest(async)]
fn test_time_scale_one_is_noop(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx = ctx.clone();
    godot::task::spawn(async move {
        let (virt_sum, real_sum, frames) = measure_ratio(&ctx, 1.0, 30).await;
        assert!(frames >= 10, "need clean frames to average, got {frames}");
        let ratio = virt_sum / real_sum;
        assert!(
            (ratio - 1.0).abs() < 0.02,
            "Virtual/Real delta ratio {ratio} should be ~1.0 at the default scale"
        );
    })
}

/// Non-finite and negative time_scale must not tear the app down (unguarded they panic
/// set_relative_speed_f64 or the driver's Duration::from_secs_f64). The counter keeps advancing.
#[itest(async)]
fn test_pathological_time_scale_does_not_tear_down(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx = ctx.clone();
    godot::task::spawn(async move {
        let counter = Counter::new();
        let c = counter.clone();

        let mut app = TestApp::new(&ctx, move |app| {
            #[derive(Resource)]
            struct Ticks(Counter);

            app.insert_resource(Ticks(c.clone()));
            app.add_systems(Update, |t: Res<Ticks>| t.0.increment());
        })
        .await;

        let _reset = ResetTimeScale;
        for scale in [f64::NAN, -2.0, f64::INFINITY] {
            let before = counter.get();
            set_time_scale(scale);
            app.updates(5).await;
            assert!(
                counter.get() > before,
                "app must keep stepping at time_scale {scale}; Update counter stalled at {before}"
            );
        }

        app.cleanup().await;
    })
}
