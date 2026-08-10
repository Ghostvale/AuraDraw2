#[cfg(target_arch = "wasm32")]
use aura_draw2::app::App;

#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;

#[cfg(target_arch = "wasm32")]
fn main() {
    // 把 Rust panic（含位置与堆栈）打印到浏览器控制台，便于排查。
    console_error_panic_hook::set_once();
    mount_to_body(App);
}

// 原生目标（cargo test）不启动应用。
#[cfg(not(target_arch = "wasm32"))]
fn main() {}
