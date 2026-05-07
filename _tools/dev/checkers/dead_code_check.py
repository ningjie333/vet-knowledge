"""
规则 6: 废弃代码检查

扫描数据库 schema 文件（.sql）中定义的表，
检查这些表是否被任何应用代码（.rs/.py/.js/.ts）引用。
未被任何代码引用的表将被标记为可疑废弃。

配置项 (check_config.json 中的 "dead_code_check" section):
    enabled       : bool,  默认 True
    schema_file   : str,   SQL schema 文件路径
    code_dirs     : [str], 应用代码目录列表
    code_ext      : [str], 代码文件扩展名，默认 [.rs, .py, .js, .ts]
    ignore        : [str], 忽略路径 glob 列表
    exclude_tables: [str], 排除检查的表名列表 (如 migration 表)

误报说明:
    - 表被动态 SQL 引用（字符串拼接）无法静态检测
    - ORM 自动映射（如 Diesel 的 table! 宏内的表名）可能未直接出现在代码中
    - 通过配置文件/环境变量引用的表名
"""

from __future__ import annotations

import re
from pathlib import Path

from checkers import CheckResult

# ---------------------------------------------------------------------------
# SQL Schema 解析
# ---------------------------------------------------------------------------

_SQL_CREATE_TABLE = re.compile(
    r"""CREATE\s+TABLE\s+(?:IF\s+NOT\s+EXISTS\s+)?[`"']?(\w+)[`"']?""",
    re.IGNORECASE,
)
_SQL_ALTER_TABLE = re.compile(
    r"""ALTER\s+TABLE\s+[`"']?(\w+)[`"']?""",
    re.IGNORECASE,
)


def extract_sql_tables(schema_file: Path) -> set[str]:
    """从 SQL schema 文件中提取所有表名。"""
    content = schema_file.read_text(encoding="utf-8", errors="replace")
    tables = set()
    for m in _SQL_CREATE_TABLE.finditer(content):
        tables.add(m.group(1))
    for m in _SQL_ALTER_TABLE.finditer(content):
        tables.add(m.group(1))
    return tables


# ---------------------------------------------------------------------------
# 应用代码引用扫描
# ---------------------------------------------------------------------------

# 通用：表名作为字符串字面量
_STRING_LITERAL = re.compile(
    r"""["'`]([a-z_]{3,})["'`]"""
)
# Rust: table! 宏、sql_query、execute
_RUST_TABLE_REF = re.compile(
    r"""table!\s*\(\s*(\w+)|sql_query[^)]*"[^"]*FROM\s+(\w+)|"[^"]*FROM\s+(\w+)"""
)
# Python: session.execute, query(...), text()
_PY_SQL_REF = re.compile(
    r"""execute\s*\([^)]*["']([^"']*)["']|query\s*\(\s*(\w+)|text\s*\(\s*["']([^"']*)["']"""
)
# JS/TS: knex, sequelize, prisma 等
_JS_TABLE_REF = re.compile(
    r"""from\s*\(\s*["'`](\w+)["'`]|table\s*\(\s*["'`](\w+)["'`]|knew?\s*\(\s*["'`](\w+)["'`]|where\s*\(\s*["'`](\w+)["'`]"""
)


def _scan_file_for_table_refs(file_path: Path, ext: str) -> dict[str, list[int]]:
    """扫描文件中的表名引用。

    Returns:
        { 表名: [行号, ...] }
    """
    content = file_path.read_text(encoding="utf-8", errors="replace")
    lines = content.splitlines()
    refs: dict[str, list[int]] = {}

    for i, line in enumerate(lines, start=1):
        stripped = line.strip()
        if stripped.startswith("//") or stripped.startswith("#") or stripped.startswith("--"):
            continue

        found_names = set()

        if ext == ".rs":
            for m in _RUST_TABLE_REF.finditer(line):
                for g in m.groups():
                    if g:
                        found_names.add(g)
        elif ext == ".py":
            for m in _PY_SQL_REF.finditer(line):
                for g in m.groups():
                    if g:
                        # SQL 语句可能引用表名
                        found_names.update(
                            w for w in g.split()
                            if len(w) >= 3 and w.replace("_", "").isalpha()
                        )
                        found_names.add(g)
        else:
            for m in _JS_TABLE_REF.finditer(line):
                for g in m.groups():
                    if g:
                        found_names.add(g)

        for name in found_names:
            refs.setdefault(name, []).append(i)

    return refs


