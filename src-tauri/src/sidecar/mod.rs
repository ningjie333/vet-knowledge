// virtual-vet sidecar 进程管理 + JSON-RPC 2.0 over stdio 封装。
//
// 设计原则:
//   - 懒启动: vet-knowledge 启动时不拉起 Python，首次 game_new_session 时 spawn
//   - 单实例: 全局只有一个 sidecar 进程（多会话共用）
//   - 互斥锁: 每次调用独占 stdin/stdout（避免多线程并发写 stdin 导致响应错位）
//   - stderr 直通: 不参与协议，仅供调试（通过 VET_VET_SIDECAR_LOG 控制级别）
//
// 协议: 单向请求-响应（无通知、无批量）；每行一个请求，每行一个响应。
// sidecar 启动后先发 ready 信号: {"jsonrpc":"2.0","method":"ready","params":{"version":1}}

pub mod protocol;
pub mod manager;

pub use manager::SidecarManager;
pub use protocol::{RpcError, RpcErrorCode};
