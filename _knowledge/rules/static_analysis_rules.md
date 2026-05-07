# 静态分析规则定义（跨项目通用）

> 基于 vet-knowledge 项目实际代码问题抽象而成。每条规则独立可用，适配多种技术栈。

---

## 规则 1: 路由死链检测（LINK-DEAD）

**检查目标**: 扫描前端模板中所有路由链接引用，验证其在路由配置文件中是否被注册。

**适用项目类型**: 任意使用前端路由的项目（Vue Router / React Router / Angular Router / SvelteKit）

**输入文件**:
- 路由定义文件（如 `src/router/index.ts`）
- 所有包含路由引用的模板文件（`*.vue`、`*.tsx`、`*.jsx`、`*.svelte`）

**检查逻辑**:
```
1. 从路由配置提取已注册路径模式
   - 静态路径 "/diseases" → 精确匹配
   - 动态路径 "/cases/:id/study" → 转为正则 ^/cases/[^/]+/study$

2. 从模板文件提取所有 :to / to / href 值
   - 静态: "/cases"
   - 动态: "/cases/${id}/study"

3. 对每条引用:
   a. 静态路径精确匹配失败 → CONFIRMED 死链
   b. 动态路径归一化为参数模式后匹配失败 → CONFIRMED 死链
      "/cases/${id}/study" → "/cases/:id/study"
   c. 使用了非 ID 字段作为路由参数 → REVIEW_NEEDED
```

**输出格式**:
```
[CONFIRMED] src/views/foo.vue:42
  引用: /diseases/${x.name_zh}
  无匹配路由模式
  建议: 改为 /diseases/${x.id}

[REVIEW_NEEDED] src/views/bar.vue:108
  引用: /items/${item.slug}
  匹配: /items/:id
  注意: slug 参数与 id 语义不同
```

**退出码**: 0=干净 | 1=有CONFIRMED死链 | 2=仅有REVIEW_NEEDED

**误报场景**: 运行时动态注册路由 / 编程式导航 `router.push` / 通配符兜底路由 / 微前端跨应用链接

**配置参数**:

| 参数 | 说明 | 示例 |
|------|------|------|
| `route_files` | 路由定义文件路径列表 | `["src/router/index.ts"]` |
| `template_extensions` | 扫描的文件后缀 | `[".vue", ".tsx", ".jsx"]` |
| `id_field_names` | 合法参数变量名白名单 | `["id","ID","uuid","slug"]` |
| `interpolation_syntax` | 模板变量语法 | `javascript` / `rust` / `erb` |
| `ignore_patterns` | 忽略的路径模式 | `["*", "/:pathMatch(.*)*"]` |

---

## 规则 2: 前后端命令一致性检测（CMD-MISMATCH）

**检查目标**: 验证前端代码调用的后端命令在注册表中是否确实存在。

**适用项目类型**: 任意前后端分离项目（Tauri / Electron / 前后端分离 Web）

**输入文件**:
- 后端命令注册入口（Rust `generate_handler![]`、Spring `@RequestMapping`、Go `http.HandleFunc`）
- 所有前端调用代码（`*.ts`、`*.vue`、`*.tsx`）

**检查逻辑**:
```
1. 从后端注册入口提取所有已注册命令名
   Tauri: generate_handler![cmd1, cmd2, ...]
   REST: @GetMapping("/path") / router.get("/path", ...)
   GraphQL: resolvers 定义

2. 从前端代码提取所有调用命令名
   Tauri: invoke('command_name', {...})
   REST: fetch('/api/xxx')
   tRPC: client.xxx.query()

3. 应用命名风格转换（如 snake_case ↔ camelCase）

4. 前端有调用但后端未注册 → CONFIRMED 不匹配
5. 后端已注册但前端从未调用 → 废弃命令警告
```

**输出格式**:
```
[CONFIRMED] src/views/Foo.ts:38
  调用: invoke('get_case_detail')
  后端 generate_handler! 中未找到
  最近似: get_case_by_id

[INFO] 后端注册但未使用的命令: export_data (lib.rs:47)
```

**退出码**: 0=全部匹配 | 1=前端调用了未注册命令 | 2=存在未使用命令