def run(config: dict) -> CheckResult:
    """执行废弃代码检查。"""
    result = CheckResult()

    enabled = config.get("enabled", True)
    if not enabled:
        result.skipped = "已在配置中禁用"
        return result

    schema_file_str = config.get("schema_file", "")
    code_dirs = config.get("code_dirs", [])
    code_ext = config.get("code_ext", [".rs", ".py", ".js", ".ts"])
    ignore_patterns = config.get("ignore", [])
    exclude_tables = set(config.get("exclude_tables", [
        "__diesel_schema_migrations", "seaql_migrations", "schema_migrations"
    ]))

    if not schema_file_str:
        result.skipped = "未指定 schema_file"
        return result
    if not code_dirs:
        result.skipped = "未指定 code_dirs"
        return result

    schema_file = Path(schema_file_str).resolve()
    if not schema_file.exists():
        result.skipped = f"schema 文件不存在: {schema_file}"
        return result

    # 提取所有 SQL 表名
    all_tables = extract_sql_tables(schema_file) - exclude_tables
    if not all_tables:
        result.skipped = f"未能从 {schema_file} 中提取到任何表名"
        return result

    # 扫描所有应用代码中的表引用
    all_refs: dict[str, list[tuple[Path, list[int]]]] = {}
    import fnmatch

    for code_dir_str in code_dirs:
        code_dir = Path(code_dir_str).resolve()
        if not code_dir.exists():
            continue

        for ext in code_ext:
            for src_file in sorted(code_dir.rglob(f"*{ext}")):
                rel_str = str(src_file)
                skip = False
                for pat in ignore_patterns:
                    if fnmatch.fnmatch(rel_str, pat) or fnmatch.fnmatch(src_file.name, pat):
                        skip = True
                        break
                if skip:
                    continue

                file_refs = _scan_file_for_table_refs(src_file, ext)
                for table_name, line_nos in file_refs.items():
                    all_refs.setdefault(table_name, []).append((src_file, line_nos))

    # 找出未引用的表
    referenced_tables = set(all_refs.keys())

    # 检测引用：如果表名在代码中作为独立标识符出现
    # 额外做一个简单的全文匹配来捕获更多引用（如模块名、match 语句中等）
    for table_name in list(all_tables):
        for code_dir_str in code_dirs:
            code_dir = Path(code_dir_str).resolve()
            if not code_dir.exists():
                continue
            # 如果表名在之前扫描中未出现，做一个简单 grep 兜底
            if table_name not in referenced_tables:
                for ext in code_ext:
                    for src_file in sorted(code_dir.rglob(f"*{ext}")):
                        content = src_file.read_text(encoding="utf-8", errors="replace")
                        # 作为独立单词出现（避免子串匹配）
                        if re.search(rf"""(?<![a-zA-Z0-9_]){re.escape(table_name)}(?![a-zA-Z0-9_])""", content):
                            # 排除在注释中的引用
                            for i, line in enumerate(content.splitlines(), start=1):
                                if re.search(rf"""(?<![a-zA-Z0-9_]){re.escape(table_name)}(?![a-zA-Z0-9_])""", line):
                                    stripped = line.strip()
                                    if not stripped.startswith("//") and not stripped.startswith("#") and not stripped.startswith("--"):
                                        all_refs.setdefault(table_name, []).append((src_file, [i]))
                                        break
                            break
                    if table_name in all_refs:
                        break
            if table_name in all_refs:
                break

    # 最终未引用表
    unreferenced = all_tables - set(all_refs.keys())

    for table_name in sorted(unreferenced):
        result.review_needed.append(
            f"schema:{schema_file.name} -> 表 '{table_name}' 未在任何应用代码中被引用，可能是废弃表（注意：动态 SQL 引用无法静态检测）"
        )

    result.passed = len(result.confirmed) == 0
    return result
