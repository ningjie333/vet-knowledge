"""
规则 3: 类型一致性检查（默认关闭，高误报风险）

扫描前端类型定义文件（.d.ts / types.ts）和后端模型文件（models.py / structs.rs），
对比同一实体在两端的字段是否匹配。

配置项 (check_config.json 中的 "type_check" section):
    enabled           : bool,  默认 False（因为高误报风险）
    frontend_types_dir : str,  前端类型定义文件目录
    backend_models_file: str,  后端模型文件路径
    entity_map        : {str: str},  实体名映射 {前端实体: 后端实体}
    field_map         : {str: {str: str}},  字段名映射 {实体名: {前端字段: 后端字段}}
    ignore_fields     : [str],  忽略的后端内部字段列表，默认 ["created_at","updated_at","deleted_at"]
    scan_ext          : [str],  文件扩展名

误报说明:
    - 前端 UI 专用字段（如 `editing`, `selected`）会被标记为后端缺失
    - 后端数据库内部字段可配置忽略
    - 前端 interface 只读字段与后端 optional 字段语义差异
    建议：仅在配置中显式启用，并配置好 field_map 和 ignore_fields
"""

from __future__ import annotations

import re
from pathlib import Path

from checkers import CheckResult

# ---------------------------------------------------------------------------
# 前端类型解析
# ---------------------------------------------------------------------------

# TypeScript interface / type
_TS_INTERFACE = re.compile(
    r"""(?:interface|type)\s+(\w+)\s*(?:extends\s+\w+(?:\s*&\s*\w+)*)?\s*\{([^}]*)\}"""
)
# Python dataclass / sqlalchemy model
_PY_CLASS_FIELD = re.compile(
    r"""^\s+(\w+)\s*[:=]\s*"""
)

# Rust struct
_RUST_STRUCT_FIELD = re.compile(
    r"""^\s*(?:pub\s+)?(\w+)\s*:\s*"""
)


def _extract_ts_interface_fields(content: str, entity_name: str) -> set[str] | None:
    """从 TypeScript 内容中提取指定 interface/type 的字段名。"""
    for m in _TS_INTERFACE.finditer(content):
        if m.group(1) == entity_name:
            fields_block = m.group(2)
            fields = set()
            for line in fields_block.splitlines():
                line = line.strip()
                if line.startswith("//") or not line or line == "}":
                    continue
                field_m = re.match(r"""(\w+)\s*[?!]?\s*:""", line)
                if field_m:
                    fields.add(field_m.group(1))
            return fields
    return None


def _extract_py_model_fields(content: str, entity_name: str) -> set[str] | None:
    """从 Python 内容中提取指定 class 的字段名。"""
    pattern = rf"""class\s+{re.escape(entity_name)}\s*\("""
    class_m = re.search(pattern, content)
    if not class_m:
        return None

    fields = set()
    # 简单提取类体内的 field 定义
    remaining = content[class_m.end():]
    # 跳过继承括号内的内容
    class_body = remaining
    paren_depth = 0
    start_idx = 0
    for i, ch in enumerate(remaining):
        if ch == '(':
            paren_depth += 1
        elif ch == ')':
            paren_depth -= 1
            if paren_depth == 0:
                start_idx = i + 1
                break
    class_body = remaining[start_idx:]

    # 提取类体字段
    for m in _PY_CLASS_FIELD.finditer(class_body):
        fname = m.group(1)
        if not fname.startswith("_"):
            fields.add(fname)

    return fields if fields else None


def _extract_rust_struct_fields(content: str, entity_name: str) -> set[str] | None:
    """从 Rust 内容中提取指定 struct 的字段名。"""
    pattern = rf"""(?:pub\s+)?struct\s+{re.escape(entity_name)}\s*\{{"""
    struct_m = re.search(pattern, content)
    if not struct_m:
        return None

    fields = set()
    remaining = content[struct_m.end():]
    brace_depth = 1
    for line in remaining.splitlines():
        if "}" in line and brace_depth == 1:
            break
        for m in _RUST_STRUCT_FIELD.finditer(line):
            fields.add(m.group(1))

    return fields if fields else None


