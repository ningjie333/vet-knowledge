// virtual-vet sidecar 进程管理器。
//
// 职责:
//   - 懒启动 Python sidecar 子进程（首次 call 时 spawn）
//   - 互斥锁保护 stdin/stdout（避免并发写 stdin 导致响应错位）
//   - 提供 async call(method, params) -> Result<Value, RpcError>
//   - shutdown() 优雅退出 sidecar
//
// 环境变量（dev 模式）:
//   VET_VET_PYTHON  - Python 可执行路径（默认 "python"）
//   VET_VET_ROOT    - virtual-vet 项目根目录
//   VET_VET_SIDECAR_LOG - sidecar 日志级别（默认 "INFO"）

use std::env;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use serde_json::Value;
use tokio::sync::Mutex;

use super::protocol::{build_request, parse_response, RpcError, ERR_INTERNAL};

/// sidecar 进程管理器（单例，由 lib.rs 注入 Tauri State）
pub struct SidecarManager {
    inner: Mutex<Inner>,
}

struct Inner {
    child: Option<Child>,
    stdin: Option<std::process::ChildStdin>,
    stdout: Option<BufReader<std::process::ChildStdout>>,
    call_counter: u64,
}

impl SidecarManager {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                child: None,
                stdin: None,
                stdout: None,
                call_counter: 0,
            }),
        }
    }

    /// 调用 sidecar RPC 方法。懒启动 sidecar 进程。
    pub async fn call(&self, method: &str, params: Value) -> Result<Value, RpcError> {
        let mut inner = self.inner.lock().await;

        // 懒启动
        if inner.child.is_none() {
            inner.spawn().map_err(|e| RpcError {
                code: ERR_INTERNAL,
                message: format!("Failed to start sidecar: {}", e),
                data: None,
            })?;
        }

        inner.call_counter += 1;
        let id = format!("rpc_{}", inner.call_counter);
        let req = build_request(&id, method, &params);

        // 写请求
        let stdin = inner.stdin.as_mut().ok_or_else(|| RpcError {
            code: ERR_INTERNAL,
            message: "sidecar stdin not available".to_string(),
            data: None,
        })?;
        stdin.write_all(req.as_bytes()).map_err(io_err)?;
        stdin.write_all(b"\n").map_err(io_err)?;
        stdin.flush().map_err(io_err)?;

        // 读响应
        let stdout = inner.stdout.as_mut().ok_or_else(|| RpcError {
            code: ERR_INTERNAL,
            message: "sidecar stdout not available".to_string(),
            data: None,
        })?;
        let mut line = String::new();
        let n = stdout.read_line(&mut line).map_err(io_err)?;
        if n == 0 {
            return Err(RpcError {
                code: ERR_INTERNAL,
                message: "sidecar closed stdout (process may have crashed)".to_string(),
                data: None,
            });
        }

        parse_response(line.trim())
    }

    /// 关闭 sidecar 进程（vet-knowledge 退出前调用）
    pub async fn shutdown(&self) -> Result<(), String> {
        let mut inner = self.inner.lock().await;
        if inner.child.is_some() {
            // 尝试发 shutdown RPC（best effort，失败也继续 kill）
            if let Some(stdin) = inner.stdin.as_mut() {
                let req = build_request("shutdown_id", "shutdown", &serde_json::json!({}));
                let _ = stdin.write_all(req.as_bytes());
                let _ = stdin.write_all(b"\n");
                let _ = stdin.flush();
            }
            // 等待进程退出（最多 2 秒）
            if let Some(mut child) = inner.child.take() {
                for _ in 0..20 {
                    match child.try_wait() {
                        Ok(Some(_)) => break,
                        Ok(None) => std::thread::sleep(Duration::from_millis(100)),
                        Err(_) => break,
                    }
                }
                // 强制杀
                let _ = child.kill();
                let _ = child.wait();
            }
        }
        inner.stdin = None;
        inner.stdout = None;
        Ok(())
    }
}

impl Inner {
    fn spawn(&mut self) -> Result<(), String> {
        let python = env::var("VET_VET_PYTHON").unwrap_or_else(|_| "python".to_string());
        let project_root = env::var("VET_VET_ROOT").unwrap_or_else(|_| {
            // dev 默认值（可被环境变量覆盖）
            r"C:\Users\ZhuanZ（无密码）\Desktop\Claudecode\01_代码实验\virtual-vet".to_string()
        });
        let script = format!("{}/sidecar_entry.py", project_root);

        let mut child = Command::new(&python)
            .arg(&script)
            .env("VIRTUAL_VET_ROOT", &project_root)
            .env("PYTHONPATH", &project_root)
            .env("PYTHONIOENCODING", "utf-8")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit()) // dev 模式 stderr 直通控制台
            .current_dir(&project_root)
            .spawn()
            .map_err(|e| format!("spawn failed (python='{}' script='{}'): {}", python, script, e))?;

        let stdin = child.stdin.take().ok_or("stdin take failed")?;
        let stdout = child.stdout.take().ok_or("stdout take failed")?;
        let mut reader = BufReader::new(stdout);

        // 读 ready 信号（sidecar 启动后立即发送）
        // 注意: 这是阻塞读。若 sidecar 启动失败会卡住——已知限制，dev 模式可接受。
        // 未来可用 tokio::process::Child + tokio::io::AsyncBufReadExt 改为异步。
        let mut ready_line = String::new();
        reader
            .read_line(&mut ready_line)
            .map_err(|e| format!("read ready signal: {}", e))?;
        let ready: Value = serde_json::from_str(ready_line.trim())
            .map_err(|e| format!("parse ready signal '{}': {}", ready_line.trim(), e))?;
        if ready.get("method").and_then(|v| v.as_str()) != Some("ready") {
            return Err(format!("unexpected ready signal: {}", ready_line.trim()));
        }

        self.child = Some(child);
        self.stdin = Some(stdin);
        self.stdout = Some(reader);
        Ok(())
    }
}

fn io_err(e: std::io::Error) -> RpcError {
    RpcError {
        code: ERR_INTERNAL,
        message: format!("IO error: {}", e),
        data: None,
    }
}
