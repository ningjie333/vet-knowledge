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
// 预留：以下未直接使用的光标准码镜像 JSON-RPC 2.0 规范与兄弟项目 virtual-vet 的协议，
// sidecar 侧会实际返回这些 code，常量保留供命令层后续按码分支使用。
pub const ERR_PARSE_ERROR: i32 = -32700;
#[allow(dead_code)]
pub const ERR_INVALID_REQUEST: i32 = -32600;
#[allow(dead_code)]
pub const ERR_METHOD_NOT_FOUND: i32 = -32601;
#[allow(dead_code)]
pub const ERR_INVALID_PARAMS: i32 = -32602;
pub const ERR_INTERNAL: i32 = -32603;

// ── 服务端自定义错误码（-32000 ~ -32099）──
// 预留：与兄弟项目 virtual-vet 的 sidecar 协议一一对应（会话/病例/动作/对局状态错误），
// 当前错误按 RpcError.code 透传给前端，常量保留供命令层后续判断使用。
#[allow(dead_code)]
pub const ERR_SESSION_NOT_FOUND: i32 = -32001;
#[allow(dead_code)]
pub const ERR_CASE_NOT_FOUND: i32 = -32002;
#[allow(dead_code)]
pub const ERR_INVALID_ACTION: i32 = -32003;
#[allow(dead_code)]
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
// 预留：命名空间入口镜像 virtual-vet 协议（协议常量以模块级 ERR_* 为准），命令层按码分支时从这里取。
#[allow(dead_code)]
pub struct RpcErrorCode;
// 预留：同上，关联常量当前未使用。
#[allow(dead_code)]
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
