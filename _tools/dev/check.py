#!/usr/bin/env python3
"""
跨项目代码一致性检查框架

用法:
  python check.py --config check_config.json           # 运行全部检查
  python check.py --config check_config.json --quick    # 仅运行规则 1+2
  python check.py --config check_config.json --full     # 运行全部检查
  python check.py --config check_config.json route_check  # 运行指定检查
  python check.py --init                                # 生成默认配置文件
  python check.py --schema                              # 输出配置 JSON Schema
  python check.py --list                                # 列出所有可用检查器
  python check.py --install-hook                        # 安装 Git pre-commit hook
"""

from __future__ import annotations

import argparse
import fnmatch
import importlib
import json
import os
import re
import shutil
import subprocess
import sys
import textwrap
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

TOOL_DIR = Path(__file__).resolve().parent
TOOLers_DIR = TOOL_DIR / "checkers"
SCHEMA_FILE = TOOL_DIR / "check_config_schema.json"

EXIT_OK = 0
EXIT_CONFIRMED = 1
EXIT_REVIEW = 2

VERSION = "1.0.0"

# Quick mode: only these checks
QUICK_CHECKS = ["route_check", "api_check"]

# All checkers in default order
ALL_CHECKS = ["route_check", "api_check", "empty_check", "hardcode_check", "dead_code_check", "type_check"]


# ---------------------------------------------------------------------------
# Data types
# ---------------------------------------------------------------------------

@dataclass
class CheckResult:
    """单个检查器的结果。"""
    passed: bool = True
    confirmed: list[str] = field(default_factory=list)
    review_needed: list[str] = field(default_factory=list)
    skipped: str = ""

    @property
    def has_findings(self) -> bool:
        return bool(self.confirmed) or bool(self.review_needed)


@dataclass
class Config:
    """解析后的配置。"""
    global_settings: dict[str, Any]
    checks: dict[str, dict[str, Any]]
    raw: dict[str, Any]

    @property
    def ignore_patterns(self) -> list[str]:
        return self.global_settings.get("ignore", [])


# ---------------------------------------------------------------------------
# .checkignore 解析
# ---------------------------------------------------------------------------

CHECKIGNORE_FILE = ".checkignore"


def load_checkignore(base_dir: Path) -> list[str]:
    """加载 .checkignore 文件，返回 glob 模式列表。"""
    ignore_file = base_dir / CHECKIGNORE_FILE
    if not ignore_file.exists():
        return []
    patterns = []
    content = ignore_file.read_text(encoding="utf-8", errors="replace")
    for line in content.splitlines():
        line = line.strip()
        if line and not line.startswith("#"):
            patterns.append(line)
    return patterns


def is_ignored(file_path: Path, patterns: list[str], base_dir: Path) -> bool:
    """检查文件是否匹配任何忽略模式。"""
    try:
        rel = str(file_path.relative_to(base_dir))
    except ValueError:
        rel = str(file_path)
    for pat in patterns:
        if fnmatch.fnmatch(rel, pat) or fnmatch.fnmatch(file_path.name, pat):
            return True
    return False


# ---------------------------------------------------------------------------
# 配置加载与验证
# ---------------------------------------------------------------------------

def load_config(config_path: Path, base_dir: Path | None = None) -> Config:
    """加载并解析配置文件。"""
    if not config_path.exists():
        print(f"[ERROR] 配置文件不存在: {config_path}", file=sys.stderr)
        sys.exit(1)

    content = config_path.read_text(encoding="utf-8", errors="replace")

    # 检测 JSON 或 TOML
    suffix = config_path.suffix.lower()
    if suffix in (".toml",):
        try:
            import tomllib
            raw = tomllib.loads(content)
        except ImportError:
            print("[ERROR] Python < 3.11 不支持 TOML 解析，请使用 .json 配置文件", file=sys.stderr)
            sys.exit(1)
    elif suffix in (".json",):
        raw = json.loads(content)
    else:
        # 尝试 JSON，失败则尝试 TOML
        try:
            raw = json.loads(content)
        except json.JSONDecodeError:
            try:
                import tomllib
                raw = tomllib.loads(content)
            except Exception:
                print(f"[ERROR] 无法解析配置文件（不支持的格式）: {config_path}", file=sys.stderr)
                sys.exit(1)

    # 分离全局配置和检查器配置
    check_sections = {}
    global_settings = {}
    for key, value in raw.items():
        if key.startswith("_") or key in ("project", "version", "ignore", "base_dir"):
            global_settings[key] = value
        elif isinstance(value, dict) and "enabled" in value:
            check_sections[key] = value
        elif isinstance(value, dict):
            check_sections[key] = value

    return Config(global_settings=global_settings, checks=check_sections, raw=raw)


