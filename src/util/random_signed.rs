use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::time::Duration;
#[cfg(not(target_arch = "wasm32"))]
use std::time::{SystemTime, UNIX_EPOCH};

/// 同源代理端点。
///
/// 前端不接触 API Key：请求发往本路径，由 Cloudflare Pages Function
/// （见 `functions/api/random.js`）注入 `apiKey` 后转发到
/// `https://api.random.org/json-rpc/4/invoke`，响应（含 `signature`）原样透传。
///
/// 本地开发时由 Trunk 把 `/api/*` 代理到本地后端（见 `Trunk.toml` 与 `dev/proxy.mjs`）。
const API_ENDPOINT: &str = "/api/random";

/// 请求地址。wasm 端 reqwest 的 `Url::parse` 不接受相对路径（会报 builder error），
/// 因此从 `location.origin` 拼出绝对 URL；原生端（仅单测）用相对路径即可。
fn api_endpoint() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        let origin = web_sys::window()
            .and_then(|w| w.location().origin().ok())
            .unwrap_or_default();
        format!("{origin}{API_ENDPOINT}")
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        API_ENDPOINT.to_string()
    }
}

/// 单次请求超时：网络卡住时让前端显示明确错误，而不是无限停留在「生成中…」。
/// wasm 端由 reqwest 内部的 AbortController 实现超时（RequestBuilder::timeout）。
const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);

/// 生成随机的 JSON-RPC 请求 ID。
///
/// ⚠️ 不能直接用 `std::time::SystemTime::now()`：wasm32-unknown-unknown 上它会
/// panic（`time not implemented on this platform`），wasm 端改用 JS `Date.now()`。
fn gen_request_id() -> u64 {
    gen_millis() % 1000000
}

#[cfg(target_arch = "wasm32")]
fn gen_millis() -> u64 {
    js_sys::Date::now() as u64
}

#[cfg(not(target_arch = "wasm32"))]
fn gen_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ==========================================
// JSON-RPC 通用协议结构体定义
// ==========================================

#[derive(Debug, Serialize)]
struct JsonRpcRequest<P> {
    jsonrpc: &'static str,
    method: &'static str,
    params: P,
    id: u64,
}

#[derive(Debug, Deserialize)]
struct JsonRpcResponse<R> {
    result: Option<R>,
    error: Option<JsonRpcError>,
    #[serde(rename = "id")]
    _id: u64,
}

#[derive(Debug, Deserialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(default)]
    #[serde(rename = "data")]
    _data: Option<serde_json::Value>,
}

/// Signed API 的 `result` 对象。
///
/// 与 Basic API 不同，Signed API 的 `result` 额外包含 `signature`——
/// 对 `random` 对象使用 RANDOM.ORG 私钥签名的 base64 数字签名，
/// 可用于验证响应确实来源于 RANDOM.ORG（不可抵赖性）。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SignedRandomResult<D> {
    random: SignedRandomData<D>,
    signature: String,
    cost: f64,
    bits_used: u64,
    bits_left: u64,
    requests_left: u64,
    advisory_delay: u64,
}

/// Signed API 的 `random` 对象（整体被签名，客户端应保存它及其 `signature`）。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SignedRandomData<D> {
    /// 产生该随机值的 API 方法名，如 `generateSignedIntegers`。
    #[allow(dead_code)]
    method: String,
    /// API Key 的 base64 编码 SHA-512 哈希，可公开关联响应而不泄露 API Key。
    hashed_api_key: String,
    data: D,
    /// 服务端完成该请求的 UTC 时间戳 (ISO 8601 格式)。
    completion_time: String,
    /// 该响应关联的序列号，同一 `apiKey` 内唯一。
    serial_number: u64,
}

/// 从响应文本中提取代理层错误（形如 `{"error": "..."}`）。
/// RANDOM.ORG 的 JSON-RPC 错误响应虽然也用 `error` 字段，但它是对象，
/// 不会匹配到这里。
fn extract_proxy_error(text: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(text).ok()?;
    value.get("error")?.as_str().map(|s| s.to_string())
}

