use wasm_bindgen::prelude::*;
use std::{
    future::Future,
    sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}},
};
//use parking_lot::{Mutex, RawMutex};
use half::f16;


#[macro_export]
macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into());
    }
}


#[wasm_bindgen(module = "/helper.js")]
extern "C" {
    pub fn get_canvas_width() -> u32;
    pub fn get_canvas_height() -> u32;
    pub fn cpu_cores() -> u32;
    pub fn get_time_milliseconds() -> f64;
    pub fn get_webgl1_version() -> String;
    pub fn get_webgl2_version() -> String;
    pub fn float2half(x: f32) -> u32;
    pub fn pack_half_2x16_js(x: f32, y: f32) -> u32;
}


/// Enable better error messages if our code ever panics
pub fn set_panic_hook() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}


/// Sets error flag and message for an egui window
#[inline(always)]
pub fn set_error_for_egui(flag: &Arc<AtomicBool>, msg: &Arc<Mutex<String>>, s: String) {
    flag.store(true, Ordering::Relaxed);
    {
        let mut mutex = msg.lock().unwrap();
        *mutex += s.as_str();
    }
}


/// Executes an asyncs Future on the current thread
#[inline(always)]
pub fn execute_future<F: Future<Output = ()> + 'static>(f: F) {
    wasm_bindgen_futures::spawn_local(f);
}


/// Transmutes a slice
#[inline(always)]
pub fn transmute_slice<S, T>(slice: &[S]) -> &[T] {
    let ptr = slice.as_ptr() as *const T;
    let len = std::mem::size_of_val(slice) / std::mem::size_of::<T>();
    unsafe { std::slice::from_raw_parts(ptr, len) }
}


/// Transmutes a mutable slice
#[inline(always)]
pub fn transmute_slice_mut<S, T>(slice: &mut [S]) -> &mut [T] {
    let ptr = slice.as_mut_ptr() as *mut T;
    let len = std::mem::size_of_val(slice) / std::mem::size_of::<T>();
    unsafe { std::slice::from_raw_parts_mut(ptr, len) }
}


/// Packs two f32s as two f16s combined together
#[inline(always)]
pub fn pack_half_2x16(x: f32, y: f32) -> u32 {
    let x_half = f16::from_f32(x);
    let y_half = f16::from_f32(y);
    let result = u32::from(x_half.to_bits()) | (u32::from(y_half.to_bits()) << 16);
    result// & 0xFFFFFFFF
}


/// Check if a float is zero
#[inline(always)]
pub fn is_float_zero(x: f32, threshold: f32) -> bool {
    return x.abs() < threshold;
}


/// Check if two floats are equal
#[inline(always)]
pub fn are_floats_equal(x: f32, y: f32, threshold: f32) -> bool {
    return is_float_zero(x-y, threshold);
}


/*
// TODO
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
*/
