# CI 检查规范（CI Check Standard）

> 本文档定义 vet-knowledge 项目静态检查的三档执行模式、触发时机、配置标准与失败处理流程。
>
> **设计原则**：检查前置（Shift-Left）、分级执行、失败必究、不可绕过。
>
> **关联文档**：[修复 SOP](./fix_sop.md)、[静态分析规则](../rules/static_analysis_rules.md)、[工作流改革方案](../../docs/WORKFLOW_REFORM.md)

---

## 1. 三档检查模式

### 1.1 模式定义

| 模式 | 命令 | 耗时 | 触发时机 | 检查器 |
|------|------|------|----------|--------|
| **Quick** | `check.py --quick` | <5s | 每次 commit 前（pre-commit hook） | route_check + api_check |
| **Full** | `check.py --full` | 10-30s | 阶段交付前、合并到主分支前 | 全部 6 个检查器 |
| **Single** | `check.py <name>` | 视检查器 | 针对性验证特定规则 | 指定检查器 |

### 1.2 执行命令

```bash
# Quick 模式（提交前自动）
python _tools/dev/check.py --config check_config.json --quick --no-color

# Full 模式（阶段交付前手动）
python _tools/dev/check.py --config check_config.json --full

# 单检查器模式（针对性验证）
python _tools/dev/check.py --config check_config.json route_check
python _tools/dev/check.py --config check_config.json api_check
python _tools/dev/check.py --config check_config.json type_check
python _tools/dev/check.py --config check_config.json empty_check
python _tools/dev/check.py --config check_config.json hardcode_check
python _tools/dev/check.py --config check_config.json dead_code_check
```

---

## 2. 触发时机与责任

### 2.1 检查时机矩阵

| 时机 | 模式 | 自动/手动 | 责任人 | 失败处理 |
|------|------|-----------|--------|----------|
| **提交前** | Quick | 自动（pre-commit hook） | 开发者 | 必须修复才能提交 |
| **推送前** | Quick | 自动（pre-push hook，可选） | 开发者 | 必须修复才能推送 |
| **阶段交付前** | Full | 手动 | 开发者 | 必须全通过才能交付 |
| **合并到 main 前** | Full | 手动 | Reviewer | 必须全通过才能合并 |
| **数据变更后** | Full + `cargo test` | 手动 | 开发者 | 必须全通过才能发布 |
| **发布前** | Full + `cargo test` + `vue-tsc` + `cargo clippy` | 手动 | 发布者 | 必须全通过才能发布 |

### 2.2 三个检查时机（源自 WORKFLOW_REFORM.md）

#### 时机一：提交前自动检查
- **触发**：`git commit` 时由 pre-commit hook 自动运行
- **模式**：Quick（route_check + api_check）
- **耗时**：<5 秒
- **失败处理**：阻止提交，开发者必须修复后重新提交
- **安装**：`python _tools/dev/check.py --install-hook`

#### 时机二：阶段交付前手动检查
- **触发**：完成一个功能模块或修复一批问题后
- **模式**：Full（全部 6 个检查器）
- **耗时**：10-30 秒
- **失败处理**：CONFIRMED 必须修复，REVIEW_NEEDED 人工判断
- **记录**：在 TodoWrite 中标记评审任务

#### 时机三：数据变更后检查
- **触发**：修改 `data/*.md`、`data/*.yaml`、`schema.sql` 后
- **模式**：Full + `cargo test` + `cargo check`
- **额外步骤**：
  1. 运行 `python tools/gen_from_yaml.py` 重新生成 SQL
  2. 递增 `src-tauri/src/db/mod.rs` 中的 `SEED_DATA_VERSION`
  3. 运行 `cargo check` 验证编译
  4. 运行 `check.py --full` 验证一致性
  5. 运行 `cargo test` 验证数据正确性

---

## 3. 退出码与处理流程

### 3.1 退出码定义

| 退出码 | 常量 | 含义 | 处理策略 |
|--------|------|------|----------|
| 0 | `EXIT_OK` | 全部通过 | 可继续后续流程 |
| 1 | `EXIT_CONFIRMED` | 存在确定问题 | **必须修复**后重新运行 |
| 2 | `EXIT_REVIEW` | 存在待人工判断项 | 人工 review 后决定是否接受 |

### 3.2 失败处理流程

```
检查失败
   │
   ├─ 退出码 1 (CONFIRMED)
   │     │
   │     ├─ 阅读输出中的 CONFIRMED 条目
   │     ├─ 按 fix_sop.md 进入三阶段闭环修复
   │     ├─ 修复后重新运行 check.py --full
   │     └─ 通过后方可提交/交付
   │
   └─ 退出码 2 (REVIEW_NEEDED)
         │
         ├─ 阅读输出中的 REVIEW_NEEDED 条目
         ├─ 人工判断是否为误报
         │    ├─ 是误报 → 加入 .checkignore 或 check_config.json 的 ignore
         │    └─ 非误报 → 按 CONFIRMED 流程修复
         └─ 记录判断结论
```

### 3.3 误报处理规范

**误报判定标准**：
- 检查器报告的问题在当前上下文中不成立
- 检查器无法理解特定的业务逻辑或设计意图
- 检查器正则匹配过于宽泛导致误报

**误报处理方式**（按优先级）：

