//! # Godot-Bevy Testability
//!
//! Testing framework for Bevy ECS systems with embedded Godot runtime.
//! This allows you to test Bevy systems that interact with Godot nodes and resources.

pub mod helpers;

pub use godot_testability_runtime::prelude::*;
pub use godot_testability_runtime::runtime::UserCallbacks;
pub use helpers::{BevyGodotTestContextExt, TestEnvironment};

use bevy::app::App;
use std::sync::mpsc::channel;

/// A test context that provides access to both Bevy App and Godot SceneTree
pub struct BevyGodotTestContext {
    /// The Bevy App instance
    pub app: App,
    /// The Godot SceneTree (as raw pointer - users convert to their type)
    pub scene_tree_ptr: *mut std::ffi::c_void,
}

// Safety: We ensure single-threaded access in tests
unsafe impl Send for BevyGodotTestContext {}

impl BevyGodotTestContext {
    /// Initialize resources that godot-bevy plugins expect to exist
    /// Mimics what BevyApp::ready() does in the normal runtime
    pub fn initialize_godot_bevy_resources(&mut self) {
        use godot_bevy::plugins::{
            collisions::CollisionEventReader,
            core::PhysicsDelta,
            input::InputEventReader,
            scene_tree::SceneTreeEventReader,
            signals::{GodotSignalReader, GodotSignalSender},
        };

        // Register signal system - creates channels for signal communication
        let (signal_sender, signal_receiver) = channel();
        self.app
            .insert_non_send_resource(GodotSignalSender(signal_sender));
        self.app
            .insert_non_send_resource(GodotSignalReader(signal_receiver));

        // Register scene tree event system
        let (_scene_sender, scene_receiver) = channel();
        self.app
            .insert_non_send_resource(SceneTreeEventReader(scene_receiver));
        // Note: In real runtime, SceneTreeWatcher sends to _scene_sender
        // For tests, we can manually send events if needed

        // Register input event system
        let (_input_sender, input_receiver) = channel();
        self.app
            .insert_non_send_resource(InputEventReader(input_receiver));
        // Note: In real runtime, GodotInputWatcher sends to _input_sender

        // Register collision event system
        let (_collision_sender, collision_receiver) = channel();
        self.app
            .insert_non_send_resource(CollisionEventReader(collision_receiver));
        // Note: In real runtime, CollisionWatcher sends to _collision_sender

        // Initialize physics delta resource
        self.app.init_resource::<PhysicsDelta>();
    }
}

/// Type alias for Bevy-Godot test functions
pub type BevyGodotTestFn = fn(&mut BevyGodotTestContext) -> TestResult<()>;