**误报场景**: 命令名动态拼接 / 反射注册 / 非标准命名转换 / 仅测试代码调用 / 插件架构动态注册

**配置参数**:

| 参数 | 说明 | 示例 |
|------|------|------|
| `backend_entry` | 命令注册入口文件 | `"src-tauri/src/lib.rs"` |
| `registration_pattern` | 注册语法类型 | `tauri_generate_handler` / `express_route` / `spring_mapping` / `go_handler` |
| `invoke_pattern` | 前端调用语法 | `tauri_invoke` / `fetch_api` / `trpc_query` |
| `naming_conversion` | 命名风格转换 | `snake_to_camel` / `none` |
| `allowed_unused` | 允许未使用的命令白名单 | `["health_check","debug_*"]` |
| `frontend_extensions` | 前端扫描后缀 | `[".ts",".vue",".tsx",".js"]` |

---

## 规则 3: 跨层类型结构同步检测（TYPE-DRIFT）

**检查目标**: 检查前端 TypeScript 类型与后端结构体中同名实体的字段一致性。

**适用项目类型**: 前后端分离且各自定义类型的项目（TS+Rust / TS+Go / TS+Java）

**输入文件**:
- 前端类型定义文件（如 `src/types/index.ts`）
- 后端模型文件（如 `models.rs`、`*.entity.go`、`entity/*.java`）

**检查逻辑**:
```
1. 按类型名匹配前后端实体（忽略语言后缀差异）

2. 字段名对比（应用命名风格转换: snake_case → camelCase）

3. 前端有但后端无 → 运行时返回 undefined → HIGH
4. 后端有但前端无 → 字段被静默忽略 → LOW
5. 同名字段类型不兼容 → HIGH

6. 启发式: 语义相似但名称不同的字段对（需配置别名映射）
   procedure ↔ procedure_text → 人工确认是否需要映射
```

**实际发现（vet-knowledge 实例）**:
```
前端 Treatment.procedure     ↔ 后端 Treatment.procedure_text
前端 Treatment.prognosis_eval ↔ 后端 Treatment.prognosis_assessment
前端 Disease.tags: string[]   后端 Disease 无 tags 字段（通过 entity_tags 关联表实现）
后端 Disease.created_at      前端未定义（合理的内部字段）
后端 Case.age: Option<f64>   前端 age: number | null（浮点精度语义差异）
```

**输出格式**:
```
[CONFIRMED] Treatment 字段名不匹配:
  前端 procedure ↔ 后端 procedure_text
  前端 prognosis_eval ↔ 后端 prognosis_assessment
  位置: src/types/index.ts:64 ↔ src-tauri/src/db/models.rs:83

[HIGH] 前端字段在后端无对应:
  Disease.tags (前端) → 后端 Disease 结构体无 tags 字段

[LOW] 后端字段前端未定义:
  Disease.created_at, updated_at → 可视为内部字段忽略
```

**退出码**: 0=完全一致 | 1=HIGH级问题(字段缺失/类型不兼容) | 2=仅LOW级(命名差异)

**误报场景**: 仅内部使用的字段（created_at）/ 组合类型继承 / 中间DTO映射层 / 枚举表示不同

**配置参数**:

| 参数 | 说明 | 示例 |
|------|------|------|
| `frontend_type_files` | 前端类型文件路径 | `["src/types/index.ts"]` |
| `backend_model_files` | 后端模型文件路径 | `["src-tauri/src/db/models.rs"]` |
| `naming_conversion` | 命名转换方向 | `snake_to_camel` |
| `ignored_backend_fields` | 后端忽略字段白名单 | `["created_at","updated_at"]` |
| `known_aliases` | 语义相同但名称不同的字段映射 | `[{"frontend":"procedure","backend":"procedure_text"}]` |
| `type_compatibility` | 跨语言类型兼容表 | `{"string":"String","number":"i64|f64"}` |

---

## 规则 4: 已注册命令的空实现检测（CMD-STUB）

**检查目标**: 检测注册表中有名但函数体为空/TODO/返回硬编码空数据的命令处理函数。

**适用项目类型**: 任意项目（Tauri command / gRPC handler / HTTP handler / CLI command）

