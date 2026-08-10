use leptos::prelude::window;

/// 复制文本到系统剪贴板（wasm 端调用 `navigator.clipboard.writeText`）。
pub async fn copy_to_clipboard(text: &str) {
    let clipboard = window().navigator().clipboard();
    let promise = clipboard.write_text(text);
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}