# ---------------------------------------------------------------------------
# 检查器发现与加载
# ---------------------------------------------------------------------------

def list_checkers() -> dict[str, str]:
    """列出所有可用检查器。"""
    checkers = {}
    if not TOOLers_DIR.exists():
        return checkers
    for f in sorted(TOOLers_DIR.iterdir()):
        if f.suffix == ".py" and not f.name.startswith("_"):
            checkers[f.stem] = str(f.relative_to(TOOL_DIR))
    return checkers


def load_checker_module(checker_name: str):
    """动态加载检查器模块。"""
    module_path = TOOLers_DIR / f"{checker_name}.py"
    if not module_path.exists():
        print(f"[ERROR] 检查器 '{checker_name}' 不存在: {module_path}", file=sys.stderr)
        sys.exit(1)

    # 通过 importlib 加载
    import importlib.util
    spec = importlib.util.spec_from_file_location(f"checkers.{checker_name}", module_path)
    import importlib.util
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


# ---------------------------------------------------------------------------
# 输出格式化
# ---------------------------------------------------------------------------

class Colors:
    """ANSI 颜色（仅在支持终端输出时使用）。"""
    RED = "\033[0;31m"
    YELLOW = "\033[0;33m"
    GREEN = "\033[0;32m"
    CYAN = "\033[0;36m"
    DIM = "\033[2m"
    BOLD = "\033[1m"
    RESET = "\033[0m"

    @classmethod
    def disable(cls):
        cls.RED = cls.YELLOW = cls.GREEN = cls.CYAN = cls.DIM = cls.BOLD = cls.RESET = ""


def use_color() -> bool:
    """检测是否应使用颜色输出。"""
    if os.environ.get("NO_COLOR"):
        return False
    if os.environ.get("FORCE_COLOR"):
        return True
    return hasattr(sys.stdout, "isatty") and sys.stdout.isatty()


# ---------------------------------------------------------------------------
# 配置生成
# ---------------------------------------------------------------------------

DEFAULT_CONFIG_JSON = """{
  "_comment": "跨项目代码一致性检查框架配置文件",
  "project": "your-project-name",
  "base_dir": ".",
  "ignore": ["node_modules/**", "**/*.min.js", "**/*.min.css", "vendor/**", ".git/**"],

  "route_check": {
    "enabled": true,
    "framework": "vue",
    "route_file": "src/router/index.ts",
    "scan_dirs": ["src/views", "src/components"],
    "scan_ext": [".vue", ".ts", ".tsx", ".js"],
    "ignore": ["**/node_modules/**"]
  },

  "api_check": {
    "enabled": true,
    "api_type": "rest",
    "backend_route_file": "src/server/routes.py",
    "scan_dirs": ["src/api", "src/services"],
    "scan_ext": [".ts", ".tsx", ".js"],
    "ignore": ["**/node_modules/**"]
  },

  "type_check": {
    "enabled": false,
    "frontend_types_dir": "src/types",
    "backend_models_file": "server/models.py",
    "entity_map": {},
    "field_map": {},
    "ignore_fields": ["created_at", "updated_at", "deleted_at"]
  },

  "empty_check": {
    "enabled": true,
    "register_file": "src/server/handlers.py",
    "impl_dirs": ["src/server"],
    "impl_ext": [".py", ".rs", ".ts"]
  },

  "hardcode_check": {
    "enabled": true,
    "scan_dirs": ["src"],
    "scan_ext": [".py", ".ts", ".js"],
    "threshold": 5,
    "min_entries": 3,
    "allow_names": ["STATUS_*", "HTTP_*", "ERROR_*"]
  },

  "dead_code_check": {
    "enabled": true,
    "schema_file": "migrations/schema.sql",
    "code_dirs": ["src"],
    "code_ext": [".rs", ".py", ".ts", ".js"],
    "exclude_tables": ["__diesel_schema_migrations", "schema_migrations"]
  }
}
"""


def init_config(output_path: Path) -> None:
    """生成默认配置文件。"""
    if output_path.exists():
        print(f"[WARN] 配置文件已存在: {output_path}")
        resp = input("覆盖? (y/N): ").strip().lower()
        if resp != "y":
            print("已取消")
            return

    output_path.write_text(DEFAULT_CONFIG_JSON, encoding="utf-8")
    print(f"[OK] 已生成默认配置文件: {output_path}")
    print("请编辑配置文件以匹配你的项目结构")


