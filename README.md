# AuraDraw2 · 大气真随机数

基于 **Rust + Leptos 0.8（CSR）+ Trunk** 的网页应用：通过 RANDOM.ORG Signed API
获取**大气噪声真随机数**，每次生成附带数字签名，展示在结果卡片上，
任何人都可用官方公钥验证结果确实来自 RANDOM.ORG（真实性与不可抵赖）。

## 功能

- **首页**：渐变标题 + 大气随机数说明 + 两个功能入口 + 特性说明
- **随机数**：输入最小值/最大值 → 生成一个真随机数（大号渐变数字 + 签名），
  可继续点击「生成」追加，最新结果置顶显示
- **游戏**：体彩 · 超级大乐透 / 七星彩 · 单注
  - 大乐透：前区 5 个 1–35 不重复 + 后区 2 个 1–12 不重复，展示时前后区各升序排序；
  - 七星彩：前区 6 位 0–9（可重复、按位排列）+ 后区 1 个 0–14；
  号码以彩色球展示，卡片头部带游戏标识徽章，一次生成一注
- **主题**：深色 / 浅色一键切换（导航栏图标按钮，localStorage 记忆），
  响应式适配手机 / PC
- **签名卡片**：右下角展示签名（截断显示），可展开查看完整内容、一键复制

## 技术栈

| 组件 | 选择 |
|---|---|
| 前端 | Rust + Leptos 0.8（CSR，wasm ~870KB），原生 CSS |
| 构建 | Trunk（`trunk build --release` → `dist/`） |
| 字体 | Noto Sans SC 经 Google Fonts 按需加载（unicode-range 自动子集化，无需本地字体） |
| 随机数 | RANDOM.ORG Signed API（Release 4，`generateSignedIntegers` / `generateSignedIntegerSequences`） |
| 密钥 | **只存在于服务端**：Cloudflare Pages Function（`functions/api/random.js`）从环境变量注入 |
| 部署 | Cloudflare Pages（静态托管 + Pages Function） |

## 架构

```
浏览器(wasm) --POST /api/random(同源, JSON-RPC, 不含 apiKey)--> Pages Function
Pages Function --注入 RANDOM_ORG_API_KEY 后转发--> api.random.org
Pages Function <--原样透传(含 signature)--------------------- RANDOM.ORG
浏览器 <--原样透传------------------------------------------ Pages Function
```

前端不接触 API Key；本地开发时由 Trunk 把 `/api/*` 转发到本地代理（行为与线上一致）。

## 快速开始（本地开发）