**输入文件**:
- 命令注册入口
- 对应的命令实现源文件

**检查逻辑**:
```
1. 提取所有已注册命令名
2. 定位每个命令的实现函数（通过命名约定映射）
3. 检测空实现特征:
   a. 函数体仅含 // TODO / # TODO / todo!() / unimplemented!() → CONFIRMED
   b. 返回硬编码空集合（vec![], Vec::new()）→ HIGH
   c. 返回空结构体字面量，无数据库查询 → HIGH
   d. 函数行数 < 阈值且无实质逻辑 → REVIEW_NEEDED
4. 区分 "返回空为合法默认" vs "未完成实现"
```

**实际发现（vet-knowledge 实例）**:
```
export_data (import_export.rs:14):
  函数体构造了 ExportData { diseases: vec![], ... } 全部为空数组
  仅写入文件但导出的是空数据
  标记: HIGH - 命令已注册且调用会成功，但导出的永远是空数据

import_data (import_export.rs:33):
  读取文件并解析 JSON 后仅有 // TODO: 验证并导入数据
  标记: CONFIRMED - 明确的未完成实现
```

**输出格式**:
```
[CONFIRMED] import_export.rs:33 - import_data
  状态: 含 TODO 标记
  风险: 前端有导入按钮，点击后文件读取成功但数据不被导入

[HIGH] import_export.rs:14 - export_data
  状态: 返回全空 ExportData，未查询数据库
  风险: 导出功能看似可用但永远导出空数据
```

**退出码**: 0=全部有完整实现 | 1=存在TODO标记命令 | 2=存在返回空数据命令

**误报场景**: 合法返回空默认值 / 委托模式薄包装 / TODO注释未删除但已实现 / 预留扩展点

**配置参数**:

| 参数 | 说明 | 示例 |
|------|------|------|
| `registration_entry` | 命令注册入口文件 | `"src-tauri/src/lib.rs"` |
| `impl_search_dirs` | 实现文件搜索目录 | `["src-tauri/src/commands/"]` |
| `stub_line_threshold` | 函数体行数阈值 | `8` |
| `todo_patterns` | TODO标记匹配模式 | `["// TODO","# TODO","todo!()","unimplemented!()"]` |
| `empty_return_patterns` | 空返回特征 | `["vec![]","Vec::new()","{..Default::default()}"]` |
| `known_stubs` | 已知stub白名单 | `["debug_dump","admin_*"]` |

---

## 规则 5: 数据管线硬编码数据检测（DATA-HARDCODE）

**检查目标**: 检测数据生成/管线脚本中应从外部数据源读取但被硬编码为 Python 字面量的数据。

**适用项目类型**: 任意包含数据管线/ETL/种子数据生成的项目

**输入文件**:
- 数据管线脚本（`tools/*.py`、`scripts/*.py`）
- 声明为数据源的文件（YAML/JSON/CSV/XML）

**检查逻辑**:
```
1. 扫描脚本中的 dict/list 字面量
2. 过滤小字面量（单元素配置、常量定义等）
3. 计算字面量键与数据源键的重合度
   - 重合度 > 阈值 → HIGH（数据应迁移到数据源）
4. 判断是否为关联表数据（多对多映射）
   - 是且数据源中有对应 section → 应迁移
5. 检查脚本是否同时读取数据源和硬编码同类数据 → 冗余风险
```

**实际发现（vet-knowledge 实例）**:
```
gen_from_yaml.py:522-538 - case_disease_map:
  硬编码字典将 15 个 case 映射到疾病 ID
  脚本已读取 relations.yaml 和 treatment_rules.yaml
  建议: 将 case_disease 关联迁移到 relations.yaml 的 case_diseases 部分
```

**输出格式**:
```
[HIGH] tools/gen_gen_from_yaml.py:522
  硬编码: case_disease_map = {'case_001': ['dis_009'], ...}
  15 个关联关系硬编码在 Python 中
  数据源 relations.yaml 已有类似结构
  建议: 迁移到数据源文件，使生成器统一从数据源读取
```

**退出码**: 0=无硬编码 | 1=存在与数据源重叠的硬编码 | 2=存在硬编码但不重叠