# ---------------------------------------------------------------------------
# 配置 Schema 输出
# ---------------------------------------------------------------------------

def print_schema() -> None:
    """输出配置文件的 JSON Schema。"""
    schema_path = TOOL_DIR / "check_config_schema.json"
    if schema_path.exists():
        print(schema_path.read_text(encoding="utf-8"))
    else:
        print("[WARN] schema 文件不存在，使用内嵌默认值")
        print(DEFAULT_CONFIG_JSON)


# ---------------------------------------------------------------------------
# Git Hook 安装
# ---------------------------------------------------------------------------

def _generate_hook(git_root: Path) -> str:
    """生成跨平台兼容的 pre-commit hook 内容。"""
    # 使用 Python 驱动，兼容 Windows / macOS / Linux
    script_dir = git_root / "_tools" / "dev"
    # 计算从 git root 到 check.py 的相对路径（用正斜杠避免转义问题）
    check_rel = (script_dir / "check.py").relative_to(git_root).as_posix()

    hook = f"""#!/usr/bin/env python3
# Git pre-commit hook - 代码一致性检查
# 由 check.py --install-hook 自动生成
# 兼容 Windows / macOS / Linux

import subprocess
import sys
from pathlib import Path

GIT_ROOT = Path(r"{git_root}")
CONFIG_FILE = GIT_ROOT / "check_config.json"
CHECK_SCRIPT = GIT_ROOT / "{check_rel}"

if not CONFIG_FILE.exists():
    print("[check] 跳过：未找到 check_config.json")
    sys.exit(0)

if not CHECK_SCRIPT.exists():
    print("[check] 跳过：未找到检查脚本")
    sys.exit(0)

print("[check] 运行代码一致性检查...")

result = subprocess.run(
    [sys.executable, str(CHECK_SCRIPT), "--config", str(CONFIG_FILE), "--quick",
     "--no-color"],
    cwd=str(GIT_ROOT),
)

if result.returncode == 1:
    print("[check] 发现确定的兼容性问题，请修复后再提交")
    print("[check] 若要强制提交，使用 git commit --no-verify")
    sys.exit(1)
elif result.returncode == 2:
    print("[check] 有需要人工确认的项目（仅警告，不阻止提交）")
    sys.exit(0)

print("[check] 检查通过")
sys.exit(0)
"""
    return hook


def install_git_hook() -> None:
    """安装 Git pre-commit hook（跨平台兼容）。"""
    try:
        result = subprocess.run(
            ["git", "rev-parse", "--show-toplevel"],
            capture_output=True, text=True, check=True,
        )
        git_root = Path(result.stdout.strip())
    except (subprocess.CalledProcessError, FileNotFoundError):
        print("[ERROR] 不在 git 仓库中或 git 未安装")
        sys.exit(1)

    hooks_dir = git_root / ".git" / "hooks"
    if not hooks_dir.exists():
        print(f"[ERROR] .git/hooks 目录不存在: {hooks_dir}")
        sys.exit(1)

    # 写两个文件：pre-commit（无扩展名，POSIX 用）和 pre-commit.py（Windows 用）
    hook_py = hooks_dir / "pre-commit.py"
    hook_sh = hooks_dir / "pre-commit"

    # 生成 Python hook 内容
    hook_content = _generate_hook(git_root)

    # 写 .py 文件（Windows 直接可用）
    hook_py.write_text(hook_content, encoding="utf-8")

    # 写 shell 脚本（POSIX 系统通过 python3 调用 .py）
    sh_content = f"""#!/bin/sh
# Git pre-commit hook - 代码一致性检查
# 由 check.py --install-hook 自动生成
# 通过 Python 脚本驱动，兼容 Windows / macOS / Linux

python3 "{hook_py}" 2>/dev/null || python "{hook_py}" 2>/dev/null || echo "[check] 警告：无法运行 python，跳过检查"
"""
    hook_sh.write_text(sh_content, encoding="utf-8")

    # 设置可执行权限（POSIX）
    try:
        hook_sh.chmod(0o755)
        hook_py.chmod(0o755)
    except OSError:
        pass

    # Windows：还需要配置 git 使用 .py 扩展名
    if sys.platform == "win32":
        try:
            subprocess.run(
                ["git", "config", "core.hooksPath", str(hooks_dir)],
                capture_output=True, text=True, cwd=str(git_root),
            )
            # 创建 .git/hooks/pre-commit.bat 作为备选
            bat_content = f'@echo off\npython "{hook_py}"\n'
            (hooks_dir / "pre-commit.bat").write_text(bat_content, encoding="utf-8")
            print("[OK] 已为 Windows 安装 .bat hook")
        except Exception:
            pass

    print(f"[OK] pre-commit hook 已安装:")
    print(f"  Python: {hook_py}")
    print(f"  Shell:  {hook_sh}")
    if sys.platform == "win32":
        print(f"  Batch:  {hooks_dir / 'pre-commit.bat'}")
    print("提示：git commit --no-verify 可跳过检查")


