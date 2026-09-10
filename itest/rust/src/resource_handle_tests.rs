use godot::{classes::Resource, prelude::*};
use godot_bevy::prelude::GodotResourceHandle;
use godot_bevy_test::prelude::*;

#[itest]
fn test_resource_handle_keeps_resource_alive(_: &TestContext) {
    let resource = Resource::new_gd();
    let id = resource.instance_id();
    assert_eq!(resource.get_reference_count(), 1);

    let mut handle = GodotResourceHandle::new(resource);
    assert!(id.lookup_validity());

    let resource = handle.get();
    assert_eq!(resource.instance_id(), id);
    assert_eq!(resource.get_reference_count(), 2);
    drop(resource);
    assert!(id.lookup_validity());

    drop(handle);
    assert!(!id.lookup_validity());
}

#[itest]
fn test_resource_handle_clone_owns_one_reference(_: &TestContext) {
    let resource = Resource::new_gd();
    let id = resource.instance_id();
    let handle = GodotResourceHandle::new(resource.clone());
    assert_eq!(resource.get_reference_count(), 2);
    assert_ne!(handle, GodotResourceHandle::new(Resource::new_gd()));

    let mut cloned = handle.clone();
    assert_eq!(handle, cloned);
    assert_eq!(resource.get_reference_count(), 3);

    drop(handle);
    assert!(id.lookup_validity());
    assert_eq!(resource.get_reference_count(), 2);
    drop(resource);
    assert!(id.lookup_validity());

    let resource = cloned
        .try_get()
        .expect("clone must keep the resource alive");
    assert_eq!(resource.instance_id(), id);
    assert_eq!(resource.get_reference_count(), 2);
    drop(resource);

    drop(cloned);
    assert!(!id.lookup_validity());
}

#[itest]
fn test_resource_handle_drop_on_another_thread(_: &TestContext) {
    let resource = Resource::new_gd();
    let id = resource.instance_id();
    let handle = GodotResourceHandle::new(resource);

    std::thread::spawn(move || {
        let mut cloned = handle.clone();
        assert_eq!(handle, cloned);
        assert_eq!(cloned.get().get_reference_count(), 3);

        drop(handle);
        assert_eq!(cloned.get().get_reference_count(), 2);
        drop(cloned);
    })
    .join()
    .expect("resource handles must support cloning and dropping on a worker");

    assert!(!id.lookup_validity());
}

#[itest]
fn test_resource_handle_drop_preserves_external_reference(_: &TestContext) {
    let resource = Resource::new_gd();
    let id = resource.instance_id();
    let handle = GodotResourceHandle::new(resource.clone());
    assert_eq!(resource.get_reference_count(), 2);

    drop(handle);
    assert!(id.lookup_validity());
    assert_eq!(resource.get_reference_count(), 1);

    drop(resource);
    assert!(!id.lookup_validity());
}

#[itest]
fn test_resource_handle_drop_bypasses_script_unreference(_: &TestContext) {
    let mut script = godot::classes::GDScript::new_gd();
    script.set_source_code(
        "extends Resource\n\
         @warning_ignore(\"native_method_override\")\n\
         func unreference() -> bool:\n\
         \treturn false\n",
    );
    assert_eq!(script.reload(), godot::global::Error::OK);
    let mut resource = Resource::new_gd();
    resource.set_script(&script);
    let id = resource.instance_id();
    assert_eq!(resource.get_reference_count(), 1);

    let handle = GodotResourceHandle::new(resource.clone());
    assert_eq!(resource.get_reference_count(), 2);
    drop(handle);
    assert_eq!(resource.get_reference_count(), 1);

    let handle = GodotResourceHandle::new(resource);
    drop(handle);
    assert!(!id.lookup_validity());
}

#[itest]
fn test_resource_handle_concurrent_clones(_: &TestContext) {
    let resource = Resource::new_gd();
    let id = resource.instance_id();
    let handle = GodotResourceHandle::new(resource.clone());
    let (ready, received) = std::sync::mpsc::channel();
    let timeout = std::time::Duration::from_secs(5);

    std::thread::scope(|scope| {
        let mut releases = Vec::new();
        for _ in 0..4 {
            let handle = &handle;
            let ready = ready.clone();
            let (release, released) = std::sync::mpsc::channel();
            releases.push(release);
            scope.spawn(move || {
                let clones: Vec<_> = (0..64).map(|_| handle.clone()).collect();
                ready.send(()).unwrap();
                released.recv_timeout(timeout).unwrap();
                drop(clones);
            });
        }
        for _ in 0..4 {
            received.recv_timeout(timeout).unwrap();
        }
        assert_eq!(resource.get_reference_count(), 258);
        for release in releases {
            release.send(()).unwrap();
        }
    });

    assert_eq!(resource.get_reference_count(), 2);
    drop(handle);
    assert_eq!(resource.get_reference_count(), 1);
    drop(resource);
    assert!(!id.lookup_validity());
}

mod shutdown {
    // Compile the production source here to tear down its private owner without a public test hook.
    include!("../../../godot-bevy/src/interop/godot_resource_handle.rs");

    use godot::obj::NewGd;
    use godot_bevy_test::prelude::*;

    fn expired_handle() -> GodotResourceHandle {
        let resource = Resource::new_gd();
        let id = resource.instance_id();
        let handle = GodotResourceHandle::new(resource.clone());
        assert_eq!(resource.get_reference_count(), 2);

        Gd::<Object>::from_instance_id(handle.owner_id).free();
        assert!(!handle.owner_id.lookup_validity());
        assert_eq!(resource.get_reference_count(), 1);
        drop(resource);
        assert!(!id.lookup_validity());
        handle
    }

    #[itest]
    fn test_resource_handle_shutdown_clone_and_drop(_: &TestContext) {
        let handle = expired_handle();
        let clone_result = std::panic::catch_unwind(|| handle.clone());
        let drop_result = std::panic::catch_unwind(|| drop(handle));
        assert!(
            drop_result.is_ok(),
            "dropping an expired handle must not panic"
        );
        let mut cloned = clone_result.expect("cloning an expired handle must not panic");
        assert!(cloned.try_get().is_none());
        assert!(std::panic::catch_unwind(move || cloned.get()).is_err());
    }

    #[itest]
    fn test_resource_handle_shutdown_drop_during_unwind(_: &TestContext) {
        let handle = expired_handle();
        let result = std::panic::catch_unwind(move || {
            let _handle = handle;
            panic!("resource_handle_unwind_sentinel");
        });
        assert_eq!(
            result.unwrap_err().downcast_ref::<&str>(),
            Some(&"resource_handle_unwind_sentinel")
        );
    }
}
