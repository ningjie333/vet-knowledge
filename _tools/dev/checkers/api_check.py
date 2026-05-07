"""
规则 2: API 调用一致性检查

扫描前端代码中的 API 调用（invoke/fetch/axios/trpc），
与后端注册的路由/命令列表对比，找出未注册的调用。

支持 API 类型：
    - Tauri  : `invoke('command_name')`
    - REST   : `fetch('/api/xxx')`, `axios.get('/api/xxx')`
    - tRPC   : `trpc.xxx.useQuery()`

配置项 (check_config.json 中的 "api_check" section):
    enabled          : bool,  默认 True
    api_type         : str,   必填，"tauri" / "rest" / "trpc"
    backend_route_file : str, 必填，后端路由/命令注册文件
    scan_dirs        : [str], 前端源码目录列表
    scan_ext         : [str], 文件扩展名，默认 [.ts, .tsx, .js, .jsx, .vue]
    ignore           : [str], 忽略 glob 列表

误报说明:
    - 动态拼接 URL 的 REST 调用标记为 REVIEW_NEEDED
    - 从配置文件读取端点的调用标记为 REVIEW_NEEDED
"""

from __future__ import annotations

import re
from pathlib import Path

from checkers import CheckResult

# ---------------------------------------------------------------------------
# 后端端点提取
# ---------------------------------------------------------------------------

# Tauri: invoke handler 注册 / command 定义
_TAURI_COMMAND_DEF = re.compile(
    r"""invoke_handler\s*\(\s*[^)]*|#\[tauri::command\]|\bregister_handler\s*\("""
)
_TAURI_COMMAND_NAME = re.compile(
    r"""fn\s+(\w+)\s*\(|(\w+)\s*\(\s*\w+\s*:\s*tauri::\w+|commands!\s*\[\s*["'](\w+)["']"""
)
# 通用命令注册
_TAURI_COMMAND_REG = re.compile(
    r"""["'](\w+)["']\s*(?::|=>)|\.register\s*\(\s*["'](\w+)["']"""
)

# REST 路由注册（广义模式）
_REST_ROUTE_GET = re.compile(
    r"""\.(?:get|post|put|delete|patch|head|options)\s*\(\s*["']([^"']+)["']"""
)
_REST_ROUTE_ROUTE = re.compile(
    r"""\.(?:route)\s*\(\s*["']([^"']+)["']"""
)
# Express / Koa / Fastify
_REST_APP_ROUTE = re.compile(
    r"""(?:app|router)\.(?:get|post|put|delete|patch|use|all)\s*\(\s*["'`]([^"'`]+)["'`]"""
)
# Actix-web / Axum / Rocket
_RUST_ROUTE_ATTR = re.compile(
    r"""#\[(?:get|post|put|delete|patch|head|options|route)\s*\(\s*["']([^"']+)["']"""
)

# tRPC
_TRPC_PROCEDURE = re.compile(
    r"""\.query\s*\(\s*["'](\w+)["']|\.mutation\s*\(\s*["'](\w+)["']|\.procedure\."""
)
_TRPC_ROUTER = re.compile(
    r"""router\s*\{[^}]*\}"""
)


def _extract_tauri_commands(route_file: Path) -> set[str]:
    content = route_file.read_text(encoding="utf-8", errors="replace")
    commands = set()
    # 找 tauri::command 标注的函数名
    for m in re.finditer(r"""#\[tauri::command(?:\([^)]*\))?\]\s*(?:pub\s+)?fn\s+(\w+)""", content):
        commands.add(m.group(1))
    # invoke_handler 宏中的命令名
    handler_block = re.search(r"""invoke_handler\s*\(\s*[\s\S]*?\)""", content)
    if handler_block:
        for m in re.finditer(r"""tauri::generate_handler!\s*\[\s*([\s\S]*?)\]""", handler_block.group()):
            for name in re.findall(r"""(\w+)\s*,??""", m.group(1)):
                commands.add(name.strip())
    # commands![...]
    for m in re.finditer(r"""commands!\s*\[\s*([\s\S]*?)\]""", content):
        for name in re.findall(r"""["'](\w+)["']""", m.group(1)):
            commands.add(name)
    return commands


