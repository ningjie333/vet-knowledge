# 工作流改革方案

> 起草日期：2026-05-07
> 核心目标：消灭"每步都无法完全按照计划推进"的问题

---

## 一、问题诊断图谱

### 根因 1：无跨文件一致性检查

| # | 问题 | 发现阶段 | 可预防性 |
|---|------|---------|---------|
| 1 | 路由死链（按钮→白屏） | 用户点击 | ✅ 脚本检查 |
| 5 | 前后端类型字段不一致 | 运行时静默丢失 | ✅ 脚本检查 |
| 6 | 命令返回类型前端未声明 | 无报错 | ✅ 脚本检查 |
| 7 | 局部类型未复用全局定义 | 无报错 | ⚠️ ESLint 规则 |
| 11 | LIKE 搜索 vs FTS5 废弃表共存 | 无报错 | ✅ 脚本检查 |

### 根因 2：无构建前置检查

| # | 问题 | 发现阶段 | 可预防性 |
|---|------|---------|---------|
| 2 | dist 未重建，app 显示旧代码 | 用户反馈 | ✅ 脚本检查 |
| 3 | 旧进程锁定 exe | 编译报错 | ✅ 已解决（bat） |
| 13 | cargo features 不匹配 | 编译可能失败 | ✅ cargo check |

### 根因 3：数据管线混合管理

| # | 问题 | 发现阶段 | 可预防性 |
|---|------|---------|---------|
| 8 | case_disease_map 硬编码在 Python 字典 | 改数据时遗漏 | ⚠️ 需迁移到 YAML |
| 9 | import_export 空实现但已注册命令 | 随时可能暴露 | ✅ 脚本扫描 TODO |

### 根因 4：无运行时冒烟测试

| # | 问题 | 发现阶段 | 可预防性 |
|---|------|---------|---------|
| 4（元问题） | 阶段完成后遗留 bug | 交付后 | ⚠️ 需流程改进 |
| 10 | Set 响应式陷阱 | 间歇性 | ⚠️ 代码审查 |
| 12 | 文档版本号与代码不同步 | 认知负担 | ✅ 脚本检查 |

---

## 二、改革措施

### 措施 A：脚本化检查工具

#### A1. `tools/dev/check_routes.py` — 路由死链检查

**做什么：** 扫描所有 `.vue` 文件中的 `<router-link :to="...">` 和 `router.push('...')`，提取字面量路径，与 `router/index.ts` 中注册的路径对比，报告未注册的路由引用。

**输出示例：**
```
[CONFIRMED] CaseDetail.vue:179 → /cases/:id/study 未注册
[REVIEW NEEDED] SomePage.vue:42 → /detail/${id}（动态路径，需人工确认）
```

**置信度分层：**
- `[CONFIRMED]`：字面量路径直接匹配 → 大概率是真实死链
- `[REVIEW NEEDED]`：动态拼接路径 → 需人工判断

**已知局限（不处理的情况）：**
- `router.push({ name: 'xxx' })` 按 name 导航 → 不做 name→path 映射
- 第三方组件内部跳转 → 扫描不到
- 动态 `import()` 路由守卫添加的路由 → 扫描到但标注为 REVIEW NEEDED

**退出码：** 0=无问题，1=有 CONFIRMED 问题，2=仅有 REVIEW NEEDED

#### A2. `tools/dev/check_cmd_register.py` — 命令注册一致性检查

**做什么：** 扫描前端所有 `invoke('xxx')` 调用，与 `lib.rs` 中 `generate_handler![...]` 宏内注册的命令列表对比，报告未注册或已注册但未调用的命令。

**提取命令名的方式：**
- 前端：正则匹配 `invoke\s*<[^>]+>\s*\(\s*['"]([^'"]+)['"]`
- 后端：正则匹配 `generate_handler!\[([^\]]+)\]` 宏内容

**输出示例：**
```
[UNREGISTERED] DrugHandbook.vue:56 → invoke('get_drug_detail') 未在 lib.rs 注册
[UNUSED] lib.rs 注册了 export_data 但前端从未调用
```

#### A3. `tools/dev/check_data_pipeline.py` — 数据管线一致性检查