/// 把 reqwest 网络错误转成可读的中文提示（区分超时 / 连接失败 / 其他）。
/// wasm 端部分 `is_*` 方法不可用，统一用错误文本兜底匹配。
fn describe_network_error(e: &reqwest::Error) -> anyhow::Error {
    let msg = e.to_string();
    if e.is_timeout()
        || msg.contains("timeout")
        || msg.contains("timed out")
        || msg.contains("aborted")
    {
        anyhow!("网络请求超时（20 秒未响应）。请检查网络连接后重试。")
    } else if msg.contains("Failed to fetch")
        || msg.contains("Connection refused")
        || msg.contains("error trying to connect")
    {
        anyhow!("网络请求失败：无法连接随机数服务。请检查网络连接后重试。")
    } else {
        anyhow!("网络请求失败：{msg}。请检查网络连接后重试。")
    }
}

/// 截取响应体用于错误提示：压缩换行、只留前 80 字符，
/// 避免超长 / 多行原文撑爆提示框（如网关返回整页 HTML）。
fn preview_text(text: &str) -> String {
    let flat: String = text
        .chars()
        .map(|c| if c == '\r' || c == '\n' { ' ' } else { c })
        .collect();
    let flat = flat.trim();
    let count = flat.chars().count();
    if count == 0 {
        return "响应体为空。".to_string();
    }
    let mut preview: String = flat.chars().take(80).collect();
    if count > 80 {
        preview.push('…');
    }
    format!("原始响应：{preview}")
}

/// 向代理端点发送 JSON-RPC 请求并解析 Signed 响应。
///
/// 带 20s 超时（reqwest `RequestBuilder::timeout`，wasm 端内部用 AbortController
/// 取消 fetch），超时返回明确错误，避免网络卡住时按钮无限停留在「生成中…」。
async fn post_invoke<D>(
    method: &'static str,
    params: impl Serialize,
) -> Result<SignedRandomResult<D>>
where
    D: serde::de::DeserializeOwned,
{
    let request = JsonRpcRequest {
        jsonrpc: "2.0",
        method,
        params,
        id: gen_request_id(),
    };

    let client = reqwest::Client::new();
    let res = client
        .post(api_endpoint())
        .json(&request)
        .timeout(REQUEST_TIMEOUT)
        .send()
        .await
        .map_err(|e| describe_network_error(&e))?;
    let status = res.status();
    let text = res
        .text()
        .await
        .map_err(|e| anyhow!("读取随机数服务响应失败：{e}。请稍后重试。"))?;

    // 代理层错误（形如 {"error": "..."}，如未配置 API Key、上游超时等）
    if let Some(msg) = extract_proxy_error(&text) {
        return Err(anyhow!("随机数服务暂不可用：{}。请稍后重试。", msg));
    }

    // 非 2xx：多为部署 / 网关层问题，响应体一般不是可解析的 JSON-RPC
    if !status.is_success() {
        return Err(anyhow!(
            "随机数服务返回异常状态 HTTP {}。{} 请稍后重试。",
            status,
            preview_text(&text)
        ));
    }

    let rpc_res: JsonRpcResponse<SignedRandomResult<D>> =
        serde_json::from_str(&text).map_err(|_| {
            anyhow!(
                "随机数服务响应无法解析（不是预期的 JSON 格式，可能是服务故障或网络代理拦截）。\
                 {} 请稍后重试。",
                preview_text(&text)
            )
        })?;

    if let Some(error) = rpc_res.error {
        return Err(anyhow!(
            "RANDOM.ORG API 错误 [{}]：{}",
            error.code,
            error.message
        ));
    }

    rpc_res
        .result
        .ok_or_else(|| anyhow!("接口响应异常（HTTP {}），未包含结果或错误信息。", status))
}

// ==========================================
// 1. generateSignedIntegers 接口
// ==========================================

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SignedIntegersParams {
    n: usize,
    min: i64,
    max: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    replacement: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    base: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pregenerated_randomization: Option<serde_json::Value>,
}