# ---------------------------------------------------------------------------
# 主运行逻辑
# ---------------------------------------------------------------------------

def run_checker(checker_name: str, check_config: dict[str, Any], base_dir: Path) -> CheckResult:
    """加载并运行单个检查器。"""
    # 禁用检查
    if not check_config.get("enabled", True):
        return CheckResult(skipped="已在配置中禁用")

    try:
        mod = load_checker_module(checker_name)
    except Exception as e:
        return CheckResult(
            passed=False,
            confirmed=[f"检查器 '{checker_name}' 加载失败: {e}"],
        )

    if not hasattr(mod, "run"):
        return CheckResult(
            passed=False,
            confirmed=[f"检查器 '{checker_name}' 未导出 run() 函数"],
        )

    try:
        result = mod.run(check_config)
        if not isinstance(result, CheckResult):
            # 也接受 checkers.CheckResult
            if hasattr(result, "passed"):
                result = CheckResult(
                    passed=result.passed,
                    confirmed=list(result.confirmed),
                    review_needed=list(result.review_needed),
                    skipped=str(result.skipped),
                )
            else:
                return CheckResult(
                    passed=False,
                    confirmed=[f"检查器 '{checker_name}' 返回了无效结果类型"],
                )
        return result
    except Exception as e:
        import traceback
        return CheckResult(
            passed=False,
            confirmed=[f"检查器 '{checker_name}' 运行异常: {e}\n{traceback.format_exc()}"],
        )


def format_results(
    checker_name: str,
    result: CheckResult,
    checker_label: str = "",
) -> tuple[bool, bool]:
    """格式化输出单个检查器的结果。

    Returns:
        (has_confirmed, has_review)
    """
    label = checker_label or checker_name
    has_confirmed = bool(result.confirmed)
    has_review = bool(result.review_needed)

    if result.skipped:
        print(f"  {Colors.DIM}[SKIP] {label} -> {result.skipped}{Colors.RESET}")
        return False, False

    if not result.has_findings:
        print(f"  {Colors.GREEN}[PASS] {label} ({len(result.confirmed)} 问题){Colors.RESET}")
    else:
        status = Colors.BOLD + Colors.RED + "FAIL" if has_confirmed else Colors.YELLOW + "WARN"
        total = len(result.confirmed) + len(result.review_needed)
        print(f"  {status}[{label}]{Colors.RESET} {total} 个问题")

    for item in result.confirmed:
        # 格式化：path:line -> desc
        print(f"    {Colors.RED}[CONFIRMED]{Colors.RESET} {item}")

    for item in result.review_needed:
        print(f"    {Colors.YELLOW}[REVIEW]{Colors.RESET}    {item}")

    return has_confirmed, has_review


def run_all_checks(config: Config, checkers_to_run: list[str] | None = None) -> int:
    """运行所有指定的检查器，返回退出码。"""
    checkers_to_run = checkers_to_run or list(config.checks.keys())

    total_confirmed = 0
    total_review = 0
    any_skipped = False

    print(f"\n{Colors.BOLD}=== 代码一致性检查 ({len(checkers_to_run)} 项) ==={Colors.RESET}\n")

    # 检查器名称中文映射
    label_map = {
        "route_check": "路由死链检查",
        "api_check": "API 一致性检查",
        "type_check": "类型一致性检查",
        "empty_check": "空实现检查",
        "hardcode_check": "硬编码数据检查",
        "dead_code_check": "废弃代码检查",
    }

    for checker_name in checkers_to_run:
        # 确定配置 section
        check_config = config.checks.get(checker_name, {})

        # 合并全局 ignore
        global_ignore = config.ignore_patterns
        check_ignore = check_config.get("ignore", [])
        merged_config = {**check_config, "ignore": global_ignore + check_ignore}

        result = run_checker(checker_name, merged_config, Path.cwd())
        label = label_map.get(checker_name, checker_name)

        has_confirmed, has_review = format_results(checker_name, result, label)

        if result.skipped:
            any_skipped = True
        total_confirmed += len(result.confirmed)
        total_review += len(result.review_needed)

    # 汇总
    print(f"\n{Colors.BOLD}--- 汇总 ---{Colors.RESET}")
    print(f"  检查器数: {len(checkers_to_run)}")
    print(f"  {Colors.RED}CONFIRMED: {total_confirmed}{Colors.RESET}")
    print(f"  {Colors.YELLOW}REVIEW:    {total_review}{Colors.RESET}")

    if total_confirmed > 0:
        print(f"\n{Colors.RED}结果: 失败（有 {total_confirmed} 个确定问题需要修复）{Colors.RESET}")
        return EXIT_CONFIRMED
    elif total_review > 0:
        print(f"\n{Colors.YELLOW}结果: 通过（有 {total_review} 个项目需人工确认）{Colors.RESET}")
        return EXIT_REVIEW
    else:
        print(f"\n{Colors.GREEN}结果: 全部通过{Colors.RESET}")
        return EXIT_OK


