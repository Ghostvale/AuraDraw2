//! 功能二：游戏号码。v1 支持体彩超级大乐透 · 单注。
//!
//! 一次点击 = 一注 = 一张卡片（前区 5 个 1–35 不重复 + 后区 2 个 1–12 不重复）。
//! 只通过按钮触发生成，不做滚动自动加载。

use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::components::result_card::{CardData, LottoBalls, SignatureItem, result_card};
use crate::util::random_signed::{format_lotto, generate_signed_lotto, lotto_balls};

/// 下拉箭头图标（select 右侧，替代浏览器默认样式）。
fn chevron_icon() -> impl IntoView {
    view! {
        <svg
            class="chevron"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
        >
            <path d="M6 9l6 6 6-6"></path>
        </svg>
    }
}

#[allow(non_snake_case)]
pub fn GamePage() -> impl IntoView {
    let (_game_type, set_game_type) = signal(String::from("体彩 · 超级大乐透"));
    let (_variant, set_variant) = signal(String::from("单注"));
    let (error, set_error) = signal(None::<String>);
    let (loading, set_loading) = signal(false);
    let (cards, set_cards) = signal(Vec::<CardData>::new());

    let start = move || {
        if loading.get() {
            return;
        }
        set_loading.set(true);
        set_error.set(None);

        let (set_loading, set_cards, set_error) = (set_loading, set_cards, set_error);
        spawn_local(async move {
            match generate_signed_lotto().await {
                Ok(resp) => {
                    let (headline, meta) = format_lotto(&resp);
                    let (front, back) = lotto_balls(&resp);
                    set_cards.update(|cs| {
                        cs.push(CardData {
                            index: cs.len() + 1,
                            headline,
                            meta,
                            signatures: vec![SignatureItem {
                                label: None,
                                value: resp.signature,
                            }],
                            balls: Some(LottoBalls { front, back }),
                            expanded: false,
                            copied: false,
                        });
                    });
                }
                Err(e) => set_error.set(Some(e.to_string())),
            }
            set_loading.set(false);
        });
    };

    view! {
        <section class="page-inner">
            <div class="page-head">
                <h1 class="page-title">"游戏号码"</h1>
                <p class="page-sub">"体彩 · 超级大乐透 · 单注 · 签名可验证"</p>
            </div>

            <div class="card form">
                <div class="form-grid">
                    <div class="field">
                        <label>"游戏类型"</label>
                        <div class="select-wrap">
                            <select
                                class="input"
                                on:change=move |ev| set_game_type.set(event_target_value(&ev))
                            >
                                <option value="体彩 · 超级大乐透" selected="true">"体彩 · 超级大乐透"</option>
                            </select>
                            {chevron_icon()}
                        </div>
                    </div>
                    <div class="field">
                        <label>"衍生玩法"</label>
                        <div class="select-wrap">
                            <select
                                class="input"
                                on:change=move |ev| set_variant.set(event_target_value(&ev))
                            >
                                <option value="单注" selected="true">"单注"</option>
                            </select>
                            {chevron_icon()}
                        </div>
                    </div>
                </div>

                <div class="row">
                    <button
                        class="btn btn-primary"
                        disabled=move || loading.get()
                        on:click=move |_| start()
                    >
                        {move || if loading.get() {
                            view! {
                                <span class="spinner"></span>
                                "生成中…"
                            }
                            .into_any()
                        } else {
                            view! { "生成一注" }.into_any()
                        }}
                    </button>
                    {move || if !cards.get().is_empty() {
                        view! {
                            <button
                                class="btn btn-ghost"
                                on:click=move |_| set_cards.set(Vec::new())
                            >
                                "清空结果"
                            </button>
                        }
                        .into_any()
                    } else {
                        view! {}.into_any()
                    }}
                </div>
                <p class="page-sub">"前区 5 个（1–35）不重复 · 后区 2 个（1–12）不重复"</p>
                {move || error.get().map(|e| view! { <p class="form-error">{e}</p> }.into_any())}
            </div>

            {move || if cards.get().is_empty() {
                view! {
                    <div class="card empty-hint">
                        "点击「生成一注」获取一组号码\n点击「加载更多」继续生成"
                    </div>
                }
                .into_any()
            } else {
                view! {
                    <div class="results">
                        <div class="results-head">
                            <p class="results-count">
                                "生成结果 · " <b>{move || cards.get().len().to_string()}</b> " 注"
                            </p>
                        </div>
                        {move || (0..cards.get().len()).map(move |i| result_card(cards, set_cards, i)).collect_view()}
                    </div>
                }
                .into_any()
            }}

            <div class="load-more">
                {move || if !cards.get().is_empty() && !loading.get() {
                    view! {
                        <button class="btn btn-ghost" on:click=move |_| start()>
                            "加载更多"
                        </button>
                    }
                    .into_any()
                } else {
                    view! {}.into_any()
                }}
            </div>
        </section>
    }
}
