"""
规则 5: 硬编码数据检查

扫描脚本文件 (.py/.js/.ts) 中的大型字典/列表字面量，
报告超过 N 个键值对的硬编码数据结构（建议从配置文件读取）。

配置项 (check_config.json 中的 "hardcode_check" section):
    enabled     : bool,  默认 True
    scan_dirs   : [str], 要扫描的脚本目录列表
    scan_ext    : [str], 文件扩展名，默认 [.py, .js, .ts]
    threshold   : int,  键值对阈值，超过此值则报告，默认 5
    min_entries : int,  最小条目数，小于此值不报告，默认 3
    ignore      : [str], 忽略路径 glob 列表
    allow_names : [str], 允许的变量名 glob 列表 (如 "STATUS_*", "HTTP_*")

误报说明:
    - HTTP 状态码映射等确实需要硬编码的常量可通过 allow_names 排除
    - 测试 fixtures 可能包含大量合法测试数据
    - 枚举/映射表可能需要硬编码
"""

from __future__ import annotations

import re
from pathlib import Path

from checkers import CheckResult

# ---------------------------------------------------------------------------
# 字面量提取
# ---------------------------------------------------------------------------

# Python dict/list 字面量（简化：计算顶层 , 号和嵌套括号对）
_PY_DICT_START = re.compile(
    r"""(\w+)\s*=\s*\{"""
)
_PY_MULTILINE_DICT = re.compile(
    r"""(\w+)\s*(?::\s*(?:dict|Dict|Mapping)\s*[\[\{][^\]\}]*[\]\}])?\s*=\s*\{"""
)

# JS/TS object/array 字面量
_JS_OBJ_START = re.compile(
    r"""(?:const|let|var)\s+(\w+)\s*=\s*\{"""
)
_JS_ARR_START = re.compile(
    r"""(?:const|let|var)\s+(\w+)\s*=\s*\["""
)


def _count_dict_entries(content: str, start_pos: int) -> int:
    """从 { 或 [ 开始计算顶层条目数。"""
    depth = 0
    entries = 0
    in_string = False
    string_char = None
    i = start_pos

    while i < len(content):
        ch = content[i]

        if in_string:
            if ch == "\\":
                i += 2
                continue
            if ch == string_char:
                in_string = False
            i += 1
            continue

        if ch in ('"', "'", "`"):
            in_string = True
            string_char = ch
            i += 1
            continue

        if ch in ("{", "[", "("):
            depth += 1
            if depth == 1:
                first = True
        elif ch in ("}", "]", ")"):
            depth -= 1
            if depth == 0:
                return max(entries, 1)
        elif ch == "," and depth == 1:
            entries += 1

        i += 1

    return max(entries, 1)


def _scan_python_hardcoded(scan_file: Path, threshold: int, min_entries: int) -> list[tuple[str, int, int]]:
    """扫描 Python 文件中的硬编码数据结构。

    Returns:
        [(变量名, 条目数, 起始列偏移行号估计), ...]
    """
    content = scan_file.read_text(encoding="utf-8", errors="replace")
    findings = []
    lines = content.splitlines()

    # 简单策略：找顶层 dict 变量赋值，计算条目数
    # 实现: 逐行找 `name = {`，然后从该位置找匹配的 `}` 内的顶层 , 号数量
    for i, line in enumerate(lines, start=1):
        stripped = line.strip()
        # 必须以变量赋值开始
        var_m = re.match(r"""(\w+)\s*=\s*\{""", stripped)
        if not var_m:
            continue
        var_name = var_m.group(1)
        # 找到字典开始位置
        brace_start = line.index("{", var_m.end(1) - var_m.start(0))
        # 计算行内闭合
        line_remain = line[brace_start:]
        depth = 0
        for ch in line_remain:
            if ch == "{":
                depth += 1
            elif ch == "}":
                depth -= 1

        if depth <= 0:
            # 单行 dict，直接计算行内条目数
            entries = _count_dict_entries(line, brace_start)
        else:
            # 多行 dict
            all_text = "\n".join(lines[i - 1:])
            abs_pos = sum(len(l) + 1 for l in lines[:i - 1]) + line.index("{", var_m.end(1) - var_m.start(0))
            entries = _count_dict_entries(content, abs_pos)

        if entries >= min_entries and entries > threshold:
            findings.append((var_name, entries, i))

    return findings


