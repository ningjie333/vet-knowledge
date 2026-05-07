"""
规则 4: 空实现检查

扫描所有注册的命令/路由处理函数，
检查函数体是否为空、只包含 TODO 注释、或返回硬编码空值。

配置项 (check_config.json 中的 "empty_check" section):
    enabled         : bool,  默认 True
    register_file   : str,   命令/路由注册文件路径
    impl_dirs       : [str], 实现代码目录列表
    impl_ext        : [str], 实现文件扩展名，默认 [.py, .rs, .ts, .js]
    allow_marker    : str,   标注允许空实现的注释标记，默认 "#![allow(unused)]" 或 "// TODO(allowed)"
    todo_patterns   : [str], 匹配 TODO 的正则列表

误报说明:
    - 有意留空的 placeholder 应在旁边添加 allow_marker 标注
    - 框架要求的空 trait 实现可能误报
"""

from __future__ import annotations

import re
from pathlib import Path

from checkers import CheckResult

# ---------------------------------------------------------------------------
# 空实现检测模式
# ---------------------------------------------------------------------------

# 空函数体模式（各种语言通用）
_EMPTY_BODY_PASS = re.compile(r"""^\s*(?:pass|return\s+None|return\s+Ok\(\(\)\)|return\s*\{\s*\}|return;\s*)$""")
_EMPTY_BODY_PANIC = re.compile(r"""^\s*(?:unimplemented!|todo!|panic!\s*\(\s*["'][^"']*["']\s*\))""")
_EMPTY_RETURN = re.compile(
    r"""return\s+(?:None|Ok\(\(\)\)|\{\s*\}|null|undefined|""|''|0|false)\s*;?\s*$"""
)
_TODO_ONLY = re.compile(r"""^\s*(?://|#|/\*|\*)\s*TODO""")
_COMMENT_ONLY = re.compile(r"""^\s*(?://|#|/\*|\*|<!--).*(?:placeholder|stub|noop|no.?op)""", re.IGNORECASE)


def _read_function_body(lines: list[str], start_idx: int, brace_char: str = "{", end_char: str = "}") -> list[str]:
    """从函数定义行开始，提取函数体行。"""
    body_lines = []
    brace_depth = 0
    started = False

    for i in range(start_idx, len(lines)):
        line = lines[i]
        stripped = line.strip()

        for ch in line:
            if ch == brace_char:
                brace_depth += 1
                started = True
            elif ch == end_char:
                brace_depth -= 1

        if started:
            if stripped and stripped != end_char:
                body_lines.append(stripped)
            if brace_depth <= 0:
                break

    return body_lines


def _is_python_function_empty(body_lines: list[str]) -> bool:
    """判断 Python 函数体是否为空实现。"""
    if not body_lines:
        return True
    code_lines = [
        l for l in body_lines
        if not l.startswith("#")
        and l not in ("pass", "return None", "...")
        and not l.startswith('"""')
        and not l.startswith("'''")
    ]
    if not code_lines:
        return True
    # 只有 TODO
    if all(_TODO_ONLY.match(l) or not l.strip() for l in code_lines):
        return True
    return False


def _is_rust_function_empty(body_lines: list[str]) -> bool:
    """判断 Rust 函数体是否为空实现。"""
    if not body_lines:
        return True
    for line in body_lines:
        if _EMPTY_BODY_PANIC.match(line):
            return True
        if _EMPTY_BODY_PASS.match(line):
            return True
        if _COMMENT_ONLY.match(line):
            return True
    return False


def _is_js_function_empty(body_lines: list[str]) -> bool:
    """判断 JS/TS 函数体是否为空实现。"""
    if not body_lines:
        return True
    code_lines = [l for l in body_lines if not l.startswith("//")]
    if not code_lines:
        return True
    for line in code_lines:
        if _TODO_ONLY.match(line):
            return True
        if _EMPTY_RETURN.match(line):
            return True
        if _COMMENT_ONLY.match(line):
            return True
    return False


# ---------------------------------------------------------------------------
# 函数位置提取
# ---------------------------------------------------------------------------

# 通用函数定义
_FUNC_DEF_PY = re.compile(r"""^def\s+(\w+)\s*\(""")
_FUNC_DEF_RS = re.compile(r"""(?:pub\s+)?(?:async\s+)?fn\s+(\w+)\s*\(""")
_FUNC_DEF_JS = re.compile(r"""(?:export\s+)?(?:async\s+)?function\s+(\w+)\s*\(|(\w+)\s*[:=]\s*(?:async\s+)?(?:function|\()""")