def _extract_rest_routes(route_file: Path) -> set[str]:
    content = route_file.read_text(encoding="utf-8", errors="replace")
    routes: set[str] = set()
    for pat in (_REST_ROUTE_GET, _REST_ROUTE_ROUTE, _REST_APP_ROUTE, _RUST_ROUTE_ATTR):
        for m in pat.finditer(content):
            for g in m.groups():
                if g:
                    routes.add(g)
    return routes


def _extract_trpc_procedures(route_file: Path) -> set[str]:
    content = route_file.read_text(encoding="utf-8", errors="replace")
    procedures: set[str] = set()
    # 从 router 定义中提取 key
    for m in re.finditer(r"""(\w+)\s*(?:\??:\s*|\.\s*(?:query|mutation))""", content):
        name = m.group(1)
        if name not in ("router", "publicProcedure", "protectedProcedure", "createTRPCRouter"):
            procedures.add(name)
    return procedures


def extract_backend_endpoints(route_file: Path, api_type: str) -> set[str]:
    """从后端注册文件中提取所有已注册的端点/命令。"""
    if api_type == "tauri":
        return _extract_tauri_commands(route_file)
    elif api_type == "rest":
        return _extract_rest_routes(route_file)
    elif api_type == "trpc":
        return _extract_trpc_procedures(route_file)
    else:
        endpoints = _extract_rest_routes(route_file)
        endpoints.update(_extract_tauri_commands(route_file))
        endpoints.update(_extract_trpc_procedures(route_file))
        return endpoints


# ---------------------------------------------------------------------------
# 前端调用扫描
# ---------------------------------------------------------------------------

# Tauri
_TAURI_INVOKE = re.compile(
    r"""invoke\s*\(\s*["']([^"']+)["']"""
)
_TAURI_INVOKE_DYNAMIC = re.compile(
    r"""invoke\s*\(\s*["'`][^"``]*\$\{"""
)

# REST
_REST_FETCH = re.compile(
    r"""(?:fetch|fetchWithTimeout)\s*\(\s*["'`]([^"'`]+)["'`]"""
)
_REST_AXIOS = re.compile(
    r"""(?:axios|request|http)\s*\.\s*(?:get|post|put|delete|patch|head|options)\s*\(\s*["'`]([^"'`]+)["'`]"""
)
_REST_DYNAMIC = re.compile(
    r"""(?:fetch|axios)\s*\([^)]*\+|(?:fetch|axios)\s*\(\s*["'`][^"``]*\$\{"""
)

# tRPC
_TRPC_CALL = re.compile(
    r"""trpc\.(\w+)\.(?:useQuery|useMutation|useInfiniteQuery)"""
)
_TRPC_DIRECT = re.compile(
    r"""trpc\.(\w+)\.query\s*\(\s*["'](\w+)["']|trpc\.(\w+)\.mutation\s*\(\s*["'](\w+)["']"""
)


