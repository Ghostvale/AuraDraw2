//! 导航栏：悬浮玻璃胶囊（PC 顶部 / 移动端底部，位置由 CSS 媒体查询控制），
//! 含品牌 Logo 与主题切换按钮（SVG 太阳/月亮图标）。

use leptos::prelude::*;

use crate::app::{Route, navigate};
use crate::theme::ThemeMode;

/// 太阳图标（点击切换主题的当前状态提示）。
fn sun_icon() -> impl IntoView {
    view! {
        <svg
            class="icon"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
        >
            <circle cx="12" cy="12" r="4"></circle>
            <path d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M6.34 17.66l-1.41 1.41M19.07 4.93l-1.41 1.41"></path>
        </svg>
    }
}

/// 月亮图标。
fn moon_icon() -> impl IntoView {
    view! {
        <svg
            class="icon"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
        >
            <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"></path>
        </svg>
    }
}

pub fn nav_bar(route: RwSignal<Route>, mode: RwSignal<ThemeMode>) -> impl IntoView {
    view! {
        <nav class="nav">
            <div class="nav-inner">
                <button class="nav-brand" on:click=move |_| navigate(Route::Home)>
                    <span class="brand-mark">"A"</span>
                    <span class="brand-name">"AuraDraw"</span>
                </button>

                <div class="nav-links">
                    <button
                        class:active=move || route.get() == Route::Home
                        on:click=move |_| navigate(Route::Home)
                    >
                        "首页"
                    </button>
                    <button
                        class:active=move || route.get() == Route::Random
                        on:click=move |_| navigate(Route::Random)
                    >
                        "随机数"
                    </button>
                    <button
                        class:active=move || route.get() == Route::Game
                        on:click=move |_| navigate(Route::Game)
                    >
                        "游戏"
                    </button>
                </div>

                <button
                    class="nav-theme"
                    title=move || format!("切换到{}", mode.get().label())
                    on:click=move |_| {
                        let next = if mode.get() == ThemeMode::Dark {
                            ThemeMode::Light
                        } else {
                            ThemeMode::Dark
                        };
                        mode.set(next);
                    }
                >
                    {move || if mode.get() == ThemeMode::Dark {
                        sun_icon().into_any()
                    } else {
                        moon_icon().into_any()
                    }}
                </button>
            </div>
        </nav>
    }
}
