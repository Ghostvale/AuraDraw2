//! 首页：渐变标题 + 大气随机数说明 + 两个功能入口 + 特性条。

use leptos::prelude::*;

/// 骰子图标（随机数入口）。
fn dice_icon() -> impl IntoView {
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
            <rect x="3" y="3" width="18" height="18" rx="4"></rect>
            <circle cx="8.5" cy="8.5" r="1.3" fill="currentColor" stroke="none"></circle>
            <circle cx="15.5" cy="8.5" r="1.3" fill="currentColor" stroke="none"></circle>
            <circle cx="12" cy="12" r="1.3" fill="currentColor" stroke="none"></circle>
            <circle cx="8.5" cy="15.5" r="1.3" fill="currentColor" stroke="none"></circle>
            <circle cx="15.5" cy="15.5" r="1.3" fill="currentColor" stroke="none"></circle>
        </svg>
    }
}

/// 票据图标（游戏号码入口）。
fn ticket_icon() -> impl IntoView {
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
            <path d="M3 9a2 2 0 0 1 0 4v3a1 1 0 0 0 1 1h16a1 1 0 0 0 1-1v-3a2 2 0 0 1 0-4V6a1 1 0 0 0-1-1H4a1 1 0 0 0-1 1Z"></path>
            <path d="M13 5v2M13 17v2M13 11v2"></path>
        </svg>
    }
}

/// 右箭头图标（入口卡片 hover 动效）。
fn arrow_icon() -> impl IntoView {
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
            <path d="M9 18l6-6-6-6"></path>
        </svg>
    }
}

#[allow(non_snake_case)]
pub fn HomePage() -> impl IntoView {
    view! {
        <section class="home">
            <div class="hero">
                <span class="hero-badge">"TRUE RANDOM · 大气噪声熵源"</span>
                <h1 class="title">"AuraDraw"</h1>
                <p class="subtitle">"大气真随机数 · 由 RANDOM.ORG 数字签名保证真实与不可抵赖"</p>
            </div>

            <div class="card intro">
                <h2>"什么是大气随机数？"</h2>
                <p>
                    "本工具通过 RANDOM.ORG 的 Signed API 获取真随机数：\
                     随机性来源于大气噪声（atmospheric noise），是物理世界的熵，\
                     而非计算机的伪随机算法。"
                </p>
                <p>
                    "每次生成都会附带 RANDOM.ORG 的数字签名（见卡片右下角），\
                     任何人都可用官方公钥验证结果确实来自 RANDOM.ORG，\
                     实现真实性与不可抵赖。"
                </p>
            </div>

            <div class="entries">
                <a class="entry" href="#/random">
                    <span class="entry-arrow">{arrow_icon()}</span>
                    <span class="entry-icon">{dice_icon()}</span>
                    <span class="entry-title">"随机数生成"</span>
                    <span class="entry-desc">"指定范围 · 真随机 · 签名可验证"</span>
                </a>
                <a class="entry" href="#/game">
                    <span class="entry-arrow">{arrow_icon()}</span>
                    <span class="entry-icon">{ticket_icon()}</span>
                    <span class="entry-title">"游戏号码"</span>
                    <span class="entry-desc">"体彩 · 超级大乐透 · 单注"</span>
                </a>
            </div>

            <div class="strip">
                <div class="strip-item">
                    <b>"大气噪声"</b>
                    <span>"随机性来自物理世界的熵，而非伪随机算法"</span>
                </div>
                <div class="strip-item">
                    <b>"服务端签名"</b>
                    <span>"每个结果都附带 RANDOM.ORG 数字签名"</span>
                </div>
                <div class="strip-item">
                    <b>"公开可验证"</b>
                    <span>"可用官方公钥验证结果来源，不可抵赖"</span>
                </div>
            </div>

            <footer class="foot">
                "数据来源：RANDOM.ORG Signed API（Release 4）· 真随机性来自大气噪声"
            </footer>
        </section>
    }
}