**做什么：**
1. 检查 `gen_from_yaml.py` 中是否有硬编码的数据字典（如 `case_disease_map`），报告应迁移到 YAML 的项
2. 检查 `schema.sql` 中定义的表是否被任何 `.rs` 文件引用（发现废弃表如 `diseases_fts`）
3. 检查所有注册的命令函数体内是否有 `// TODO` 空实现

#### A4. `tools/dev/gate_check.py` — 统一检查入口

**做什么：** 串联上述所有检查脚本，支持两档模式：

```bash
python tools/dev/gate_check.py --quick    # 路由 + 命令检查，<5s
python tools/dev/gate_check.py --full     # 全套检查，<30s
python tools/dev/gate_check.py --install-hook  # 安装 Git pre-commit hook
```

**退出码：** 任一子检查返回非零则整体返回非零。

**pre-commit hook 行为：**
- 只跑 `--quick`（速度优先）
- 检查失败阻止 commit，提示"修复后重试或使用 --no-verify 跳过"
- `GATE_SKIP=1` 环境变量可跳过（输出 `[SKIPPED]` 标记）

---

### 措施 B：SOP 文档

所有 SOP 存放在 `_knowledge/workflows/` 目录下（与现有知识库一致）。

#### B1. `_knowledge/workflows/new_page.md` — 新增页面 SOP

```markdown
# SOP: 新增前端页面

## 前置条件
- [ ] 确认路由路径和名称
- [ ] 确认是否需要侧边栏菜单项

## 步骤
1. 创建 `src/views/{category}/{PageName}.vue`（骨架）
2. 在 `src/router/index.ts` 添加路由注册
3. 如需要，在 `src/components/Sidebar.vue` 添加菜单项
4. 运行 `python tools/dev/gate_check.py --quick` 确认无死链
5. 如果是 learning/game 类别，确认面包屑导航正常

## 验证
- [ ] gate_check 无报错
- [ ] 手动访问页面确认渲染正常
- [ ] 页面内所有按钮/链接点击无白屏

## 错误处理
- gate_check 报路由死链 → 检查 router/index.ts 是否漏注册
- 页面白屏 → 检查组件文件路径是否与 import 路径一致
- 菜单项不显示 → 检查 Sidebar.vue 的菜单配置
```

#### B2. `_knowledge/workflows/data_change.md` — 数据变更 SOP

```markdown
# SOP: 数据变更（疾病/症状/药物/检查/病例）

## 前置条件
- [ ] 确认变更类型（新增实体 / 修改字段 / 新增关系）

## 步骤
1. 编辑对应的 `.md` 文件（实体）或 `.yaml` 文件（关系）
2. 运行 `python tools/gen_from_yaml.py` 生成新 SQL
3. 递增 `src-tauri/src/db/mod.rs` 中的 `SEED_DATA_VERSION`
4. 运行 `cd src-tauri && cargo check` 确认编译通过
5. 运行 `python tools/dev/gate_check.py --full` 确认数据管线一致

## 验证
- [ ] SQL 文件生成无报错
- [ ] cargo check 通过
- [ ] gate_check --full 无报错

## 错误处理
- gen_from_yaml.py 报错 → 检查 YAML 缩进和 frontmatter 格式
- cargo check 报错 → 检查 SEED_DATA_VERSION 是否正确递增
- gate_check 报废弃表引用 → 检查是否误用了 FTS 表
```

---

### 措施 C：数据管线整理

#### C1. 将 case_disease_map 从硬编码迁移到 YAML

**当前：** `gen_from_yaml.py` 内部 Python dict 硬编码了 15 个病例-疾病关联
**目标：** 提取到 `data/relations.yaml` 的 `case_diseases` section

**执行步骤（三步独立验证）：**
1. 在 `relations.yaml` 添加 `case_diseases` section，不碰 gen_from_yaml.py
2. 验证 YAML 可读：`python -c "import yaml; yaml.safe_load(open('data/relations.yaml'))"`
3. 修改 gen_from_yaml.py 读取 YAML，保留硬编码作为 fallback（YAML 解析失败时降级 + 警告）
4. 对比生成 SQL 输出与旧硬编码输出（diff 确认一致）
5. 确认无误后移除硬编码