def _find_function_lines(file_path: Path, ext: str) -> list[tuple[str, int, list[str]]]:
    """查找文件中所有函数及其函数体。

    Returns:
        [(函数名, 行号, 函数体行列表), ...]
    """
    content = file_path.read_text(encoding="utf-8", errors="replace")
    lines = content.splitlines()
    results = []

    if ext == ".py":
        in_class = False
        indent_stack = [0]
        for i, line in enumerate(lines, start=1):
            func_m = _FUNC_DEF_PY.match(line)
            if func_m:
                fname = func_m.group(1)
                body_start = i  # 0-indexed
                # Python 函数体从下一行到下一个同缩进或更少缩进的行
                func_indent = len(line) - len(line.lstrip())
                body_lines = []
                for j in range(body_start, len(lines)):
                    bl = lines[j]
                    bl_stripped = bl.strip()
                    if not bl_stripped:
                        body_lines.append("")
                        continue
                    bl_indent = len(bl) - len(bl.lstrip())
                    if bl_indent <= func_indent and bl_stripped:
                        break
                    body_lines.append(bl_stripped)
                results.append((fname, i, body_lines))
    else:
        # Rust / JS / TS 使用 {} 提取
        pattern = _FUNC_DEF_RS if ext == ".rs" else _FUNC_DEF_JS
        for i, line in enumerate(lines, start=1):
            func_m = pattern.search(line)
            if func_m:
                fname = func_m.group(1) or (func_m.group(2) if func_m.lastindex and func_m.lastindex >= 2 else None)
                if fname:
                    # 提取函数体：从下一行到匹配的闭合括号/分号
                    body_lines = []
                    brace_count = 0
                    started = False
                    for j in range(i, len(lines)):
                        bl = lines[j]
                        brace_count += bl.count('{') - bl.count('}')
                        if '{' in bl:
                            started = True
                        if started:
                            body_lines.append(bl.strip())
                        if started and brace_count <= 0:
                            break
                    results.append((fname, i, body_lines))

    return results


def run(config: dict) -> CheckResult:
    """执行空实现检查。"""
    result = CheckResult()

    enabled = config.get("enabled", True)
    if not enabled:
        result.skipped = "已在配置中禁用"
        return result

    register_file_str = config.get("register_file", "")
    impl_dirs = config.get("impl_dirs", [])
    impl_ext = config.get("impl_ext", [".py", ".rs", ".ts", ".js"])
    allow_marker = config.get("allow_marker", "")
    todo_patterns_cfg = config.get("todo_patterns", [])

    if not register_file_str:
        result.skipped = "未指定 register_file"
        return result
    if not impl_dirs:
        result.skipped = "未指定 impl_dirs"
        return result

    register_file = Path(register_file_str).resolve()
    if not register_file.exists():
        result.skipped = f"注册文件不存在: {register_file}"
        return result

    # 从注册文件中提取所有注册命令/路由名
    reg_content = register_file.read_text(encoding="utf-8", errors="replace")
    # 提取所有被引用的函数/命令名
    registered_names: set[str] = set()

    # 通用：提取标识符
    for m in re.finditer(r"""["'](\w+)["']""", reg_content):
        registered_names.add(m.group(1))
    # Rust: handler_name 作为标识符
    for m in re.finditer(r"""(\w+)\s*,\s*$""", reg_content, re.MULTILINE):
        registered_names.add(m.group(1))
    # Python: 函数名
    for m in _FUNC_DEF_PY.finditer(reg_content):
        registered_names.add(m.group(1))

    # 合并自定义 TODO 模式
    all_todo_patterns = [_TODO_ONLY]
    for pat_str in todo_patterns_cfg:
        try:
            all_todo_patterns.append(re.compile(pat_str))
        except re.error:
            pass

    for impl_dir_str in impl_dirs:
        impl_dir = Path(impl_dir_str).resolve()
        if not impl_dir.exists():
            continue

        for ext in impl_ext:
            for src_file in sorted(impl_dir.rglob(f"*{ext}")):
                # 检查 allow_marker
                file_content = src_file.read_text(encoding="utf-8", errors="replace")
                if allow_marker and allow_marker in file_content:
                    # 如果文件包含 allow_marker，只检查函数附近的标注
                    pass

                functions = _find_function_lines(src_file, ext)

                for fname, line_no, body_lines in functions:
                    # 只检查注册过的函数
                    if fname not in registered_names:
                        continue

                    # 检查函数附近是否有 allow_marker
                    lines = file_content.splitlines()
                    context_start = max(0, line_no - 3)
                    context_end = min(len(lines), line_no + 5)
                    context = "\n".join(lines[context_start:context_end])
                    if allow_marker and allow_marker in context:
                        continue

                    # 检查是否为空实现
                    is_empty = False
                    if ext == ".py":
                        is_empty = _is_python_function_empty(body_lines)
                    elif ext == ".rs":
                        is_empty = _is_rust_function_empty(body_lines)
                    else:
                        is_empty = _is_js_function_empty(body_lines)

                    if is_empty:
                        rel_path = src_file.relative_to(Path.cwd()) if src_file.is_relative_to(Path.cwd()) else src_file
                        # 判断是否看起来像 intentional placeholder
                        body_joined = "\n".join(body_lines)
                        if any(p.search(body_joined) for p in all_todo_patterns):
                            result.confirmed.append(
                                f"{rel_path}:{line_no} -> 注册函数 '{fname}' 仅有 TODO 标注，尚未实现"
                            )
                        else:
                            result.confirmed.append(
                                f"{rel_path}:{line_no} -> 注册函数 '{fname}' 为空实现"
                            )

    result.passed = len(result.confirmed) == 0
    return result