# ---------------------------------------------------------------------------
# CLI 入口
# ---------------------------------------------------------------------------

def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="check.py",
        description="跨项目代码一致性检查框架",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=textwrap.dedent("""\
            退出码:
              0 - 通过（无问题或仅有跳过）
              1 - 有 CONFIRMED 确定的问题需要修复
              2 - 仅有 REVIEW_NEEDED 项目需人工确认

            示例:
              python check.py --init
              python check.py --config check_config.json --quick
              python check.py --config check_config.json route_check
        """),
    )
    parser.add_argument("--config", "-c", type=Path, help="配置文件路径 (.json 或 .toml)")
    parser.add_argument("--quick", action="store_true", help="仅运行快速检查（规则 1+2）")
    parser.add_argument("--full", action="store_true", help="运行全部检查")
    parser.add_argument("--init", action="store_true", help="生成默认配置文件")
    parser.add_argument("--schema", action="store_true", help="输出配置 JSON Schema")
    parser.add_argument("--list", action="store_true", help="列出所有可用检查器")
    parser.add_argument("--install-hook", action="store_true", help="安装 Git pre-commit hook")
    parser.add_argument("--no-color", action="store_true", help="禁用彩色输出")
    parser.add_argument("check_name", nargs="?", help="仅运行指定的检查器")
    return parser


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()

    # 颜色设置
    if args.no_color or not use_color():
        Colors.disable()

    # --list
    if args.list:
        checkers = list_checkers()
        print("可用检查器:")
        for name, path in checkers.items():
            desc = {
                "route_check": "路由死链检查",
                "api_check": "API 调用一致性检查",
                "type_check": "类型一致性检查 (默认关闭)",
                "empty_check": "空实现检查",
                "hardcode_check": "硬编码数据检查",
                "dead_code_check": "废弃代码检查",
            }
            label = desc.get(name, "")
            print(f"  {name:<20} {label}")
        return EXIT_OK

    # --schema
    if args.schema:
        print_schema()
        return EXIT_OK

    # --init
    if args.init:
        output = Path(args.config) if args.config else Path("check_config.json")
        init_config(output)
        return EXIT_OK

    # --install-hook
    if args.install_hook:
        install_git_hook()
        return EXIT_OK

    # 需要 --config
    config_path = args.config
    if not config_path:
        # 尝试查找默认配置文件
        candidates = [
            Path("check_config.json"),
            Path("check_config.toml"),
            Path("_tools/dev/check_config.json"),
        ]
        for c in candidates:
            if c.exists():
                config_path = c
                break
        if not config_path:
            parser.error("请指定 --config，或使用 --init 生成配置文件")

    # 加载配置
    config = load_config(config_path, config_path.parent)

    # 确定要运行的检查器
    if args.check_name:
        checkers_to_run = [args.check_name]
    elif args.quick:
        checkers_to_run = [c for c in QUICK_CHECKS if c in config.checks]
        if not checkers_to_run:
            print("[ERROR] quick 模式需要 route_check 或 api_check 的配置", file=sys.stderr)
            return EXIT_CONFIRMED
    elif args.full:
        checkers_to_run = [c for c in ALL_CHECKS if c in config.checks]
    else:
        # 默认：运行配置中存在的所有检查器
        checkers_to_run = list(config.checks.keys())

    if not checkers_to_run:
        print("[WARN] 没有找到任何可运行的检查器。请检查配置文件。", file=sys.stderr)
        return EXIT_OK

    # 运行检查
    exit_code = run_all_checks(config, checkers_to_run)
    return exit_code


if __name__ == "__main__":
    sys.exit(main())
