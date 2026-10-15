// virtual-vet sidecar 进程管理器。
//
// 职责:
//   - 懒启动 Python sidecar 子进程（首次 call 时 spawn）
//   - 互斥锁保护 stdin/stdout（避免并发写 stdin 导致响应错位）
//   - 提供 async call(method, params) -> Result<Value, RpcError>
//   - 超时保护（method_timeout 按 method 分级）
//   - 崩溃自动重启（spawn_with_rate_limit，5s 内 ≥3 次放弃）
//   - IO 错误/超时自动清理（invalidate，下次 call 自动重启）
//   - stderr piped + 监控 task（eprintln 输出，便于调试）
//   - shutdown() 优雅退出 sidecar（发 RPC + 等 2s + 强制 kill）
//   - Drop 时 start_kill 兜底清理 child
//
// 环境变量（dev 模式）:
//   VET_VET_PYTHON  - Python 可执行路径（默认 "python"）
//   VET_VET_ROOT    - virtual-vet 项目根目录（未设则探测 cwd 同级的 ../virtual-vet，见 sidecar/root.rs）
//   VET_VET_SIDECAR_LOG - sidecar 日志级别（默认 "INFO"）

use std::env;
use std::process::Stdio;
use std::time::Instant;

use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;
use tokio::time::{timeout, Duration};

use super::protocol::{build_request, parse_response, RpcError, ERR_INTERNAL};
use super::root::{resolve_virtual_vet_root, ROOT_ENV, SIBLING_DIR};

/// sidecar 进程管理器（单例，由 lib.rs 注入 Tauri State）
pub struct SidecarManager {
    inner: Mutex<Inner>,
}

struct Inner {
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    stdout: Option<BufReader<ChildStdout>>,
    call_counter: u64,
    /// 最近重启时间戳，用于频率限制（避免重启风暴）
    restart_history: Vec<Instant>,
}

impl SidecarManager {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                child: None,
                stdin: None,
                stdout: None,
                call_counter: 0,
                restart_history: Vec::new(),
            }),
        }
    }

    /// 调用 sidecar RPC 方法。懒启动 sidecar 进程。
    /// 自动检测崩溃并重启（带频率限制）。IO 错误/超时自动清理状态。
    pub async fn call(&self, method: &str, params: Value) -> Result<Value, RpcError> {
        let mut inner = self.inner.lock().await;

        // 健康检查：若 child 已退出则清理状态（下次 spawn 重启）
        if let Some(child) = inner.child.as_mut() {
            match child.try_wait() {
                Ok(Some(_)) => inner.invalidate().await, // 已退出
                Ok(None) => {}                           // 仍在运行
                Err(_) => inner.invalidate().await,
            }
        }

        // 懒启动（带重启频率限制）
        if inner.child.is_none() {
            inner
                .spawn_with_rate_limit(method)
                .await
                .map_err(|e| RpcError {
                    code: ERR_INTERNAL,
                    message: format!("Failed to start sidecar: {}", e),
                    data: None,
                })?;
        }

        inner.call_counter += 1;
        let id = format!("rpc_{}", inner.call_counter);
        let req = build_request(&id, method, &params);

        let timeout_secs = method_timeout(method);

        // 整体包 timeout：写请求 + 读响应
        let io_result = timeout(Duration::from_secs(timeout_secs), inner.do_call(&req)).await;

        match io_result {
            Ok(Ok(v)) => Ok(v),
            Ok(Err(e)) => {
                // IO 错误/EOF（ERR_INTERNAL）需要清理；parse 错误/RPC 错误不需要
                if e.code == ERR_INTERNAL {
                    inner.invalidate().await;
                }
                Err(e)
            }
            Err(_) => {
                // 超时：清理状态后返回错误
                inner.invalidate().await;
                Err(RpcError {
                    code: ERR_INTERNAL,
                    message: format!("sidecar timeout after {}s", timeout_secs),
                    data: None,
                })
            }
        }
    }

    /// 关闭 sidecar 进程（vet-knowledge 退出前调用）
    /// 先发 shutdown RPC（best effort），再优雅等待 2s，超时则强制 kill。
    pub async fn shutdown(&self) -> Result<(), String> {
        let mut inner = self.inner.lock().await;
        if inner.child.is_some() {
            // 尝试发 shutdown RPC（best effort，失败也继续 kill）
            if let Some(stdin) = inner.stdin.as_mut() {
                let req = build_request("shutdown_id", "shutdown", &serde_json::json!({}));
                let _ = stdin.write_all(req.as_bytes()).await;
                let _ = stdin.write_all(b"\n").await;
                let _ = stdin.flush().await;
            }
            // 优雅等待最多 2 秒
            if let Some(mut child) = inner.child.take() {
                match timeout(Duration::from_secs(2), child.wait()).await {
                    Ok(Ok(_)) => {} // 已正常退出
                    _ => {
                        // 超时或错误，强制杀
                        let _ = child.kill().await;
                        let _ = child.wait().await;
                    }
                }
            }
        }
        inner.stdin = None;
        inner.stdout = None;
        Ok(())
    }
}

