//! 主题：深色/浅色模式。样式本身在 `styles/main.css`（CSS 变量），
//! 这里只负责状态切换、写入 DOM 与 localStorage 持久化。

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    Dark,
    Light,
}

impl ThemeMode {
    /// 切换按钮文案：显示点击后要切换到的模式。
    pub fn label(self) -> &'static str {
        match self {
            ThemeMode::Dark => "浅色",
            ThemeMode::Light => "深色",
        }
    }

    /// 写入 `<html data-theme="...">` 的属性值。
    pub fn data_theme(self) -> &'static str {
        match self {
            ThemeMode::Dark => "dark",
            ThemeMode::Light => "light",
        }
    }
}

use leptos::prelude::*;

/// 初始主题：localStorage 优先，默认深色。
pub fn initial_theme() -> ThemeMode {
    let stored = window()
        .local_storage()
        .ok()
        .flatten()
        .and_then(|ls| ls.get_item("auradraw-theme").ok().flatten());
    match stored.as_deref() {
        Some("light") => ThemeMode::Light,
        Some("dark") => ThemeMode::Dark,
        _ => ThemeMode::Dark,
    }
}

/// 应用主题：更新 `<html data-theme>` 并持久化。
pub fn apply_theme(mode: ThemeMode) {
    if let Some(el) = document().document_element() {
        let _ = el.set_attribute("data-theme", mode.data_theme());
    }
    if let Some(ls) = window().local_storage().ok().flatten() {
        let _ = ls.set_item("auradraw-theme", mode.data_theme());
    }
}
