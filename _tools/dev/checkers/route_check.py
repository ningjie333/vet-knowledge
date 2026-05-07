"""
规则 1: 路由死链检查

扫描前端代码中的路由引用（router-link, router.push, Link, navigate, Route），
与路由注册文件中声明的路径对比，找出不存在的路径引用。

支持框架：Vue（router-link / router.push）、React（Link / navigate / Route）

配置项 (对应 check_config.json 中的 "route_check" section):
    enabled         : bool,  默认 True
    framework       : str,   必填，"vue" 或 "react"
    route_file      : str,   必填，路由注册文件路径
    scan_dirs       : [str], 要扫描的源码目录列表
    scan_ext        : [str], 扫描的文件扩展名，默认 [.vue, .ts, .tsx, .js, .jsx]
    ignore          : [str], 忽略路径 glob 列表

误报说明:
    - 动态拼接路径（如 `/prefix/${var}`）标记为 REVIEW_NEEDED
    - 按 name 导航无法静态解析，标记为 REVIEW_NEEDED
    - 路由守卫动态添加的路由无法静态检测，标记为 REVIEW_NEEDED
"""

from __future__ import annotations

import re
from pathlib import Path
from typing import Optional

from checkers import CheckResult

# ---------------------------------------------------------------------------
# 路径收集：从路由注册文件中提取所有已声明的路径
# ---------------------------------------------------------------------------

# Vue Router 常见模式
_VUE_ROUTE_PATH = re.compile(
    r"""(?:path|alias)\s*:\s*["']([^"']+)["']"""
)
# React Router 常见模式
_REACT_ROUTE_PATH = re.compile(
    r"""path\s*[=:]\s*["']([^"']+)["']|path:\s*["']([^"']+)["']|<Route[^>]*path\s*=\s*["']([^"']+)["']"""
)
# 通用 routes 数组中的路径
_GENERIC_ROUTE_PATH = re.compile(
    r"""["']path["']\s*:\s*["']([^"']+)["']"""
)


def _extract_route_paths_vue(route_file: Path) -> set[str]:
    content = route_file.read_text(encoding="utf-8", errors="replace")
    paths = set()
    for m in _VUE_ROUTE_PATH.finditer(content):
        val = m.group(1)
        if val:
            paths.add(val)
    for m in _GENERIC_ROUTE_PATH.finditer(content):
        val = m.group(1)
        if val:
            paths.add(val)
    return paths


def _extract_route_paths_react(route_file: Path) -> set[str]:
    content = route_file.read_text(encoding="utf-8", errors="replace")
    paths = set()
    for m in _REACT_ROUTE_PATH.finditer(content):
        for g in m.groups():
            if g:
                paths.add(g)
    for m in _GENERIC_ROUTE_PATH.finditer(content):
        val = m.group(1)
        if val:
            paths.add(val)
    return paths


def extract_declared_paths(route_file: Path, framework: str) -> set[str]:
    """从路由注册文件中提取所有已声明的路径集合。"""
    if framework == "vue":
        return _extract_route_paths_vue(route_file)
    elif framework == "react":
        return _extract_route_paths_react(route_file)
    else:
        # 通用：尝试所有模式
        paths = _extract_route_paths_vue(route_file)
        paths.update(_extract_route_paths_react(route_file))
        return paths


# ---------------------------------------------------------------------------
# 引用扫描：从前端源码中提取所有路由引用
# ---------------------------------------------------------------------------

# Vue 模式
_VUE_ROUTER_LINK = re.compile(
    r"""<router-link\s+[^:]*\bto\s*=\s*["']([^"']+)["']"""
)
_VUE_ROUTER_LINK_DYNAMIC = re.compile(
    r"""<router-link\s+[^:]*\bto\s*=\s*["']\s*\+\s*\w+|\bto\s*=\s*["'][^"']*["']\s*\+\s*\w+"""
)
_VUE_ROUTER_PUSH = re.compile(
    r"""\$router\.(?:push|replace)\s*\(\s*["']([^"']+)["']|\brouter\.(?:push|replace)\s*\(\s*["']([^"']+)["']"""
)
_VUE_ROUTER_PUSH_NAME = re.compile(
    r"""\$router\.(?:push|replace)\s*\(\s*\{[^}]*\bname\s*:|\brouter\.(?:push|replace)\s*\(\s*\{[^}]*\bname\s*:"""
)