/// Signed API 随机整数接口的返回数据结构。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedIntegersResponse {
    /// 生成的随机整数数组。10 进制下为整数；其他进制为带前导零的字符串。
    pub data: Vec<serde_json::Value>,
    /// 服务端完成该请求的 UTC 时间戳 (ISO 8601 格式)。
    pub completion_time: String,
    /// 响应序列号，同一 `apiKey` 内唯一。
    pub serial_number: u64,
    /// API Key 的 base64 编码 SHA-512 哈希。
    pub hashed_api_key: String,
    /// 对 `random` 对象的 base64 数字签名。
    pub signature: String,
    /// 该请求的费用（USD）。
    pub cost: f64,
    /// 满足此请求所消耗的真随机比特数。
    pub bits_used: u64,
    /// 客户端剩余的估算可用随机比特数。
    pub bits_left: u64,
    /// 客户端剩余的估算可用 API 请求次数。
    pub requests_left: u64,
    /// 建议客户端在发送下一个请求之前延迟的毫秒数。
    pub advisory_delay: u64,
}

/// 校验 `generateSignedIntegers` 的参数。
fn validate_integers(n: usize, min: i64, max: i64) -> Result<()> {
    if n < 1 || n > 10000 {
        return Err(anyhow!(
            "参数 `n` 越界，必须在 [1, 10000] 之间。当前值: {}",
            n
        ));
    }
    if min < -1_000_000_000 || min > 1_000_000_000 {
        return Err(anyhow!(
            "参数 `min` 越界，必须在 [-1,000,000,000, 1,000,000,000] 之间。当前值: {}",
            min
        ));
    }
    if max < -1_000_000_000 || max > 1_000_000_000 {
        return Err(anyhow!(
            "参数 `max` 越界，必须在 [-1,000,000,000, 1,000,000,000] 之间。当前值: {}",
            max
        ));
    }
    if min > max {
        return Err(anyhow!(
            "参数下界 `min` ({}) 不能大于参数上界 `max` ({})。",
            min,
            max
        ));
    }
    Ok(())
}

/// 生成指定范围内的随机整数。
///
/// * `n` - 数量，`[1, 10000]`。
/// * `min` / `max` - 闭区间上下界，`[-1e9, 1e9]`。
/// * `replacement` - `true` 放回抽样（允许重复）；`false` 不重复。
pub async fn generate_signed_integers(
    n: usize,
    min: i64,
    max: i64,
    replacement: bool,
) -> Result<SignedIntegersResponse> {
    validate_integers(n, min, max)?;

    let params = SignedIntegersParams {
        n,
        min,
        max,
        replacement: Some(replacement),
        base: Some(10),
        pregenerated_randomization: None,
    };

    let result = post_invoke("generateSignedIntegers", params).await?;
    Ok(SignedIntegersResponse {
        data: result.random.data,
        completion_time: result.random.completion_time,
        serial_number: result.random.serial_number,
        hashed_api_key: result.random.hashed_api_key,
        signature: result.signature,
        cost: result.cost,
        bits_used: result.bits_used,
        bits_left: result.bits_left,
        requests_left: result.requests_left,
        advisory_delay: result.advisory_delay,
    })
}

/// v1 便捷入口：一次调用生成一个随机整数（一张卡片）。
pub async fn generate_signed_integer(min: i64, max: i64) -> Result<SignedIntegersResponse> {
    generate_signed_integers(1, min, max, true).await
}

// ==========================================
// 2. generateSignedIntegerSequences 接口
// ==========================================

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SignedIntegerSequencesParams {
    n: usize,
    length: serde_json::Value,
    min: serde_json::Value,
    max: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    replacement: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    base: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pregenerated_randomization: Option<serde_json::Value>,
}

/// Signed API 随机整数序列接口的返回数据结构。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedIntegerSequencesResponse {
    /// 生成的随机序列二维数组。
    pub data: Vec<Vec<serde_json::Value>>,
    /// 服务端完成该请求的 UTC 时间戳 (ISO 8601 格式)。
    pub completion_time: String,
    /// 响应序列号，同一 `apiKey` 内唯一。
    pub serial_number: u64,
    /// API Key 的 base64 编码 SHA-512 哈希。
    pub hashed_api_key: String,
    /// 对 `random` 对象的 base64 数字签名。
    pub signature: String,
    /// 该请求的费用（USD）。
    pub cost: f64,
    /// 满足此请求所消耗的真随机比特数。
    pub bits_used: u64,
    /// 客户端剩余的估算可用随机比特数。
    pub bits_left: u64,
    /// 客户端剩余的估算可用 API 请求次数。
    pub requests_left: u64,
    /// 建议客户端在发送下一个请求之前延迟的毫秒数。
    pub advisory_delay: u64,
}