impl Drop for SidecarManager {
    fn drop(&mut self) {
        // Drop 不能 await，用 try_lock + start_kill（同步发 SIGKILL/TerminateProcess）
        // tokio::process::Child 的 start_kill() 是同步方法；wait() 由 Child 自身 Drop 处理。
        if let Ok(mut inner) = self.inner.try_lock() {
            if let Some(child) = inner.child.as_mut() {
                let _ = child.start_kill();
            }
            // stdin/stdout drop 会自动关闭管道
            inner.child = None;
            inner.stdin = None;
            inner.stdout = None;
        }
    }
}

impl Inner {
    /// 强制清理 child + stdin + stdout（kill + wait）
    async fn invalidate(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
        self.stdin = None;
        self.stdout = None;
    }

    /// 带重启频率限制的 spawn（5 秒内 ≥3 次则放弃，避免重启风暴）
    async fn spawn_with_rate_limit(&mut self, method: &str) -> Result<(), String> {
        let now = Instant::now();
        let window = Duration::from_secs(5);
        // 清理窗口外的记录
        self.restart_history
            .retain(|t| now.duration_since(*t) < window);
        if self.restart_history.len() >= 3 {
            return Err("sidecar restarting too fast, giving up".to_string());
        }
        self.restart_history.push(now);
        self.spawn(method).await
    }

    /// 拉起 Python sidecar 子进程，读 ready 信号。
    /// stderr piped + 监控 task 持续 eprintln 输出。
    /// ready 信号读取带 15s 超时。
    async fn spawn(&mut self, method: &str) -> Result<(), String> {
        let python = env::var("VET_VET_PYTHON").unwrap_or_else(|_| "python".to_string());
        let project_root = resolve_virtual_vet_root().ok_or_else(|| {
            let checked = env::current_dir()
                .map(|dir| dir.join("..").join(SIBLING_DIR))
                .unwrap_or_else(|_| std::path::PathBuf::from("..").join(SIBLING_DIR));
            format!(
                "virtual-vet root not found while serving RPC method '{}': set {} to a virtual-vet \
                 checkout, or put the checkout next to the current dir (checked '{}'); \
                 also verify sidecar_entry.py exists in that root",
                method, ROOT_ENV, checked.display()
            )
        })?;
        let script = format!("{}/sidecar_entry.py", project_root);

        let mut child = Command::new(&python)
            .arg(&script)
            .env("VIRTUAL_VET_ROOT", &project_root)
            .env("PYTHONPATH", &project_root)
            .env("PYTHONIOENCODING", "utf-8")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .current_dir(&project_root)
            .spawn()
            .map_err(|e| {
                format!(
                    "spawn failed (python='{}' script='{}'): {}",
                    python, script, e
                )
            })?;

        let stdin = child.stdin.take().ok_or("stdin take failed")?;
        let stdout = child.stdout.take().ok_or("stdout take failed")?;
        let stderr = child.stderr.take().ok_or("stderr take failed")?;

        // stderr 监控 task：持续读并 eprintln（简化版，不存 ring buffer）
        tokio::spawn(async move {
            let mut reader = BufReader::new(stderr);
            loop {
                let mut line = String::new();
                match reader.read_line(&mut line).await {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        eprintln!("[sidecar stderr] {}", line.trim_end());
                    }
                    Err(_) => break,
                }
            }
        });

        let mut reader = BufReader::new(stdout);

        // 读 ready 信号（sidecar 启动后立即发送），带 15s 超时
        let mut ready_line = String::new();
        timeout(Duration::from_secs(15), reader.read_line(&mut ready_line))
            .await
            .map_err(|_| "read ready signal timeout (15s)".to_string())?
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

    /// 执行一次 RPC 请求-响应（写 stdin + 读 stdout）。
    /// 由 call() 包 timeout 调用，IO 错误返回 ERR_INTERNAL。
    async fn do_call(&mut self, req: &str) -> Result<Value, RpcError> {
        // 写请求
        let stdin = self.stdin.as_mut().ok_or_else(|| RpcError {
            code: ERR_INTERNAL,
            message: "sidecar stdin not available".to_string(),
            data: None,
        })?;
        stdin.write_all(req.as_bytes()).await.map_err(io_err)?;
        stdin.write_all(b"\n").await.map_err(io_err)?;
        stdin.flush().await.map_err(io_err)?;

        // 读响应
        let stdout = self.stdout.as_mut().ok_or_else(|| RpcError {
            code: ERR_INTERNAL,
            message: "sidecar stdout not available".to_string(),
            data: None,
        })?;
        let mut line = String::new();
        let n = stdout.read_line(&mut line).await.map_err(io_err)?;
        if n == 0 {
            return Err(RpcError {
                code: ERR_INTERNAL,
                message: "sidecar closed stdout (process may have crashed)".to_string(),
                data: None,
            });
        }

        parse_response(line.trim())
    }
}

/// 按 method 分级返回超时秒数
fn method_timeout(method: &str) -> u64 {
    match method {
        "game.new_session" => 30,
        "game.diagnose" => 15,
        _ => 10,
    }
}

fn io_err(e: std::io::Error) -> RpcError {
    RpcError {
        code: ERR_INTERNAL,
        message: format!("IO error: {}", e),
        data: None,
    }
}
