// virtual-vet 根目录解析——sidecar spawn 与症状/疾病映射表读取共用的唯一实现。
//
// 解析顺序（解析不出就返回 None，由调用方各走自己的降级分支）:
//   1. 环境变量 VET_VET_ROOT：非空即采用，原样返回（保留原有 dev 行为）
//   2. 当前工作目录的同级目录 ../virtual-vet：存在则返回其绝对路径
//   3. None
//
// 第 2 级依赖启动进程的 cwd（tauri/cargo 可能把 cwd 落在 src-tauri，此时探测不到很正常），
// 打包/非同级布局部署请显式设 VET_VET_ROOT。

use std::env;
use std::path::{Component, Path, PathBuf};

/// 显式指定 virtual-vet 根目录的环境变量名
pub const ROOT_ENV: &str = "VET_VET_ROOT";
/// 未设环境变量时探测的同级目录名
pub const SIBLING_DIR: &str = "virtual-vet";

/// 解析 virtual-vet 项目根目录。
pub fn resolve_virtual_vet_root() -> Option<String> {
    let env_value = env::var(ROOT_ENV).ok();
    let cwd = env::current_dir().ok();
    resolve_from(env_value.as_deref(), cwd.as_deref())
}

/// `resolve_virtual_vet_root` 的纯函数形态：env 值与 cwd 由调用方注入，便于单测。
fn resolve_from(env_value: Option<&str>, cwd: Option<&Path>) -> Option<String> {
    if let Some(value) = env_value {
        if !value.trim().is_empty() {
            return Some(value.to_string());
        }
    }
    let sibling = normalize(&cwd?.join("..").join(SIBLING_DIR));
    if !sibling.is_dir() {
        return None;
    }
    Some(sibling.to_string_lossy().into_owned())
}

/// 纯词法折叠 `.` / `..`，不触碰文件系统。
/// 刻意不用 fs::canonicalize：Windows 下它会加 `\\?\` 前缀，Python 子进程处理不了。
fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Mutex, MutexGuard};

    // 进程级状态（环境变量 + cwd）不能并发修改，相关测试串行跑。
    static PROCESS_STATE: Mutex<()> = Mutex::new(());

    fn lock_process_state() -> MutexGuard<'static, ()> {
        PROCESS_STATE
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// 在 temp 下建一个唯一的空目录并返回路径（自带清理前缀）。
    fn make_sandbox(tag: &str) -> PathBuf {
        static SEQ: AtomicUsize = AtomicUsize::new(0);
        let dir = env::temp_dir()
            .join("vet-knowledge-root-tests")
            .join(format!(
                "{}-{}-{}",
                tag,
                std::process::id(),
                SEQ.fetch_add(1, Ordering::SeqCst)
            ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create sandbox");
        dir
    }

    #[test]
    fn env_var_is_used_verbatim_and_wins() {
        let _guard = lock_process_state();
        let custom = make_sandbox("env").join(SIBLING_DIR);
        fs::create_dir_all(&custom).expect("create custom root");

        let prev = env::var(ROOT_ENV);
        env::set_var(ROOT_ENV, &custom);
        let got = resolve_virtual_vet_root();
        match prev {
            Ok(v) => env::set_var(ROOT_ENV, v),
            Err(_) => env::remove_var(ROOT_ENV),
        }
        let _ = fs::remove_dir_all(custom.parent().unwrap());

        assert_eq!(got.as_deref(), Some(custom.to_str().unwrap()));
    }

    #[test]
    fn unset_without_sibling_returns_no_path_at_all() {
        let _guard = lock_process_state();
        let original = env::current_dir().expect("get cwd");
        // <sandbox>/deep/proj 下没有 <sandbox>/deep/virtual-vet，探测必然落空
        let proj = make_sandbox("unset").join("deep").join("proj");
        fs::create_dir_all(&proj).expect("create proj");

        let prev = env::var(ROOT_ENV);
        env::remove_var(ROOT_ENV);
        env::set_current_dir(&proj).expect("chdir sandbox");
        let got = resolve_virtual_vet_root();
        env::set_current_dir(&original).expect("restore cwd");
        if let Ok(v) = prev {
            env::set_var(ROOT_ENV, v);
        }
        let _ = fs::remove_dir_all(proj.parent().unwrap().parent().unwrap());

        // None 等价于"结果里既没有盘符也没有用户目录"——不存在任何写死的根
        let joined = got.unwrap_or_default();
        assert!(
            joined.is_empty(),
            "unset VET_VET_ROOT + no sibling must resolve to nothing, got: {:?}",
            joined
        );
    }

    #[test]
    fn unset_falls_back_to_sibling_of_cwd() {
        let sandbox = make_sandbox("sibling");
        let project = sandbox.join("vet-knowledge");
        let sibling = sandbox.join(SIBLING_DIR);
        fs::create_dir_all(&project).expect("create project");
        fs::create_dir_all(&sibling).expect("create sibling");

        let got = resolve_from(None, Some(&project));
        let _ = fs::remove_dir_all(&sandbox);

        // 词法折叠后不带 `..`，且只可能来自 cwd，不是任何写死的根
        assert_eq!(got.as_deref(), Some(sibling.to_str().unwrap()));
        assert!(!got.unwrap().contains(".."));
    }

    #[test]
    fn real_root_is_either_absent_or_the_sibling_probe_result() {
        let _guard = lock_process_state();
        let prev = env::var(ROOT_ENV);
        env::remove_var(ROOT_ENV);
        let got = resolve_virtual_vet_root();
        if let Ok(v) = prev {
            env::set_var(ROOT_ENV, v);
        }

        if let Some(root) = &got {
            assert!(root.ends_with(SIBLING_DIR), "unexpected root: {}", root);
        }
        // 与纯函数形态在同一 cwd 下必须一致（证明没有第二份实现）
        assert_eq!(got, resolve_from(None, env::current_dir().ok().as_deref()));
    }

    #[test]
    fn normalize_collapses_dot_and_dotdot() {
        assert_eq!(normalize(Path::new("a/b/../c/./d")), PathBuf::from("a/c/d"));
        // 多余的 `..` 在相对路径上按字面保留（pop 到空即停），不会 panic
        assert_eq!(normalize(Path::new("a/../..")), PathBuf::new());
    }
}