/// 校验 `generateSignedIntegerSequences` 的参数。
fn validate_sequences(n: usize) -> Result<()> {
    if n < 1 || n > 1000 {
        return Err(anyhow!(
            "参数 `n` 越界，序列数量必须在 [1, 1000] 之间。当前值: {}",
            n
        ));
    }
    Ok(())
}

/// 生成随机整数序列。
///
/// `length` / `min` / `max` / `replacement` 支持单个标量（均匀序列）
/// 或与 `n` 等长的数组（多变序列，逐序列生效）。
pub async fn generate_signed_integer_sequences(
    n: usize,
    length: serde_json::Value,
    min: serde_json::Value,
    max: serde_json::Value,
    replacement: Option<serde_json::Value>,
) -> Result<SignedIntegerSequencesResponse> {
    validate_sequences(n)?;

    let params = SignedIntegerSequencesParams {
        n,
        length,
        min,
        max,
        replacement,
        base: None,
        pregenerated_randomization: None,
    };

    let result = post_invoke("generateSignedIntegerSequences", params).await?;
    Ok(SignedIntegerSequencesResponse {
        data: result.random.data,
        completion_time: result.random.completion_time,
        serial_number: result.random.serial_number,
        hashed_api_key: result.random.hashed_api_key,
        signature: result.signature,
        cost: result.cost,
        bits_used: result.bits_used,
        bits_left: result.bits_left,
        requests_left: result.requests_left,
        advisory_delay: result.advisory_delay,
    })
}

/// 体彩超级大乐透 · 单注。
///
/// 一次调用生成一注：`n=2` 两条序列——`length=[5,2]` 分别对应前区 5 个
/// （1–35，不重复）与后区 2 个（1–12，不重复），一张卡片对应一个 `signature`。
///
/// ⚠️ `length`/`min`/`max`/`replacement` 数组长度必须等于 `n`（每条序列一组参数）：
/// 实测 `n=1` 时传 `[5,2]` 会被拒绝（API Error 203: length 数组过长）。
pub async fn generate_signed_lotto() -> Result<SignedIntegerSequencesResponse> {
    generate_signed_integer_sequences(
        2,
        serde_json::json!([5, 2]),
        serde_json::json!([1, 1]),
        serde_json::json!([35, 12]),
        Some(serde_json::json!([false, false])),
    )
    .await
}

/// 体彩七星彩 · 单注。
///
/// 规则：前区 6 位（每位 0–9，可重复、按位排列，位置敏感）+ 后区 1 个（0–14）。
/// 与 `generate_signed_lotto` 同构：`n=2`、`length=[6,1]`；前区允许重复故 `replacement=true`。
pub async fn generate_signed_qixing() -> Result<SignedIntegerSequencesResponse> {
    generate_signed_integer_sequences(
        2,
        serde_json::json!([6, 1]),
        serde_json::json!([0, 0]),
        serde_json::json!([9, 14]),
        Some(serde_json::json!([true, true])),
    )
    .await
}

/// 把「序列」接口响应拆分为 (前区, 后区) 号码数组。
///
/// `n=2` 的响应 `data` 为两条序列：`data[0]` = 前区，`data[1]` = 后区。
/// `sort` 为 `true` 时（大乐透）前后区各升序排序；七星彩按位排列、顺序敏感，传 `false`。
fn seq_balls(resp: &SignedIntegerSequencesResponse, sort: bool) -> (Vec<i64>, Vec<i64>) {
    let to_i64 = |nums: &[serde_json::Value]| {
        let mut v: Vec<i64> = nums.iter().map(|v| v.as_i64().unwrap_or(0)).collect();
        if sort {
            v.sort_unstable();
        }
        v
    };
    let front = resp.data.first().cloned().unwrap_or_default();
    let back = resp.data.get(1).cloned().unwrap_or_default();
    (to_i64(&front), to_i64(&back))
}

/// 把大乐透响应拆分为 (前区, 后区) 号码数组，供卡片号码球渲染；前后区各升序排序。
pub fn lotto_balls(resp: &SignedIntegerSequencesResponse) -> (Vec<i64>, Vec<i64>) {
    seq_balls(resp, true)
}