# React 模式
_REACT_LINK = re.compile(
    r"""<Link\s+[^>]*?\bto\s*=\s*["']([^"']+)["']"""
)
_REACT_LINK_DYNAMIC = re.compile(
    r"""<Link\s+[^>]*?\bto\s*=\s*\{[^}`]*`[^`]*\$\{"""
)
_REACT_NAVIGATE = re.compile(
    r"""\bnavigate\s*\(\s*["']([^"']+)["']"""
)
_REACT_NAVIGATE_DYNAMIC = re.compile(
    r"""\bnavigate\s*\(\s*["'`][^"``]*\$\{"""
)
_REACT_ROUTE = re.compile(
    r"""<Route[^>]*path\s*=\s*["']([^"']+)["']"""
)
_REACT_ROUTE_TPL = re.compile(
    r"""<Route[^>]*path\s*=\s*["']([^"']*:\w+[^"']*)["']"""
)


def _scan_file_for_references(
    file_path: Path,
    framework: str,
) -> tuple[list[tuple[str, int, str]], list[tuple[str, int, str]]]:
    """扫描单个文件中的路由引用。

    Returns:
        (concrete_refs, dynamic_refs)
        concrete_refs: [(路径, 行号, 原始文本), ...]
        dynamic_refs:  [(路径前缀, 行号, 原始文本), ...]  -- 动态拼接，需人工确认
    """
    content = file_path.read_text(encoding="utf-8", errors="replace")
    lines = content.splitlines()
    concrete: list[tuple[str, int, str]] = []
    dynamic: list[tuple[str, int, str]] = []

    for i, line in enumerate(lines, start=1):
        stripped = line.strip()
        if stripped.startswith("//") or stripped.startswith("#"):
            continue

        if framework == "vue":
            # 静态 router-link
            for m in _VUE_ROUTER_LINK.finditer(line):
                path = m.group(1)
                concrete.append((path, i, m.group(0)))
            # 动态 router-link（拼接）
            for m in _VUE_ROUTER_LINK_DYNAMIC.finditer(line):
                # 提取静态前缀
                prefix_m = re.search(r"""to\s*=\s*["'](/[^"']*)["']\s*\+\s*\w+""", line)
                if prefix_m:
                    dynamic.append((prefix_m.group(1), i, m.group(0)))
                else:
                    dynamic.append(("<dynamic>", i, m.group(0)))
            # router.push 静态
            for m in _VUE_ROUTER_PUSH.finditer(line):
                path = m.group(1) or m.group(2)
                if path:
                    concrete.append((path, i, m.group(0)))
            # router.push 按 name
            for m in _VUE_ROUTER_PUSH_NAME.finditer(line):
                dynamic.append(("<by-name>", i, m.group(0)))

        elif framework == "react":
            # Link 静态
            for m in _REACT_LINK.finditer(line):
                path = m.group(1)
                concrete.append((path, i, m.group(0)))
            # Link 动态
            for m in _REACT_LINK_DYNAMIC.finditer(line):
                prefix_m = re.search(r"""to\s*=\s*\{[^}`]*(/[^"`{]*)\$\{""", line)
                if prefix_m:
                    dynamic.append((prefix_m.group(1).rstrip("`{} "), i, m.group(0)))
                else:
                    dynamic.append(("<dynamic>", i, m.group(0)))
            # navigate 静态
            for m in _REACT_NAVIGATE.finditer(line):
                path = m.group(1)
                concrete.append((path, i, m.group(0)))
            # navigate 动态
            for m in _REACT_NAVIGATE_DYNAMIC.finditer(line):
                dynamic.append(("<dynamic>", i, m.group(0)))
            # Route path 含参数
            for m in _REACT_ROUTE_TPL.finditer(line):
                path = m.group(1)
                concrete.append((path, i, m.group(0)))

    return concrete, dynamic


