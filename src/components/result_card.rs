//! 结果卡片：主内容 + 副信息 + 右下角 signature（截断 / 展开 / 复制）。
//!
//! 随机数页展示大号数字；游戏页展示号码球（前区 / 后区，大乐透 / 七星彩），
//! 卡片头部带游戏标识徽章。
//! 所有信号读取均用 `.get(i)` 安全访问：若视图短暂落后于数据变化，
//! 渲染空内容而不是 panic。

use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::util::clipboard::copy_to_clipboard;

/// 一条签名记录。
///
/// 大乐透若退化为两次 API 调用，一注会携带两条签名，用 `label`
/// （如「前区」「后区」）区分；单签名场景 `label` 为 `None`。
#[derive(Clone)]
pub struct SignatureItem {
    pub label: Option<String>,
    pub value: String,
}

/// 大乐透号码球数据（前区 / 后区）。
#[derive(Clone)]
pub struct LottoBalls {
    pub front: Vec<i64>,
    pub back: Vec<i64>,
}

/// 单张结果卡片的数据（存于页面信号，展开/复制状态也在这里）。
#[derive(Clone)]
pub struct CardData {
    /// 卡片序号（第几张）。
    pub index: usize,
    /// 主内容（大号展示；游戏页为格式化文本兜底，实际渲染号码球）。
    pub headline: String,
    /// 副信息（完成时间 / 序列号）。
    pub meta: String,
    /// 签名列表（右下角展示第一条，展开后展示全部）。
    pub signatures: Vec<SignatureItem>,
    /// 号码球（前区 / 后区）；`None` 时按普通文本渲染 `headline`。
    pub balls: Option<LottoBalls>,
    /// 卡片对应的游戏标识（如「超级大乐透」「七星彩」）；随机数卡片为空字符串。
    pub badge: String,
    /// 是否已展开完整签名。
    pub expanded: bool,
    /// 是否已复制签名。
    pub copied: bool,
}

/// 截断签名：保留头 16 字符 + 尾 6 字符（base64，纯 ASCII，按字节切安全）。
fn truncate_signature(sig: &str) -> String {
    let bytes = sig.as_bytes();
    if bytes.len() <= 30 {
        sig.to_string()
    } else {
        let head = &sig[..16];
        let tail = &sig[bytes.len() - 6..];
        format!("{head}…{tail}")
    }
}

/// 渲染第 `i` 张结果卡片。数据从 `cards` 信号实时读取，
/// 展开/复制状态通过 `set_cards` 写回。
pub fn result_card(
    cards: ReadSignal<Vec<CardData>>,
    set_cards: WriteSignal<Vec<CardData>>,
    i: usize,
) -> impl IntoView {
    let toggle = move |_| {
        set_cards.update(|cs| {
            if let Some(c) = cs.get_mut(i) {
                c.expanded = !c.expanded;
            }
        });
    };
    let copy = move |_| {
        let text = cards
            .get()
            .get(i)
            .and_then(|c| c.signatures.first().map(|s| s.value.clone()))
            .unwrap_or_default();
        spawn_local(async move {
            copy_to_clipboard(&text).await;
            set_cards.update(|cs| {
                if let Some(c) = cs.get_mut(i) {
                    c.copied = true;
                }
            });
        });
    };

    view! {
        <div class="card result">
            <div class="card-head">
                <span class="card-tags">
                    {move || cards.get().get(i).map(|c| {
                        let is_qixing = c.badge == "七星彩";
                        if c.badge.is_empty() {
                            return view! {}.into_any();
                        }
                        view! {
                            <span class="card-tag" class:qixing=is_qixing>
                                {c.badge.clone()}
                            </span>
                        }
                        .into_any()
                    })}
                    <span class="card-index">
                        {move || cards.get().get(i).map(|c| format!("#{}", c.index)).unwrap_or_default()}
                    </span>
                </span>
                <span>
                    {move || cards.get().get(i).map(|c| c.meta.clone()).unwrap_or_default()}
                </span>
            </div>

            // 主内容：号码球 / 普通大号数字
            {move || {
                let Some(card) = cards.get().get(i).cloned() else {
                    return view! {}.into_any();
                };
                let CardData { headline, balls, .. } = card;
                match balls {
                    Some(b) => view! {
                        <div class="balls">
                            <div class="ball-group">
                                <span class="ball-label">"前区"</span>
                                {b.front
                                    .iter()
                                    .map(|n| view! { <span class="ball ball-front">{format!("{n}")}</span> })
                                    .collect_view()}
                            </div>
                            <div class="ball-group">
                                <span class="ball-label">"后区"</span>
                                {b.back
                                    .iter()
                                    .map(|n| view! { <span class="ball ball-back">{format!("{n}")}</span> })
                                    .collect_view()}
                            </div>
                        </div>
                    }
                    .into_any(),
                    None => view! {
                        <div class="card-num">{headline}</div>
                    }
                    .into_any(),
                }
            }}

            {move || {
                let expanded = cards.get().get(i).map(|c| c.expanded).unwrap_or(false);
                if expanded {
                    view! {
                        <div class="sig-full">
                            {move || cards.get().get(i).map(|c| {
                                c.signatures
                                    .iter()
                                    .map(|s| {
                                        let label = s.label.clone().unwrap_or_default();
                                        format!("{label}{}", s.value)
                                    })
                                    .collect::<Vec<_>>()
                                    .join("\n")
                            }).unwrap_or_default()}
                        </div>
                    }
                    .into_any()
                } else {
                    view! {}.into_any()
                }
            }}

            <div class="card-foot">
                <span class="sig">
                    {move || cards.get().get(i).map(|c| {
                        c.signatures
                            .first()
                            .map(|s| format!("签名 {}", truncate_signature(&s.value)))
                            .unwrap_or_default()
                    }).unwrap_or_default()}
                </span>
                <button class="btn btn-ghost btn-sm" on:click=toggle>
                    {move || if cards.get().get(i).map(|c| c.expanded).unwrap_or(false) { "收起" } else { "展开" }}
                </button>
                <button class="btn btn-ghost btn-sm" on:click=copy>
                    {move || if cards.get().get(i).map(|c| c.copied).unwrap_or(false) { "已复制 ✓" } else { "复制" }}
                </button>
            </div>
        </div>
    }
}
