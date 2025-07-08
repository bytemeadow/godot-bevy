use godot::{
    obj::{Bounds, RawGd, bounds::DynMemory},
    prelude::*,
};

/// used to access raw (RawGd) object
struct Gd_<T: GodotClass> {
    raw: RawGd<T>,
}

pub fn maybe_inc_ref<T: GodotClass>(gd: &mut Gd<T>) {
    let gd_: &mut Gd_<T> = unsafe { std::mem::transmute(gd) };
    <Object as Bounds>::DynMemory::maybe_inc_ref(&mut gd_.raw);
}

pub fn maybe_inc_ref_opt<T: GodotClass>(gd: &mut Option<Gd<T>>) {
    if let Some(gd) = gd {
        let gd_: &mut Gd_<T> = unsafe { std::mem::transmute(gd) };
        <Object as Bounds>::DynMemory::maybe_inc_ref(&mut gd_.raw);
    }
}

pub fn maybe_dec_ref<T: GodotClass>(gd: &mut Gd<T>) -> bool {
    let gd_: &mut Gd_<T> = unsafe { std::mem::transmute(gd) };
    unsafe { <Object as Bounds>::DynMemory::maybe_dec_ref(&mut gd_.raw) }
}
