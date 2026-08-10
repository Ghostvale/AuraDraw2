//! AuraDraw2 —— Leptos 版本。
//!
//! UI 模块仅在 wasm 目标编译（本应用面向浏览器）；
//! `util`（RANDOM.ORG Signed API 客户端等）在原生目标也编译，
//! 便于 `cargo test` 跑单元测试。

#[cfg(target_arch = "wasm32")]
pub mod app;
#[cfg(target_arch = "wasm32")]
pub mod components;
#[cfg(target_arch = "wasm32")]
pub mod pages;
#[cfg(target_arch = "wasm32")]
pub mod theme;
pub mod util;