def _scan_js_hardcoded(scan_file: Path, threshold: int, min_entries: int) -> list[tuple[str, int, int]]:
    """扫描 JS/TS 文件中的硬编码数据结构。"""
    content = scan_file.read_text(encoding="utf-8", errors="replace")
    lines = content.splitlines()
    findings = []

    for i, line in enumerate(lines, start=1):
        stripped = line.strip()
        var_m = re.match(r"""(?:const|let|var)\s+(\w+)\s*=\s*[\{\[]""", stripped)
        if not var_m:
            continue

        var_name = var_m.group(1)
        bracket_char = "{" if "{" in stripped[var_m.end(1):] else "["
        bracket_pos = stripped.index(bracket_char, var_m.end(1) - var_m.start(0))

        # 尝试计算行内是否闭合
        depth = 0
        for ch in stripped[bracket_pos:]:
            if ch in ("{", "["):
                depth += 1
            elif ch in ("}", "]"):
                depth -= 1

        if depth <= 0:
            entries = _count_dict_entries(stripped, bracket_pos)
        else:
            all_text = "\n".join(lines[i - 1:])
            abs_pos = sum(len(l) + 1 for l in lines[:i - 1]) + bracket_pos
            entries = _count_dict_entries(content, abs_pos)

        if entries >= min_entries and entries > threshold:
            findings.append((var_name, entries, i))

    return findings


def run(config: dict) -> CheckResult:
    """执行硬编码数据检查。"""
    result = CheckResult()

    enabled = config.get("enabled", True)
    if not enabled:
        result.skipped = "已在配置中禁用"
        return result

    scan_dirs = config.get("scan_dirs", [])
    scan_ext = config.get("scan_ext", [".py", ".js", ".ts"])
    threshold = config.get("threshold", 5)
    min_entries = config.get("min_entries", 3)
    ignore_patterns = config.get("ignore", [])
    allow_names = config.get("allow_names", [])

    if not scan_dirs:
        result.skipped = "未指定 scan_dirs"
        return result

    import fnmatch

    for scan_dir_str in scan_dirs:
        scan_dir = Path(scan_dir_str).resolve()
        if not scan_dir.exists():
            continue

        for ext in scan_ext:
            for src_file in sorted(scan_dir.rglob(f"*{ext}")):
                # 计算相对于 cwd 的路径用于 ignore 匹配
                try:
                    rel_path = src_file.relative_to(Path.cwd())
                except ValueError:
                    rel_path = src_file
                rel_str = str(rel_path).replace("\\", "/")
                skip = False
                for pat in ignore_patterns:
                    pat_norm = pat.replace("\\", "/")
                    if fnmatch.fnmatch(rel_str, pat_norm) or fnmatch.fnmatch(src_file.name, pat_norm):
                        skip = True
                        break
                if skip:
                    continue

                if ext == ".py":
                    findings = _scan_python_hardcoded(src_file, threshold, min_entries)
                else:
                    findings = _scan_js_hardcoded(src_file, threshold, min_entries)

                for var_name, count, line_no in findings:
                    # 检查是否被允许
                    allowed = False
                    for allow_pat in allow_names:
                        if fnmatch.fnmatch(var_name, allow_pat):
                            allowed = True
                            break
                    if allowed:
                        continue

                    rel_path = src_file.relative_to(Path.cwd()) if src_file.is_relative_to(Path.cwd()) else src_file
                    result.confirmed.append(
                        f"{rel_path}:{line_no} -> 变量 '{var_name}' 包含 {count} 个条目的硬编码数据结构（阈值 {threshold}），建议迁移到配置文件"
                    )

    result.passed = len(result.confirmed) == 0
    return result
