//! 功能一：随机数。一次点击 = 一个数字 = 一张卡片。
//!
//! v1 隐藏「生成数量」与「是否允许重复」（方案 Y，见 docs/DESIGN.md §0），
//! 批量生成留待后续版本。只通过按钮触发生成，不做滚动自动加载。

use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::components::result_card::{CardData, SignatureItem, result_card};
use crate::util::random_signed::generate_signed_integer;

#[allow(non_snake_case)]
pub fn RandomPage() -> impl IntoView {
    let (min, set_min) = signal(String::from("1"));
    let (max, set_max) = signal(String::from("100"));
    let (form_error, set_form_error) = signal(None::<String>);
    let (loading, set_loading) = signal(false);
    let (cards, set_cards) = signal(Vec::<CardData>::new());

    // 触发生成（「生成」按钮 / 「加载更多」按钮共用）
    let start = move || {
        if loading.get() {
            return;
        }
        let min_v: i64 = match min.get().trim().parse() {
            Ok(v) => v,
            Err(_) => {
                set_form_error.set(Some("最小值必须是整数".into()));
                return;
            }
        };
        let max_v: i64 = match max.get().trim().parse() {
            Ok(v) => v,
            Err(_) => {
                set_form_error.set(Some("最大值必须是整数".into()));
                return;
            }
        };
        if !(-1_000_000_000..=1_000_000_000).contains(&min_v) {
            set_form_error.set(Some(
                "最小值超出范围 [-1,000,000,000, 1,000,000,000]".into(),
            ));
            return;
        }
        if !(-1_000_000_000..=1_000_000_000).contains(&max_v) {
            set_form_error.set(Some(
                "最大值超出范围 [-1,000,000,000, 1,000,000,000]".into(),
            ));
            return;
        }
        if min_v > max_v {
            set_form_error.set(Some("最小值不能大于最大值".into()));
            return;
        }

        set_loading.set(true);
        set_form_error.set(None);

        let (set_loading, set_cards, set_form_error) = (set_loading, set_cards, set_form_error);
        spawn_local(async move {
            match generate_signed_integer(min_v, max_v).await {
                Ok(resp) => {
                    let value = resp
                        .data
                        .first()
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "?".into());
                    set_cards.update(|cs| {
                        cs.push(CardData {
                            index: cs.len() + 1,
                            headline: value,
                            meta: format!(
                                "完成 {} · 序列号 {}",
                                resp.completion_time, resp.serial_number
                            ),
                            signatures: vec![SignatureItem {
                                label: None,
                                value: resp.signature,
                            }],
                            balls: None,
                            badge: String::new(),
                            expanded: false,
                            copied: false,
                        });
                    });
                }
                Err(e) => set_form_error.set(Some(e.to_string())),
            }
            set_loading.set(false);
        });
    };

    view! {
        <section class="page-inner">
            <div class="page-head">
                <h1 class="page-title">"随机数"</h1>
                <p class="page-sub">"由大气噪声真随机生成 · 每次生成带 RANDOM.ORG 数字签名"</p>
            </div>

            <div class="card form">
                <div class="form-grid">
                    <div class="field">
                        <label>"最小值"</label>
                        <input
                            class="input"
                            type="text"
                            placeholder="如 -1000"
                            value="1"
                            on:input=move |ev| set_min.set(event_target_value(&ev))
                        />
                    </div>
                    <div class="field">
                        <label>"最大值"</label>
                        <input
                            class="input"
                            type="text"
                            placeholder="如 1000"
                            value="100"
                            on:input=move |ev| set_max.set(event_target_value(&ev))
                        />
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
                            view! { "生成" }.into_any()
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
                {move || form_error.get().map(|e| view! { <p class="form-error">{e}</p> }.into_any())}
            </div>

            {move || if cards.get().is_empty() {
                view! {
                    <div class="card empty-hint">
                        "点击「生成」获取一个真随机数\n点击「加载更多」继续生成"
                    </div>
                }
                .into_any()
            } else {
                view! {
                    <div class="results">
                        <div class="results-head">
                            <p class="results-count">
                                "生成结果 · " <b>{move || cards.get().len().to_string()}</b> " 张"
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