**误报场景**: 字段白名单/列名映射 / 默认值常量 / 小型查找表 / ORM DSL / 测试 fixture

**配置参数**:

| 参数 | 说明 | 示例 |
|------|------|------|
| `pipeline_scripts` | 管线脚本文件路径 | `["tools/gen_from_yaml.py"]` |
| `data_source_files` | 数据源文件路径 | `["data/relations.yaml"]` |
| `min_complexity` | 最小复杂度阈值(键值对数) | `5` |
| `overlap_threshold` | 键重合比例阈值 | `0.5` |
| `exclude_patterns` | 排除的合法模式 | `["*_cols","*_fields","DEFAULT_*","*_MAP"]` |

---

## 规则 6: 声明但未使用的数据库对象检测（DB-DEAD）

**检查目标**: 检测 Schema 定义了但应用代码中从未被引用的数据库对象。

**适用项目类型**: 使用关系型数据库且有独立 Schema 定义的项目（SQLite / PostgreSQL / MySQL）

**输入文件**:
- DDL/Schema 文件（`schema.sql`、`migrations/*.sql`）
- 应用代码（含 SQL 字符串和 ORM 模型）

**检查逻辑**:
```
1. 从 DDL 提取所有声明的对象: 表、视图、索引、触发器
2. 从应用代码提取所有引用的表名:
   a. 直接 SQL 字符串引用 (FROM, INSERT INTO, UPDATE, DELETE FROM)
   b. ORM 模型隐式引用
   c. 触发器维护的 FTS 表
3. 对比:
   - 表在 Schema 中存在但代码中无任何引用 → CONFIRMED 未使用表
   - FTS 表存在触发器但查询走 LIKE 基表 → FTS 表为死代码
   - 触发器定义但对应 FTS 表无人查询 → 触发器也是死代码
   - 索引存在但无引用该表的任何查询 → 索引可能无用（保守标记）
```

**实际发现（vet-knowledge 实例）**:
```
FTS5 表: diseases_fts, symptoms_fts, drugs_fts, cases_fts
触发器: diseases_ai/au/ad, symptoms_ai/au/ad, drugs_ai/au/ad, cases_ai/au/ad
问题: search.rs 使用 LIKE 查询基表，FTS5 表和触发器从未被使用
影响: schema 膨胀 + 写入时触发器执行但无查询收益
```

**输出格式**:
```
[CONFIRMED] FTS5 表为死代码:
  diseases_fts, symptoms_fts, drugs_fts, cases_fts (schema.sql:171-182)
  触发器: diseases_ai/au/ad, ...
  原因: search.rs 使用 LIKE 查询基表，FTS 仅定义未填充/查询

[CONFIRMED] learning_progress 表无引用 (schema.sql:194)
  无任何 SQL / ORM 引用此表
```

**退出码**: 0=全部被引用 | 1=存在无任何引用的表 | 2=存在未使用的索引（需人工确认）

**误报场景**: 预留表 / 仅管理后台手动查询 / 迁移工具写入但应用不读 / 报告查询使用但未在应用代码中硬编码表名

**配置参数**:

| 参数 | 说明 | 示例 |
|------|------|------|
| `schema_files` | DDL/Schema 文件路径 | `["src-tauri/data/seed/schema.sql"]` |
| `code_search_dirs` | 应用代码搜索目录 | `["src-tauri/src/"]` |
| `sql_extensions` | SQL 所在文件后缀 | `[".rs",".ts",".py",".sql"]` |
| `orm_framework` | ORM框架类型 | `sqlx` / `diesel` / `jpa` / `prisma` / `none` |
| `known_reserved` | 已知预留对象白名单 | `["audit_log_*","migration_*"]` |

---

## 置信度分层方案

所有规则统一使用以下置信度级别:

| 级别 | 含义 | 处理方式 |
|------|------|----------|
| **CONFIRMED** | 确定性问题，证据充分，无歧义 | 退出码 1，必须修复 |
| **HIGH** | 高度疑似，逻辑上几乎确定 | 退出码 1，应修复 |
| **REVIEW_NEEDED** | 需要人工判断，可能为误报 | 退出码 2，建议审查 |
| **INFO** | 仅供参考，不一定有问题 | 仅输出，不影响退出码 |
