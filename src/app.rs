//! 应用外壳：hash 路由、主题状态、导航 + 页面切换。
//!
//! 响应式完全交给 CSS（`styles/main.css` 的媒体查询），
//! Rust 侧只关心路由与主题。

use leptos::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

use crate::components::nav_bar::nav_bar;
use crate::pages::game::GamePage;
use crate::pages::home::HomePage;
use crate::pages::random::RandomPage;
use crate::theme;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Route {
    Home,
    Random,
    Game,
}

fn route_from_hash() -> Route {
    match window().location().hash().unwrap_or_default().as_str() {
        "#/random" => Route::Random,
        "#/game" => Route::Game,
        _ => Route::Home,
    }
}

/// 通过修改 location.hash 导航（同时触发 hashchange）。
pub fn navigate(route: Route) {
    let hash = match route {
        Route::Home => "#/",
        Route::Random => "#/random",
        Route::Game => "#/game",
    };
    let _ = window().location().set_hash(hash);
}

#[allow(non_snake_case)]
pub fn App() -> impl IntoView {
    let mode = RwSignal::new(theme::initial_theme());
    let route = RwSignal::new(route_from_hash());

    // 主题：写回 DOM（data-theme）+ localStorage
    Effect::new(move |_| {
        let m = mode.get();
        theme::apply_theme(m);
    });

    // hash 路由监听（浏览器前进/后退、手动改 hash）
    {
        let route = route;
        let cb = Closure::wrap(Box::new(move || {
            route.set(route_from_hash());
        }) as Box<dyn FnMut()>);
        let _ =
            window().add_event_listener_with_callback("hashchange", cb.as_ref().unchecked_ref());
        cb.forget();
    }

    view! {
        <div class="app">
            {nav_bar(route, mode)}
            <main class="page">
                {move || match route.get() {
                    Route::Home => HomePage().into_any(),
                    Route::Random => RandomPage().into_any(),
                    Route::Game => GamePage().into_any(),
                }}
            </main>
        </div>
    }
}