# ---------------------------------------------------------------------------
# 对比逻辑
# ---------------------------------------------------------------------------

def run(config: dict) -> CheckResult:
    """执行类型一致性检查。"""
    result = CheckResult()

    enabled = config.get("enabled", False)
    if not enabled:
        result.skipped = "已在配置中禁用（默认关闭，需显式启用）"
        return result

    frontend_dir_str = config.get("frontend_types_dir", "")
    backend_file_str = config.get("backend_models_file", "")
    entity_map = config.get("entity_map", {})
    field_map: dict[str, dict[str, str]] = config.get("field_map", {})
    ignore_fields = set(config.get("ignore_fields", ["created_at", "updated_at", "deleted_at"]))
    scan_ext = config.get("scan_ext", [".ts", ".d.ts"])

    if not frontend_dir_str:
        result.skipped = "未指定 frontend_types_dir"
        return result
    if not backend_file_str:
        result.skipped = "未指定 backend_models_file"
        return result
    if not entity_map:
        result.skipped = "未指定 entity_map（需要 {前端实体: 后端实体} 映射）"
        return result

    frontend_dir = Path(frontend_dir_str).resolve()
    backend_file = Path(backend_file_str).resolve()

    if not frontend_dir.exists():
        result.skipped = f"前端类型目录不存在: {frontend_dir}"
        return result
    if not backend_file.exists():
        result.skipped = f"后端模型文件不存在: {backend_file}"
        return result

    # 读取后端文件
    backend_content = backend_file.read_text(encoding="utf-8", errors="replace")
    backend_suffix = backend_file.suffix

    # 读取前端目录下所有类型文件
    frontend_contents: dict[str, str] = {}
    for ext in scan_ext:
        for f in sorted(frontend_dir.rglob(f"*{ext}")):
            key = f.stem
            if key not in frontend_contents:
                frontend_contents[key] = ""
            frontend_contents[key] += "\n" + f.read_text(encoding="utf-8", errors="replace")

    for frontend_entity, backend_entity in entity_map.items():
        fe_fields = None
        for fcontent in frontend_contents.values():
            fe_fields = _extract_ts_interface_fields(fcontent, frontend_entity)
            if fe_fields is not None:
                break

        if fe_fields is None:
            result.review_needed.append(
                f"类型检查 -> 前端实体 '{frontend_entity}' 未找到类型定义"
            )
            continue

        # 提取后端字段
        if backend_suffix == ".py":
            be_fields = _extract_py_model_fields(backend_content, backend_entity)
        elif backend_suffix == ".rs":
            be_fields = _extract_rust_struct_fields(backend_content, backend_entity)
        else:
            # 尝试所有
            be_fields = _extract_py_model_fields(backend_content, backend_entity)
            if be_fields is None:
                be_fields = _extract_rust_struct_fields(backend_content, backend_entity)

        if be_fields is None:
            result.review_needed.append(
                f"类型检查 -> 后端实体 '{backend_entity}' 未找到类型定义"
            )
            continue

        # 应用字段映射
        fm = field_map.get(frontend_entity, {})
        fe_fields_mapped = set()
        for f in fe_fields:
            fe_fields_mapped.add(fm.get(f, f))

        be_fields_filtered = be_fields - ignore_fields
        fe_fields_filtered = fe_fields_mapped - ignore_fields

        # 前端有但后端没有
        missing_in_backend = fe_fields_filtered - (be_fields_filtered | set(fm.values()))
        for field in sorted(missing_in_backend):
            result.review_needed.append(
                f"类型检查 -> 前端字段 '{frontend_entity}.{field}' 在后端模型 '{backend_entity}' 中未找到（可能是 UI 专用字段）"
            )

        # 后端有但前端没有（可选提示）
        missing_in_frontend = be_fields_filtered - fe_fields_mapped
        for field in sorted(missing_in_frontend):
            result.review_needed.append(
                f"类型检查 -> 后端字段 '{backend_entity}.{field}' 在前端类型 '{frontend_entity}' 中未找到（可能是内部字段）"
            )

    result.passed = len(result.confirmed) == 0
    return result