/// Macro for creating a test main function that runs Bevy systems with Godot runtime.
///
/// # Example
///
/// ```rust,ignore
/// use godot_bevy_testability::*;
/// use bevy::prelude::*;
/// use godot::prelude::*;
///
/// fn test_bevy_system(ctx: &mut BevyGodotTestContext) -> TestResult<()> {
///     // Add a Bevy system
///     ctx.app.add_systems(Update, my_system);
///
///     // Convert scene tree pointer to Godot type
///     let scene_tree = unsafe {
///         // User handles conversion based on their godot version
///         convert_scene_tree_ptr(ctx.scene_tree_ptr)
///     };
///
///     // Run the Bevy app for one frame
///     ctx.app.update();
///
///     Ok(())
/// }
///
/// bevy_godot_test_main! {
///     test_bevy_system,
/// }
/// ```
#[macro_export]
macro_rules! bevy_godot_test_main {
    ($($test_name:ident),* $(,)?) => {
        fn main() {
            use godot_bevy_testability::{BevyGodotTestContext, UserCallbacks, GodotRuntime, RuntimeConfig};
            use godot_testability_runtime::__private::libtest_mimic::{Arguments, Trial};
            use std::sync::{Arc, Mutex};

            // Parse command line arguments
            let mut args = Arguments::from_args();
            args.test_threads = Some(1); // Force single-threaded for Godot

            // Collect test functions
            let tests: Vec<(&str, fn(&mut BevyGodotTestContext) -> godot_bevy_testability::TestResult<()>)> = vec![
                $((stringify!($test_name), $test_name),)*
            ];

            if tests.is_empty() {
                eprintln!("No tests defined!");
                std::process::exit(1);
            }

            // Storage for test results
            let test_results = Arc::new(Mutex::new(Vec::new()));
            let test_results_clone = test_results.clone();

            // Filter tests
            let filter = args.filter.as_ref().map(|s| s.as_str());
            let filtered_tests: Vec<_> = tests
                .into_iter()
                .filter(|(name, _)| filter.map_or(true, |f| name.contains(f)))
                .collect();

            if filtered_tests.is_empty() {
                println!("No tests to run with filter: {:?}", filter);
                std::process::exit(0);
            }

            // Create callbacks for godot-rust integration
            let callbacks = UserCallbacks {
                initialize_ffi: |get_proc_addr, library| {
                    unsafe {
                        type GetProcAddrFn = unsafe extern "C" fn(*const i8) -> Option<unsafe extern "C" fn()>;
                        let get_proc_addr_fn = std::mem::transmute::<*const std::ffi::c_void, GetProcAddrFn>(get_proc_addr);
                        let library_ptr = library as *mut ::godot::sys::__GdextClassLibrary;
                        let config = ::godot::sys::GdextConfig::new(false);
                        ::godot::sys::initialize(Some(get_proc_addr_fn), library_ptr, config);
                    }
                    Ok(())
                },
                load_class_method_table: |level| {
                    let init_level = match level {
                        0 => ::godot::init::InitLevel::Core,
                        1 => ::godot::init::InitLevel::Servers,
                        2 => ::godot::init::InitLevel::Scene,
                        3 => ::godot::init::InitLevel::Editor,
                        _ => return,
                    };
                    unsafe {
                        ::godot::sys::load_class_method_table(init_level);
                    }
                },
                register_classes: Some(|level| {
                    let init_level = match level {
                        0 => ::godot::init::InitLevel::Core,
                        1 => ::godot::init::InitLevel::Servers,
                        2 => ::godot::init::InitLevel::Scene,
                        3 => ::godot::init::InitLevel::Editor,
                        _ => return,
                    };
                    // Use the same path as the bevy_app macro
                    ::godot::private::class_macros::registry::class::auto_register_classes(init_level);
                }),
            };

            // Run tests in Godot runtime
            let config = RuntimeConfig {
                headless: true,
                verbose: false,
                custom_args: vec![],
            };

            let runtime_result = GodotRuntime::run_godot(config, callbacks, move |scene_tree_ptr| {
                println!("\n🧪 Running {} Bevy-Godot tests...\n", filtered_tests.len());

                for (name, test_fn) in &filtered_tests {
                    print!("test {} ... ", name);

                    // Create a fresh Bevy App for each test
                    let app = ::bevy::app::App::new();

                    let mut ctx = BevyGodotTestContext {
                        app,
                        scene_tree_ptr,
                    };

                    // Initialize godot-bevy resources that plugins expect
                    // Note: GodotBaseCorePlugin will add MinimalPlugins itself
                    ctx.initialize_godot_bevy_resources();

                    match test_fn(&mut ctx) {
                        Ok(()) => {
                            println!("ok");
                            test_results_clone
                                .lock()
                                .unwrap()
                                .push((name.to_string(), Ok(())));
                        }
                        Err(e) => {
                            println!("FAILED");
                            println!("  Error: {}", e);
                            test_results_clone
                                .lock()
                                .unwrap()
                                .push((name.to_string(), Err(format!("{}", e))));
                        }
                    }
                }

                println!();

                // Convert and quit
                unsafe {
                    let obj_ptr = scene_tree_ptr as ::godot::sys::GDExtensionObjectPtr;
                    let mut scene_tree = ::godot::prelude::Gd::<::godot::prelude::SceneTree>::from_sys_init_opt(|ptr| {
                        *(ptr as *mut ::godot::sys::GDExtensionObjectPtr) = obj_ptr;
                    })
                    .expect("Failed to create SceneTree from pointer");
                    scene_tree.quit();
                }

                Ok(())
            });

            // Handle runtime errors
            if let Err(e) = runtime_result {
                eprintln!("Runtime error: {}", e);
                std::process::exit(1);
            }

            // Create trials for reporting
            let results = test_results.lock().unwrap().clone();
            let trials: Vec<Trial> = results
                .into_iter()
                .map(|(name, result)| {
                    Trial::test(name.clone(), move || {
                        result.clone().map_err(|e| godot_testability_runtime::__private::libtest_mimic::Failed::from(e))
                    })
                })
                .collect();

            // Run the trials and exit
            godot_testability_runtime::__private::libtest_mimic::run(&args, trials).exit();
        }
    };
}