/// 把七星彩响应拆分为 (前区, 后区) 号码数组；按位排列，顺序敏感，不排序。
pub fn qixing_balls(resp: &SignedIntegerSequencesResponse) -> (Vec<i64>, Vec<i64>) {
    seq_balls(resp, false)
}

/// 把前后区号码格式化为卡片内容：`前区 xx xx …   后区 xx xx …`（两位补零）。
/// 返回 (主内容, 副信息)。
fn format_balls(
    front: &[i64],
    back: &[i64],
    resp: &SignedIntegerSequencesResponse,
) -> (String, String) {
    let fmt = |nums: &[i64]| {
        nums.iter()
            .map(|n| format!("{n:02}"))
            .collect::<Vec<_>>()
            .join(" ")
    };

    let headline = format!("前区 {}   后区 {}", fmt(front), fmt(back));
    let meta = format!(
        "完成 {} · 序列号 {}",
        resp.completion_time, resp.serial_number
    );
    (headline, meta)
}

/// 把大乐透响应格式化为卡片内容（前后区已排序）。返回 (主内容, 副信息)。
pub fn format_lotto(resp: &SignedIntegerSequencesResponse) -> (String, String) {
    let (front, back) = lotto_balls(resp);
    format_balls(&front, &back, resp)
}

/// 把七星彩响应格式化为卡片内容（按位排列，不排序）。返回 (主内容, 副信息)。
pub fn format_qixing(resp: &SignedIntegerSequencesResponse) -> (String, String) {
    let (front, back) = qixing_balls(resp);
    format_balls(&front, &back, resp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_validate_integers() {
        // 数量越界
        assert!(validate_integers(0, 1, 10).is_err());
        assert!(validate_integers(10001, 1, 10).is_err());
        // min/max 越界
        assert!(validate_integers(1, -1_000_000_001, 10).is_err());
        assert!(validate_integers(1, 1, 1_000_000_001).is_err());
        // min > max
        assert!(validate_integers(1, 10, 1).is_err());
        // 合法
        assert!(validate_integers(1, -1_000_000_000, 1_000_000_000).is_ok());
        assert!(validate_integers(10000, 0, 0).is_ok());
    }

    #[test]
    fn test_validate_sequences() {
        assert!(validate_sequences(0).is_err());
        assert!(validate_sequences(1001).is_err());
        assert!(validate_sequences(1).is_ok());
        assert!(validate_sequences(1000).is_ok());
    }

    /// 使用官方文档示例的响应格式验证 Signed API 响应反序列化是否正确，
    /// 重点覆盖 `signature`、`cost`、`serialNumber`、`hashedApiKey` 字段。
    #[test]
    fn test_signed_response_deserialization() {
        let sample = r#"{
            "jsonrpc": "2.0",
            "result": {
                "random": {
                    "method": "generateSignedIntegers",
                    "hashedApiKey": "ncGk4bCmDT7GSc64MzGzNvRUoDT++pTPjntmtuu075JFqKbz/G4nKerq0JQoldvtQxYOCePxMN5gcYZSOC2DTg==",
                    "n": 3,
                    "min": 1,
                    "max": 6,
                    "replacement": true,
                    "base": 10,
                    "pregeneratedRandomization": null,
                    "data": [1, 3, 1],
                    "license": {
                        "type": "developer",
                        "text": "Random values licensed strictly for development and testing only",
                        "infoUrl": null
                    },
                    "licenseData": null,
                    "userData": null,
                    "ticketData": null,
                    "completionTime": "2021-03-15 13:51:32Z",
                    "serialNumber": 6116
                },
                "signature": "hprai35Zc95uAM47oVpqUTEiVla/GvF+u/8GjZCvcGKRG86fQrnVvuzn1HN5VrJoU13SDE96DmggtTYECzkk9bzfVnhHg47/Zn+7w27GedseB2F4QxNtf7aycvcdBHnSg08IaVo+ohPiqlZcxpx5TVUfmLb6LfYRPirQUHMv5vpT7ba/hDSb7bQ6wGpiV1By48nDC5p/ncZEvfAHQcrNxtrtCbwQoI9BMBxRXqV5DaG6YYPxTpQeg9dWJMhZJuBNWIf4hsCKoOGkyBI/uHPaGgTy5jmSk4cFutK3jQP+9vWkDwYQ9sgok0U9Dgp5jG2zC6JOwaEgosagY7B29r1s6aXxcZCXFtX9yBdAh6Of7Z1PeLeva14lQWdZmqYSYvD56HlYWQfeb0lY2Lgf7Yvr9W/lxUxSg9OUvXi+urR0sprXpGwOcml5dSVRXyG6oyDphwXsvJ8h9ofiCP5rkyxHNphR6s1LF5NQ91OCBDllXiwXAKvJBcBxftFVAJRqpRALuLQB2xTXlrld/XBEBc93Pve3e+B0DancFa1XHgBFLlRSmF+MpSY+8qIT2U4hHSGO38ISSX2RdHYR+talXoQ8Vj6fiibzZCUNMbXp4HcYRjmWUVCii0otGYC/fSg25ZmnpG/SMJXfDbVpzx8sC49qYpaN9GRG5QC5pHfA69nJVqo=",
                "cost": 0.0,
                "bitsUsed": 8,
                "bitsLeft": 249992,
                "requestsLeft": 999,
                "advisoryDelay": 2310
            },
            "id": 6995
        }"#;

        let rpc: JsonRpcResponse<SignedRandomResult<Vec<serde_json::Value>>> =
            serde_json::from_str(sample).unwrap();

        assert!(rpc.error.is_none());

        let result = rpc
            .result
            .expect("successful response should contain result");
        assert_eq!(result.random.method, "generateSignedIntegers");
        assert_eq!(
            result.random.hashed_api_key,
            "ncGk4bCmDT7GSc64MzGzNvRUoDT++pTPjntmtuu075JFqKbz/G4nKerq0JQoldvtQxYOCePxMN5gcYZSOC2DTg=="
        );
        assert_eq!(
            result.random.data,
            vec![
                serde_json::json!(1),
                serde_json::json!(3),
                serde_json::json!(1)
            ]
        );
        assert_eq!(result.random.completion_time, "2021-03-15 13:51:32Z");
        assert_eq!(result.random.serial_number, 6116);
        assert!(result.signature.starts_with("hprai35Zc95uAM47oVpq"));
        assert_eq!(result.cost, 0.0);
        assert_eq!(result.bits_used, 8);
        assert_eq!(result.bits_left, 249992);
        assert_eq!(result.requests_left, 999);
        assert_eq!(result.advisory_delay, 2310);
    }

    /// 验证 Signed API 的错误响应（`error` 分支）也能被正确反序列化。
    #[test]
    fn test_signed_error_response_deserialization() {
        let sample = r#"{
            "jsonrpc": "2.0",
            "error": {
                "code": 422,
                "message": "The ticket you specified has already been used",
                "data": null
            },
            "id": 6995
        }"#;

        let rpc: JsonRpcResponse<SignedRandomResult<Vec<serde_json::Value>>> =
            serde_json::from_str(sample).unwrap();

        assert!(rpc.result.is_none());
        let error = rpc.error.expect("error response should contain error");
        assert_eq!(error.code, 422);
        assert_eq!(
            error.message,
            "The ticket you specified has already been used"
        );
    }

    /// 代理层错误（如未配置 API Key）应被识别并给出可读提示。
    #[test]
    fn test_extract_proxy_error() {
        assert_eq!(
            extract_proxy_error(r#"{"error": "server not configured"}"#),
            Some("server not configured".into())
        );
        // JSON-RPC 错误响应的 error 是对象，不应误判为代理错误
        assert_eq!(
            extract_proxy_error(r#"{"jsonrpc":"2.0","error":{"code":422,"message":"x"},"id":1}"#),
            None
        );
        assert_eq!(extract_proxy_error("not json"), None);
    }

    /// 验证大乐透固定参数拼装正确（序列数、长度、范围、去重属性）。
    #[test]
    fn test_lotto_params() {
        let params = SignedIntegerSequencesParams {
            n: 2,
            length: serde_json::json!([5, 2]),
            min: serde_json::json!([1, 1]),
            max: serde_json::json!([35, 12]),
            replacement: Some(serde_json::json!([false, false])),
            base: None,
            pregenerated_randomization: None,
        };
        let body = serde_json::to_value(&params).unwrap();
        // length/min/max/replacement 数组长度必须等于 n（每条序列一组参数）
        assert_eq!(body["n"], 2);
        assert_eq!(body["length"], serde_json::json!([5, 2]));
        assert_eq!(body["min"], serde_json::json!([1, 1]));
        assert_eq!(body["max"], serde_json::json!([35, 12]));
        assert_eq!(body["replacement"], serde_json::json!([false, false]));
        assert_eq!(body["length"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_format_lotto() {
        // n=2 的响应：data 为两条序列（前区 5 个 + 后区 2 个），
        // 输入乱序，验证拆分后前后区各自升序排序。
        let resp = SignedIntegerSequencesResponse {
            data: vec![
                vec![json!(33), json!(5), json!(28), json!(12), json!(21)],
                vec![json!(11), json!(3)],
            ],
            completion_time: "2026-08-09 12:00:00Z".into(),
            serial_number: 42,
            hashed_api_key: "hash".into(),
            signature: "sig".into(),
            cost: 0.0,
            bits_used: 0,
            bits_left: 0,
            requests_left: 0,
            advisory_delay: 0,
        };
        let (front, back) = lotto_balls(&resp);
        assert_eq!(front, vec![5, 12, 21, 28, 33]);
        assert_eq!(back, vec![3, 11]);
        let (headline, meta) = format_lotto(&resp);
        assert_eq!(headline, "前区 05 12 21 28 33   后区 03 11");
        assert!(meta.contains("序列号 42"));
    }

    #[test]
    fn test_format_lotto_edge() {
        // 空数据兜底：不应 panic
        let resp = SignedIntegerSequencesResponse {
            data: vec![],
            completion_time: "".into(),
            serial_number: 0,
            hashed_api_key: "".into(),
            signature: "".into(),
            cost: 0.0,
            bits_used: 0,
            bits_left: 0,
            requests_left: 0,
            advisory_delay: 0,
        };
        let (headline, _) = format_lotto(&resp);
        assert_eq!(headline, "前区    后区 ");
    }

    /// 验证七星彩固定参数拼装正确（前区 6 位 0–9 可重复 + 后区 1 个 0–14）。
    #[test]
    fn test_qixing_params() {
        let params = SignedIntegerSequencesParams {
            n: 2,
            length: serde_json::json!([6, 1]),
            min: serde_json::json!([0, 0]),
            max: serde_json::json!([9, 14]),
            replacement: Some(serde_json::json!([true, true])),
            base: None,
            pregenerated_randomization: None,
        };
        let body = serde_json::to_value(&params).unwrap();
        assert_eq!(body["n"], 2);
        assert_eq!(body["length"], serde_json::json!([6, 1]));
        assert_eq!(body["min"], serde_json::json!([0, 0]));
        assert_eq!(body["max"], serde_json::json!([9, 14]));
        assert_eq!(body["replacement"], serde_json::json!([true, true]));
    }

    /// 七星彩按位排列：顺序敏感、不排序；重复数字原样保留。
    #[test]
    fn test_qixing_balls_keep_order() {
        let resp = SignedIntegerSequencesResponse {
            data: vec![
                vec![json!(3), json!(0), json!(9), json!(3), json!(7), json!(1)],
                vec![json!(11)],
            ],
            completion_time: "2026-08-09 12:00:00Z".into(),
            serial_number: 43,
            hashed_api_key: "hash".into(),
            signature: "sig".into(),
            cost: 0.0,
            bits_used: 0,
            bits_left: 0,
            requests_left: 0,
            advisory_delay: 0,
        };
        let (front, back) = qixing_balls(&resp);
        assert_eq!(front, vec![3, 0, 9, 3, 7, 1]);
        assert_eq!(back, vec![11]);
        let (headline, meta) = format_qixing(&resp);
        assert_eq!(headline, "前区 03 00 09 03 07 01   后区 11");
        assert!(meta.contains("序列号 43"));
    }

    /// 错误提示的响应体预览：空体、短文本、超长文本截断。
    #[test]
    fn test_preview_text() {
        assert_eq!(preview_text(""), "响应体为空。");
        assert!(preview_text("not json").starts_with("原始响应：not json"));
        // 换行压缩 + 截断到 80 字符并补省略号
        let long = format!("line1\n{}", "a".repeat(200));
        let p = preview_text(&long);
        assert!(p.starts_with("原始响应：line1 aaaa"));
        assert!(p.ends_with('…'));
        assert!(p.len() < 130);
    }
}