需要 Rust（含 `wasm32-unknown-unknown` 目标）、[Trunk](https://trunkrs.dev/)、
Node ≥ 18 或 Python 3。

```sh
# 1. 起本地代理（持有 API Key，转发给 RANDOM.ORG）
RANDOM_ORG_API_KEY=你的key node dev/proxy.mjs
# 或 python 版：RANDOM_ORG_API_KEY=你的key python3 dev/proxy.py

# 2. 起前端（Trunk 会把 /api/* 转发到上面的代理，见 Trunk.toml）
trunk serve --release
```

打开 http://127.0.0.1:8080 。

> 没有 RANDOM.ORG API Key 时页面功能不可用（代理会返回 500「server not configured」）。
> 免费 Key 可在 https://api.random.org/ 注册申请。

### 联调排查（生成卡住 / 无反应）

生成按钮带 20s 请求超时，代理对上游失败会回 502 错误 JSON，因此**不会无限卡在
「生成中…」**，页面上会显示具体错误。若仍有异常，依次检查：

1. 代理是否在跑：`curl http://127.0.0.1:8787/healthz`
   （应返回 `{"ok":true,"apiKey":"set"}`；`apiKey` 为 `missing` 说明没设环境变量）；
2. 代理终端是否打印 `[proxy] upstream error: ...`（网络到 `api.random.org`
   不通/超时，会以 502 提示到页面）；
3. RANDOM.ORG 免费 Key 的每日比特配额 / 每秒约 1 次的限速；
4. 浏览器硬刷新（Cmd+Shift+R）清掉旧 wasm 缓存；控制台 `panicked at` 报错
   说明 wasm 运行异常（wasm 端不能用 `std::time::SystemTime`，已改用 JS `Date.now()`）。

### 单元测试

```sh
cargo test        # 原生：API 模型/校验/大乐透参数与格式化等纯逻辑测试
```

## 部署（Cloudflare Pages）

### 方式 A：Git 集成（推 GitHub 后自动构建）⭐ 推荐

1. 把仓库推送到 GitHub；
2. Cloudflare Dashboard → **Workers & Pages → Create → Pages → Connect to Git**，
   选中本仓库；
3. 框架预设选 **None**，构建配置：
   - 构建命令：
     ```
     curl https://sh.rustup.rs -sSf | sh -s -- -y --profile minimal --default-toolchain stable --target wasm32-unknown-unknown && source $HOME/.cargo/env && cargo install trunk --locked && trunk build --release
     ```
   - 输出目录：`dist`
4. 项目 **Settings → Environment variables**：添加 `RANDOM_ORG_API_KEY`（生产 Key）；
5. 首次构建较慢（需编译 trunk 工具链 + 项目），之后增量构建；
6. 部署完成后，可以给你的 Pages 项目绑定自定义域名
   （**Workers & Pages → 你的项目 → Custom domains**）。

> `functions/api/random.js` 会被 Pages Functions 自动识别为 `/api/random`，
> `dist/_headers` 由 trunk 自动生成（wasm MIME 类型保险），无需额外配置。

### 方式 B：wrangler 直接上传（最快，无需在 CF 上编译 Rust）

```sh
npm i -g wrangler      # 或 npx wrangler
wrangler login

trunk build --release
wrangler pages deploy dist --project-name auradraw
```

上传后到 Dashboard 给 `auradraw` 项目配置环境变量 `RANDOM_ORG_API_KEY`。

> `wrangler.toml` 已写好（`pages_build_output_dir = "dist"`），
> 之后直接 `wrangler pages deploy` 即可。

### 发布前检查清单

- [ ] `RANDOM_ORG_API_KEY` 已配置在 Cloudflare 项目环境变量（**不要**写进代码或前端）；
- [ ] 部署后验证：`curl -X POST https://你的域名/api/random -H 'content-type: application/json' -d '{"jsonrpc":"2.0","method":"generateSignedIntegers","params":{"n":1,"min":1,"max":100,"replacement":true,"base":10},"id":1}'`
      应返回带 `signature` 的 JSON-RPC 响应；
- [ ] 建议配置 **Rate Limiting**，防公网 `/api/random` 被刷。

### ⚠️ License 合规

RANDOM.ORG 免费 Key 是 **developer license**（"licensed strictly for development
and testing only"），**仅限开发/测试**。正式公开上线请购买付费套餐
（https://api.random.org/pricing ），并把付费 Key 配置到 CF 环境变量。

### 安全提示

- API Key 只存在于 CF 环境变量，前端不接触；
- 建议平台侧限流 + 只允许本站域名的引用；
- 代理函数不打印 Key，日志请保持脱敏。

## 目录结构

```
src/
  main.rs            入口（wasm 挂载，含 panic hook 便于排查）
  app.rs             hash 路由 + 主题状态 + 导航布局
  pages/             首页 / 随机数 / 游戏
  components/        导航栏 / 结果卡片（号码球 + 游戏徽章 + 签名截断·展开·复制）
  util/
    random_signed.rs RANDOM.ORG Signed API 客户端（serde 模型 + 异步，可原生单测）
    clipboard.rs     wasm 剪贴板封装
styles/main.css     全部样式（CSS 变量主题 + 响应式媒体查询）
functions/api/random.js  Cloudflare Pages Function（/api/random）
dev/proxy.mjs       本地开发代理（node）
dev/proxy.py        本地开发代理（python）
wrangler.toml       Cloudflare Pages 部署配置
```