def _scan_file_api_calls(
    file_path: Path,
    api_type: str,
) -> tuple[list[tuple[str, int, str]], list[tuple[str, int, str]]]:
    """扫描单个文件的 API 调用。

    Returns:
        (concrete_calls, dynamic_calls)
        concrete_calls: [(端点名, 行号, 原始文本), ...]
        dynamic_calls:  [(前缀, 行号, 原始文本), ...]
    """
    content = file_path.read_text(encoding="utf-8", errors="replace")
    lines = content.splitlines()
    concrete: list[tuple[str, int, str]] = []
    dynamic: list[tuple[str, int, str]] = []

    for i, line in enumerate(lines, start=1):
        stripped = line.strip()
        if stripped.startswith("//") or stripped.startswith("#"):
            continue

        if api_type == "tauri":
            for m in _TAURI_INVOKE.finditer(line):
                cmd = m.group(1)
                concrete.append((cmd, i, m.group(0)))
            for m in _TAURI_INVOKE_DYNAMIC.finditer(line):
                dynamic.append(("<dynamic>", i, m.group(0)))

        elif api_type == "rest":
            for pat in (_REST_FETCH, _REST_AXIOS):
                for m in pat.finditer(line):
                    url = m.group(1)
                    concrete.append((url, i, m.group(0)))
            for m in _REST_DYNAMIC.finditer(line):
                dynamic.append(("<dynamic>", i, m.group(0)))

        elif api_type == "trpc":
            for m in _TRPC_CALL.finditer(line):
                name = m.group(1)
                concrete.append((name, i, m.group(0)))
            for m in _TRPC_DIRECT.finditer(line):
                name = m.group(1) or m.group(3)
                if name:
                    concrete.append((name, i, m.group(0)))

    return concrete, dynamic


def run(config: dict) -> CheckResult:
    """执行 API 调用一致性检查。"""
    result = CheckResult()

    enabled = config.get("enabled", True)
    if not enabled:
        result.skipped = "已在配置中禁用"
        return result

    api_type = config.get("api_type", "")
    backend_file_str = config.get("backend_route_file", "")
    scan_dirs = config.get("scan_dirs", [])
    scan_ext = config.get("scan_ext", [".ts", ".tsx", ".js", ".jsx", ".vue"])
    ignore_patterns = config.get("ignore", [])

    if not api_type:
        result.skipped = "未指定 api_type（需要 tauri/rest/trpc）"
        return result
    if not backend_file_str:
        result.skipped = "未指定 backend_route_file"
        return result
    if not scan_dirs:
        result.skipped = "未指定 scan_dirs"
        return result

    backend_file = Path(backend_file_str).resolve()
    if not backend_file.exists():
        result.skipped = f"后端路由文件不存在: {backend_file}"
        return result

    # 如果提供了多个后端文件（逗号分隔）
    backend_endpoints: set[str] = set()
    for bf_str in backend_file_str.split(","):
        bf = Path(bf_str.strip()).resolve()
        if bf.exists():
            backend_endpoints.update(extract_backend_endpoints(bf, api_type))

    if not backend_endpoints:
        result.skipped = f"未能从后端文件中提取到任何端点，请检查文件格式或 api_type 配置"
        return result

    for scan_dir_str in scan_dirs:
        scan_dir = Path(scan_dir_str).resolve()
        if not scan_dir.exists():
            continue

        for ext in scan_ext:
            for src_file in sorted(scan_dir.rglob(f"*{ext}")):
                rel_str = str(src_file)
                skip = False
                for pat in ignore_patterns:
                    import fnmatch
                    if fnmatch.fnmatch(rel_str, pat) or fnmatch.fnmatch(src_file.name, pat):
                        skip = True
                        break
                if skip:
                    continue

                concrete_calls, dynamic_calls = _scan_file_api_calls(src_file, api_type)

                for endpoint, line_no, raw in concrete_calls:
                    if endpoint not in backend_endpoints:
                        rel_path = src_file.relative_to(Path.cwd()) if src_file.is_relative_to(Path.cwd()) else src_file
                        result.confirmed.append(
                            f"{rel_path}:{line_no} -> API 端点 '{endpoint}' 未在后端注册"
                        )

                for prefix, line_no, raw in dynamic_calls:
                    rel_path = src_file.relative_to(Path.cwd()) if src_file.is_relative_to(Path.cwd()) else src_file
                    result.review_needed.append(
                        f"{rel_path}:{line_no} -> 动态 API 调用 '{prefix}'，无法静态判断（需人工确认）"
                    )

    result.passed = len(result.confirmed) == 0
    return result
