// JSON-RPC 2.0 协议封装。
// 参考: https://www.jsonrpc.org/specification
//
// 约束:
//   - 单向请求-响应（无通知、无批量）
//   - 错误码遵循 JSON-RPC 2.0 标准（-32700 ~ -32099 服务端自定义区间）
//   - stderr 不参与协议，仅供调试日志

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ── JSON-RPC 标准错误码 ──
pub const ERR_PARSE_ERROR: i32 = -32700;
pub const ERR_INVALID_REQUEST: i32 = -32600;
pub const ERR_METHOD_NOT_FOUND: i32 = -32601;
pub const ERR_INVALID_PARAMS: i32 = -32602;
pub const ERR_INTERNAL: i32 = -32603;

// ── 服务端自定义错误码（-32000 ~ -32099）──
pub const ERR_SESSION_NOT_FOUND: i32 = -32001;
pub const ERR_CASE_NOT_FOUND: i32 = -32002;
pub const ERR_INVALID_ACTION: i32 = -32003;
pub const ERR_GAME_ENDED: i32 = -32004;

/// JSON-RPC 错误对象
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl std::fmt::Display for RpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for RpcError {}

/// 标准错误码常量集合，供命令层判断
pub struct RpcErrorCode;
impl RpcErrorCode {
    pub const SESSION_NOT_FOUND: i32 = ERR_SESSION_NOT_FOUND;
    pub const CASE_NOT_FOUND: i32 = ERR_CASE_NOT_FOUND;
}

/// 构造 JSON-RPC 请求行（不含尾随换行）
pub fn build_request(id: &str, method: &str, params: &Value) -> String {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params,
    })
    .to_string()
}

/// 解析 JSON-RPC 响应，返回 Ok(result) 或 Err(RpcError)
pub fn parse_response(line: &str) -> Result<Value, RpcError> {
    let v: Value = serde_json::from_str(line).map_err(|e| RpcError {
        code: ERR_PARSE_ERROR,
        message: format!("Parse error: {}", e),
        data: None,
    })?;

    if let Some(err) = v.get("error") {
        let err: RpcError = serde_json::from_value(err.clone()).unwrap_or(RpcError {
            code: ERR_INTERNAL,
            message: "Malformed error object".to_string(),
            data: None,
        });
        return Err(err);
    }

    v.get("result")
        .cloned()
        .ok_or_else(|| RpcError {
            code: ERR_INTERNAL,
            message: "Response missing result field".to_string(),
            data: None,
        })
}
