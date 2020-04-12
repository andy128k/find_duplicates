use glib::glib_sys::gpointer;
use glib::gobject_sys::{g_object_get_data, g_object_set_data_full};
use glib::{translate::ToGlibPtr, Object};
use std::ffi::CString;

extern "C" fn drop_value<T>(ptr: gpointer) {
    if ptr.is_null() {
        return;
    }
    let value: Box<T> = unsafe { Box::from_raw(ptr as *mut T) };
    std::mem::drop(value)
}

pub fn object_set_data<T>(obj: &Object, key: &str, value: Box<T>) {
    let ckey = CString::new(key).unwrap();
    let ptr: gpointer = Box::leak(value) as *mut T as gpointer;
    unsafe {
        g_object_set_data_full(
            obj.to_glib_none().0,
            ckey.as_ptr(),
            ptr,
            Some(drop_value::<T>),
        );
    }
}

pub fn object_get_data<'o, T>(obj: &'o Object, key: &str) -> Option<&'o T> {
    let ckey = CString::new(key).ok()?;
    let ptr = unsafe { g_object_get_data(obj.to_glib_none().0, ckey.as_ptr()) };
    if ptr.is_null() {
        return None;
    }
    let value: &T = unsafe { &*(ptr as *const T) };
    Some(value)
}