# ---------------------------------------------------------------------------
# 匹配逻辑
# ---------------------------------------------------------------------------

def _normalize_path(path: str) -> str:
    """标准化路径用于比较。"""
    # 移除尾部斜杠
    path = path.rstrip("/")
    if not path:
        return "/"
    return path


def _path_matches(declared_set: set[str], ref_path: str) -> bool:
    """检查引用路径是否匹配已声明的路径集合。

    支持精确匹配和参数化匹配（如 /user/:id 匹配 /user/123）。
    """
    norm = _normalize_path(ref_path)

    # 根路径
    if norm == "/":
        return "/" in declared_set or "" in declared_set

    # 精确匹配
    if ref_path in declared_set or norm in declared_set:
        return True

    # 参数化匹配：逐段比较
    ref_parts = [p for p in norm.split("/") if p]
    for declared in declared_set:
        if declared == "/":
            continue
        declared_norm = _normalize_path(declared)
        dec_parts = [p for p in declared_norm.split("/") if p]
        if len(dec_parts) != len(ref_parts):
            continue
        if all(
            dp == rp or dp.startswith(":")
            for dp, rp in zip(dec_parts, ref_parts)
        ):
            return True

    return False


def run(config: dict) -> CheckResult:
    """执行路由死链检查。"""
    result = CheckResult()

    # 读取配置
    enabled = config.get("enabled", True)
    if not enabled:
        result.skipped = "已在配置中禁用"
        return result

    framework = config.get("framework", "")
    route_file_str = config.get("route_file", "")
    scan_dirs = config.get("scan_dirs", [])
    scan_ext = config.get("scan_ext", [".vue", ".ts", ".tsx", ".js", ".jsx"])
    ignore_patterns = config.get("ignore", [])

    if not framework:
        result.skipped = "未指定 framework（需要 vue 或 react）"
        return result

    if not route_file_str:
        result.skipped = "未指定 route_file"
        return result

    if not scan_dirs:
        result.skipped = "未指定 scan_dirs"
        return result

    route_file = Path(route_file_str).resolve()
    if not route_file.exists():
        result.skipped = f"路由注册文件不存在: {route_file}"
        return result

    # 提取声明的路径
    declared_paths = extract_declared_paths(route_file, framework)
    if not declared_paths:
        result.skipped = f"未能从 {route_file} 中提取到任何路由路径，请检查文件格式或 framework 配置"
        return result

    # 扫描源码引用
    for scan_dir_str in scan_dirs:
        scan_dir = Path(scan_dir_str).resolve()
        if not scan_dir.exists():
            continue

        for ext in scan_ext:
            for src_file in sorted(scan_dir.rglob(f"*{ext}")):
                # 检查忽略规则
                rel_str = str(src_file)
                skip = False
                for pat in ignore_patterns:
                    import fnmatch
                    if fnmatch.fnmatch(rel_str, pat) or fnmatch.fnmatch(src_file.name, pat):
                        skip = True
                        break
                if skip:
                    continue

                concrete_refs, dynamic_refs = _scan_file_for_references(src_file, framework)

                # 检查静态引用
                for path, line_no, raw in concrete_refs:
                    if not _path_matches(declared_paths, path):
                        rel_path = src_file.relative_to(Path.cwd()) if src_file.is_relative_to(Path.cwd()) else src_file
                        result.confirmed.append(
                            f"{rel_path}:{line_no} -> 路由路径 '{path}' 未在路由注册文件中声明"
                        )

                # 动态引用标记为 REVIEW_NEEDED
                for prefix, line_no, raw in dynamic_refs:
                    rel_path = src_file.relative_to(Path.cwd()) if src_file.is_relative_to(Path.cwd()) else src_file
                    result.review_needed.append(
                        f"{rel_path}:{line_no} -> 动态路由引用 '{prefix}'，无法静态判断是否有效（需人工确认）"
                    )

    result.passed = len(result.confirmed) == 0
    return result