---

## 三、检查点设计

### 时机 1：代码提交前（自动）

**触发方式：** Git pre-commit hook（`gate_check.py --install-hook` 一键安装）

检查项：
- 路由死链（check_routes.py）
- 命令未注册（check_cmd_register.py）

**耗时目标：** < 5 秒

### 时机 2：阶段交付前（手动）

**触发方式：** 开发者手动运行 `gate_check.py --full`

额外检查项：
- 数据管线一致性（check_data_pipeline.py）
- 废弃表引用
- 空实现命令

**耗时目标：** < 30 秒

### 时机 3：数据变更后（手动 + 脚本）

**触发方式：** 按 `data_change.md` SOP 执行

额外检查项：
- `gen_from_yaml.py` 生成成功
- `cargo check` 通过
- `SEED_DATA_VERSION` 已递增

---

## 四、落地路线图

### Phase A：立即落地（1-2 天）

**目标：** 消灭路由死链和命令不一致问题

| 任务 | 文件 | 工作量 |
|------|------|--------|
| 写 check_routes.py | `tools/dev/check_routes.py` | 2h |
| 写 check_cmd_register.py | `tools/dev/check_cmd_register.py` | 2h |
| 写 gate_check.py + hook | `tools/dev/gate_check.py` | 1h |
| 写 new_page SOP | `_knowledge/workflows/new_page.md` | 0.5h |
| 写 data_change SOP | `_knowledge/workflows/data_change.md` | 0.5h |
| 迁移 case_disease_map | `data/relations.yaml` + `gen_from_yaml.py` | 2h |
| 安装并测试 hook | 本地 git config | 0.5h |

**Phase A 验收标准：**
- gate_check 对当前代码库至少发现 1 个已知问题（证明工具有效）
- pre-commit hook 能成功拦截有问题的 commit
- case_disease_map 成功迁移到 YAML，SQL 输出与迁移前一致

### Phase B：基础设施完善（3-5 天）

| 任务 | 文件 | 工作量 |
|------|------|--------|
| 写 check_data_pipeline.py | `tools/dev/check_data_pipeline.py` | 3h |
| 清理 FTS5 废弃表引用 | `schema.sql` + `search.rs` | 4h |
| 对齐前后端类型定义 | `types/index.ts` + `models.rs` | 3h |
| 为 import_export.rs 返回 Err | `commands/import_export.rs` | 1h |
| 建立 metrics/ 目录和 dashboard | `metrics/` | 2h |

### Phase C：长期优化（1-2 周）

| 任务 | 说明 |
|------|------|
| 引入 ts-rs | 从 Rust 结构体自动生成 TypeScript 类型，消除手写不一致 |
| 冒烟测试脚本 | 启动 app → 访问每个路由 → 确认 200（Playwright 或 tauri-driver） |
| ESLint 规则 | 禁止 Vue 文件中定义 interface（强制导入全局类型） |

---

## 五、效果度量

### 核心指标

| 指标 | 定义 | 测量方式 | 目标值（1个月） | 目标值（3个月） |
|------|------|---------|---------------|---------------|
| **返工率** | fix commit / feat commit | `metrics/rework_rate.csv`（每周自动采集） | 比基线降 30% | 比基线降 50% |
| **问题前移比例** | gate 拦截 / (gate 拦截 + 运行时暴露) | `metrics/shift_left.csv`（每月采集） | > 30% | > 80% |
| **检查覆盖率** | gate 实际运行 / 总 commit | `metrics/gate_runs.log`（hook 自动记录） | > 90% | > 95% |
| **SOP 遵循率** | 有 SOP checklist 且必选项全完成的 commit | `metrics/check_sop.py`（每月采集） | > 60% | > 85% |
| **工具精确度** | 真阳性 / 总报告数 | commit message 标注 gate:true_positive/false_positive | > 75% | > 80% |

### 数据收集机制

```
vet-knowledge/
├── metrics/
│   ├── gate_runs.log        # hook 自动写入（每次 commit 尝试）
│   ├── gate_blocked.log     # gate_check 拦截时自动写入
│   ├── rework_rate.csv      # 每周自动生成
│   ├── shift_left.csv       # 每月自动生成
│   └── dashboard.py         # 一键汇总所有指标
```

