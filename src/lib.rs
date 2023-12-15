#![allow(
    unused_assignments,
    unused_variables,
    dead_code,
)]

use wasm_bindgen::prelude::*;

mod utils;
mod scene;
mod renderer;


#[wasm_bindgen(start)]
pub fn dummy_main() {
}


#[wasm_bindgen]
pub async fn run() {
    utils::set_panic_hook();
    renderer::main().await;
}