1. **修复检查器逻辑**（首选）：调整正则或判断条件，从根源消除误报
2. **加入 ignore 配置**：在 `check_config.json` 对应检查器的 `ignore` 数组中加入文件 glob
3. **加入 .checkignore**：项目根目录的 `.checkignore` 文件，glob 模式忽略特定文件

**禁止行为**：
- 禁止直接关闭整个检查器规避误报（违反 F-10）
- 禁止在不记录原因的情况下加入 ignore

---

## 4. 配置文件管理

### 4.1 配置文件层级

```
check_config.json              # 项目专用配置（启用哪些检查器、扫描目录、ignore）
_tools/dev/check_config_schema.json  # 配置文件的 JSON Schema（IDE 补全 + 校验）
.checkignore                   # 全局 glob 忽略（可选）
```

### 4.2 配置变更规范

修改 `check_config.json` 时必须：
1. 确认变更符合 `check_config_schema.json` 的约束
2. 在 commit message 中说明变更原因
3. 运行 `check.py --schema` 验证配置合法性
4. 运行 `check.py --full` 验证配置生效

### 4.3 当前项目配置要点

| 检查器 | 状态 | 关键配置 |
|--------|------|----------|
| route_check | ✅ 启用 | Vue 框架，扫描 `src/views`、`src/components` |
| api_check | ✅ 启用 | Tauri 类型，后端入口 `src-tauri/src/lib.rs` |
| type_check | ⚠️ 默认关闭 | 高误报风险，**应按 fix_sop 修复后开启** |
| empty_check | ✅ 启用 | 注册文件 `lib.rs`，实现目录 `commands` |
| hardcode_check | ✅ 启用 | 扫描 `tools/*.py`，threshold=5 |
| dead_code_check | ✅ 启用 | schema=`schema.sql`，排除 `schema_migrations`、`app_meta` |

---

## 5. Git Hook 安装与管理

### 5.1 安装 pre-commit hook

```bash
python _tools/dev/check.py --install-hook
```

**生成的文件**（跨平台）：
- `.git/hooks/pre-commit`（Linux/macOS shell 脚本）
- `.git/hooks/pre-commit.bat`（Windows 批处理）
- `.git/hooks/pre-commit.py`（Python 实现，被上述脚本调用）

**hook 行为**：
- 自动运行 `check.py --quick --no-color`
- 退出码非 0 时阻止 `git commit`
- 输出带颜色（除非 `--no-color`）

### 5.2 跳过 hook（仅紧急情况）

```bash
git commit --no-verify
```

**警告**：跳过 hook 违反规范，必须在 commit message 中说明跳过原因，并在 24 小时内补跑 `check.py --full`。

---

## 6. CI/CD 集成（规划中）

### 6.1 GitHub Actions 工作流（建议）

```yaml
# .github/workflows/check.yml
name: Code Quality Check
on: [push, pull_request]
jobs:
  check:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-python@v5
        with: { python-version: '3.x' }
      - uses: dtolnay/rust-toolchain@stable
      - uses: pnpm/action-setup@v3
      - run: pnpm install
      - run: python _tools/dev/check.py --config check_config.json --full
      - run: cd src-tauri && cargo test
      - run: pnpm vue-tsc --noEmit
```

### 6.2 检查门禁

| 检查项 | 阻塞合并 | 说明 |
|--------|----------|------|
| `check.py --full` | ✅ | 退出码必须为 0 |
| `cargo test` | ✅ | 所有测试通过 |
| `cargo clippy` | ⚠️ | 警告需处理，错误阻塞 |
| `vue-tsc --noEmit` | ✅ | 零类型错误 |
| `cargo check` | ✅ | 编译通过 |

---

## 7. 检查器扩展规范

### 7.1 新增检查器流程

1. 在 `_tools/dev/checkers/` 下新建 `<name>_check.py`
2. 文件头 docstring 必须包含：规则编号、检查目标、配置项、误报场景
3. 从 `checkers` 模块导入 `CheckResult`
4. 导出 `run(check_config: dict) -> CheckResult` 函数
5. 在 `_knowledge/rules/static_analysis_rules.md` 中添加对应规则定义
6. 在 `check_config_schema.json` 中添加对应 section schema
7. 在 `check_config.json` 中启用并配置
8. 运行 `check.py --list` 确认被发现
9. 运行 `check.py <name>` 验证功能

### 7.2 CheckResult 规范

```python
@dataclass
class CheckResult:
    passed: bool                              # 是否通过
    confirmed: list[dict]                     # 确定问题（必须修复）
    review_needed: list[dict]                 # 待人工判断
    skipped: list[str]                        # 跳过的检查项及原因
```

**问题条目格式**：
```python
{
    "rule": "LINK-DEAD",
    "severity": "CONFIRMED",
    "file": "src/views/knowledge/Home.vue",
    "line": 42,
    "message": "路由 /xxx 未在 router/index.ts 注册",
    "suggestion": "在 router/index.ts 添加路由定义"
}
```

---

## 8. 相关文档

- [修复 SOP](./fix_sop.md)
- [测试规范](./testing_standard.md)
- [错误处理规范](./error_handling_standard.md)
- [静态分析规则](../rules/static_analysis_rules.md)
- [工作流改革方案](../../docs/WORKFLOW_REFORM.md)