**运行节奏：**
- 每次 commit：pre-commit hook 自动执行（<5s）
- 每周：`python metrics/dashboard.py` 生成指标快照（<1min）
- 每月：根据趋势调校工具规则（5-10min）

### Commit Message 约定（半自动化采集的基础）

```
# 返工修复（自动计入返工率）
fix: 修复疫苗列表分页逻辑

# 运行时暴露的跨文件问题（自动计入前移比例分母）
fix: 路由名称不一致
source:runtime

# gate_check 拦截后的修复（自动计入精确度）
fix: 补全命令注册
gate:true_positive

# SOP checklist（自动计入遵循率）
feat: 新增疫苗库存页面
[SOP:new_page]
- [x] 路由检查通过
- [x] 命令注册检查通过
- [x] gate_check 通过
```

---

## 六、风险与缓解

| 风险 | 概率 | 影响 | 缓解方案 |
|------|------|------|---------|
| **脚本本身过时**（路由写法变化导致误报） | 高 | 高 | 输出标注置信度层级；提供 `.gateignore` 配置；在 SOP 中明确"gate 报错时开发者有责任判断真假阳性" |
| **不强制 = 不存在**（开发者不跑检查） | 高 | 高 | pre-commit hook 自动触发；逃生舱仅限 `--no-verify` 且留痕 |
| **假阳性过高导致集体跳过** | 中 | 高 | Phase A 优先校准精确度 > 覆盖面；假阳性率超 25% 时停修规则不加新规则 |
| **数据迁移无回滚**（case_disease_map 提取） | 中 | 高 | 三步独立验证；gen_from_yaml.py 保留硬编码作为 fallback；迁移前后 SQL diff 对比 |
| **Python 3.14 兼容性** | 低 | 低 | 脚本零外部依赖（仅 stdlib）；头部版本检查 |
| **与现有工作流冲突** | 中 | 中 | 实施前逐一确认现有 7 个 tools/ 脚本功能；gate_check 调用而非重写 |

---

## 七、改革后的目录结构

```
vet-knowledge/
├── tools/
│   ├── gen_from_yaml.py          # 已有：数据管线
│   ├── _gen_populate.py          # 已有
│   ├── populate_drug_fields.py   # 已有
│   ├── _build_data.py            # 已有
│   ├── _write_data.py            # 已有
│   ├── _write_json.py            # 已有
│   ├── _make_script.py           # 已有
│   └── dev/                      # 新增：开发检查工具
│       ├── gate_check.py         # 统一入口 + hook 安装
│       ├── check_routes.py       # 路由死链检查
│       ├── check_cmd_register.py # 命令注册一致性
│       └── check_data_pipeline.py # 数据管线一致性
├── metrics/                      # 新增：度量数据
│   ├── gate_runs.log
│   ├── gate_blocked.log
│   └── dashboard.py
├── _knowledge/
│   ├── 01_pitfalls.md            # 已有：踩坑记录
│   ├── 02_workflows.md           # 已有：工作流索引
│   ├── 03_environment.md         # 已有：环境配置
│   ├── 04_snippets.md            # 已有：代码片段
│   └── workflows/                # 新增：操作 SOP
│       ├── new_page.md           # 新增页面 SOP
│       └── data_change.md        # 数据变更 SOP
├── src/
├── src-tauri/
└── docs/
    └── WORKFLOW_REFORM.md        # 本文档
```

---

## 八、与 WAT 框架的映射

| WAT 原则 | 在本方案中的体现 |
|----------|----------------|
| **AI 编排，脚本执行** | gate_check 系列脚本接管确定性检查；AI 专注于判断和创造 |
| **SOP 驱动** | `_knowledge/workflows/` 下的 SOP 文档，含步骤、验证、错误处理 |
| **自改进闭环** | 度量数据 → 发现工具规则问题 → 修脚本 → 更新 SOP |
| **工具优先** | 先建检查脚本，再让 AI 在开发流程中调用，而非 AI 直接肉眼检查 |
| **经验固化** | 踩坑 → 写进 check_xxx.py → 变成自动检查 → 永不复发 |
