# 兽医知识库 Code Wiki

> 本文档为 `vet-knowledge` 仓库的完整结构化 Code Wiki，涵盖项目整体架构、主要模块职责、关键类与函数说明、依赖关系以及项目运行方式。
>
> 项目简介：一个面向兽医专业学生与临床医师的结构化知识学习与诊断推理桌面平台。基于 Tauri 2 + Vue 3 + SQLite 构建，核心能力包括症状驱动的差异化诊断推理、SM-2 间隔重复闪卡复习、疾病对比与病例学习。

---

## 目录

- [1. 项目概览](#1-项目概览)
  - [1.1 项目定位](#11-项目定位)
  - [1.2 数据规模](#12-数据规模)
  - [1.3 技术栈一览](#13-技术栈一览)
- [2. 整体架构](#2-整体架构)
  - [2.1 分层架构图](#21-分层架构图)
  - [2.2 顶层目录结构](#22-顶层目录结构)
  - [2.3 启动与运行时数据流](#23-启动与运行时数据流)
- [3. 后端模块（Rust / Tauri 2）](#3-后端模块rust--tauri-2)
  - [3.1 模块总览](#31-模块总览)
  - [3.2 入口与启动流程](#32-入口与启动流程)
  - [3.3 数据库层 `db/`](#33-数据库层-db)
  - [3.4 诊断推理引擎 `engine.rs`](#34-诊断推理引擎-enginers)
  - [3.5 命令层 `commands/`](#35-命令层-commands)
- [4. 前端模块（Vue 3 + TypeScript）](#4-前端模块vue-3--typescript)
  - [4.1 模块总览](#41-模块总览)
  - [4.2 入口与路由](#42-入口与路由)
  - [4.3 布局组件](#43-布局组件)
  - [4.4 视图层 `views/`](#44-视图层-views)
  - [4.5 类型定义](#45-类型定义)
- [5. 数据层](#5-数据层)
  - [5.1 数据源组织](#51-数据源组织)
  - [5.2 数据库 Schema](#52-数据库-schema)
  - [5.3 YAML 关系文件](#53-yaml-关系文件)
  - [5.4 Markdown 实体格式](#54-markdown-实体格式)
- [6. 数据生成与维护工具](#6-数据生成与维护工具)
  - [6.1 `tools/gen_from_yaml.py`（核心数据管线）](#61-toolsgen_from_yamlpy核心数据管线)
  - [6.2 `tools/populate_drug_fields.py`（药物字段补全）](#62-toolspopulate_drug_fieldspy药物字段补全)
  - [6.3 其他工具脚本](#63-其他工具脚本)
- [7. 静态检查框架](#7-静态检查框架)
  - [7.1 框架定位与设计来源](#71-框架定位与设计来源)
  - [7.2 主入口 `_tools/dev/check.py`](#72-主入口-_toolsdevcheckpy)
  - [7.3 检查器 `checkers/`](#73-检查器-checkers)
  - [7.4 配置文件](#74-配置文件)
  - [7.5 规则定义](#75-规则定义)
- [8. 依赖关系](#8-依赖关系)
  - [8.1 前端依赖（package.json）](#81-前端依赖packagejson)
  - [8.2 后端依赖（Cargo.toml）](#82-后端依赖cargotoml)
  - [8.3 模块间依赖图](#83-模块间依赖图)
- [9. 项目运行方式](#9-项目运行方式)
  - [9.1 环境要求](#91-环境要求)
  - [9.2 普通用户：安装运行](#92-普通用户安装运行)
  - [9.3 开发者：从源码构建](#93-开发者从源码构建)
  - [9.4 数据变更工作流](#94-数据变更工作流)
  - [9.5 开发检查工作流](#95-开发检查工作流)
- [10. 关键设计说明](#10-关键设计说明)
  - [10.1 诊断推理引擎算法](#101-诊断推理引擎算法)
  - [10.2 SM-2 间隔重复算法](#102-sm-2-间隔重复算法)
  - [10.3 标签系统](#103-标签系统)
  - [10.4 数据库迁移与种子版本管理](#104-数据库迁移与种子版本管理)
  - [10.5 SQL 安全分割策略](#105-sql-安全分割策略)
- [11. 附录](#11-附录)
  - [11.1 Tauri 命令清单](#11-tauri-命令清单)
  - [11.2 路由清单](#12-路由清单)
  - [11.3 数据库表清单](#13-数据库表清单)
  - [11.4 文件路径速查](#14-文件路径速查)

---

## 1. 项目概览

### 1.1 项目定位

**兽医知识库** 是一个桌面端兽医学习与诊断推理平台，目标用户为兽医专业学生与临床医师。核心能力包括：

| 能力 | 描述 |
|------|------|
| 诊断推理引擎 | 输入症状组合，结合疾病-症状频率权重、核心症状加成、物种上下文，计算加权匹配分并返回排序的候选疾病列表 |
| SM-2 闪卡复习 | 基于 SuperMemo SM-2 间隔重复算法动态调整卡片复习间隔，优先推送即将遗忘的内容 |
| 病例学习 | 从主诉到预后完整理清临床决策链，每条病例关联对应疾病知识点 |
| 疾病对比 | 2–4 种疾病并排对比，从流行病学、病理生理、治疗方案多维度辅助鉴别诊断训练 |
| 全文检索 | 跨疾病/症状/药物/病例的 LIKE 模糊匹配搜索（FTS5 索引已建但搜索命令当前用 LIKE 实现） |
| 数据导入导出 | 命令已注册，实现待开发（`import_export.rs` 当前为占位） |

### 1.2 数据规模

| 实体 | 数量 | 数据源目录 |
|------|------|------------|
| 疾病 | 84 | `data/diseases/dis_*.md` |
| 症状 | 43 | `data/symptoms/sym_*.md` |
| 药物 | 69 | `data/drugs/drug_*.md` |
| 诊断检查 | 36 | `data/tests/test_*.md` |
| 病例 | 15 | `data/cases/case_*.md` |
| 治疗方案 | 6 | `data/treatments/trt_*.md` |
| 预置标签 | 33 | `data/tags.yaml` |

### 1.3 技术栈一览

| 层 | 技术 | 说明 |
|----|------|------|
| 桌面外壳 | Tauri 2 + WebView2 | 内存占用 < 80MB，无 Electron 式 Node 运行时 |
| 前端 | Vue 3.5 + TypeScript 5.6 + Vite 6 | `<script setup>` Composition API |
| 状态/路由 | Pinia 2 + Vue Router 4 | |
| 后端 | Rust 1.70+ (edition 2021) | Tauri 2 命令模式 |
| 数据库 | SQLite (WAL + FTS5) | sqlx 0.8 异步驱动 |
| 数据格式 | Markdown + Frontmatter（实体） / YAML（关系） | |
| 数据生成 | Python 3 脚本 | `tools/gen_from_yaml.py` |
| 静态检查 | 自研 6 规则框架 | `_tools/dev/check.py` |

---

## 2. 整体架构

### 2.1 分层架构图

```
┌──────────────────────────────────────────────────────────┐
│                    Desktop Shell                          │
│                 (Tauri 2 / WebView2)                      │
├───────────────────────┬──────────────────────────────────┤
│       Frontend        │           Rust Backend            │
│                       │                                  │
│  Vue 3 + TypeScript   │  Tauri Commands (invoke_handler) │
│  Pinia + Vue Router   │  Diagnosis Engine (加权评分)      │
│  Vite 6 (dev server)  │  SM-2 Scheduler                  │
│                       │  SQLite (WAL + FTS5)             │
│  invoke('cmd', args)  │  sqlx 0.8 (async)                │
└───────────────────────┴──────────────────────────────────┘
              ↑                          ↑
              │   Tauri IPC bridge        │
              └──────────────────────────┘

┌──────────────────────────────────────────────────────────┐
│                    Data Pipeline                          │
│  data/*.md  +  data/*.yaml  ──►  tools/gen_from_yaml.py  │
│                                       │                   │
│                                       ▼                   │
│              src-tauri/data/seed/001_initial.sql           │
└──────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────┐
│                Static Analysis Framework                 │
│  _tools/dev/check.py  +  checkers/*.py  (6 rules)         │
│  check_config.json  ←→  check_config_schema.json         │
│            Design source: docs/WORKFLOW_REFORM.md         │
└──────────────────────────────────────────────────────────┘
```

### 2.2 顶层目录结构

```
vet-knowledge/
├── src/                      # 前端 Vue 3 源码
│   ├── App.vue              # 根布局
│   ├── main.ts              # Vue 应用入口
│   ├── assets/              # 全局样式
│   ├── components/           # 通用组件（Sidebar、TopBar）
│   ├── router/              # Vue Router 配置
│   ├── types/                # TypeScript 类型定义
│   └── views/               # 页面视图
│       ├── knowledge/       # 知识库页面（疾病/症状/药物/诊断）
│       ├── learning/        # 学习页面（病例/闪卡）
│       ├── graph/           # 知识图谱（占位）
│       └── game/            # 诊断游戏（占位）
├── src-tauri/                # Tauri 2 后端（Rust）
│   ├── src/                 # Rust 源码
│   │   ├── main.rs          # 二进制入口
│   │   ├── lib.rs           # 库入口 + Tauri Builder + 命令注册
│   │   ├── engine.rs       # 诊断推理引擎
│   │   ├── db/              # 数据库模块
│   │   └── commands/        # Tauri 命令实现
│   ├── data/seed/           # SQL schema + 种子数据
│   ├── gen/schemas/         # Tauri 生成的 schema
│   ├── icons/               # 应用图标
│   ├── Cargo.toml           # Rust 依赖清单
│   ├── build.rs            # Tauri 构建脚本
│   └── tauri.conf.json     # Tauri 应用配置
├── data/                     # 数据源（实体 MD + 关系 YAML）
│   ├── cases/              # 病例 MD 文件
│   ├── diseases/           # 疾病 MD 文件
│   ├── drugs/              # 药物 MD 文件
│   ├── symptoms/           # 症状 MD 文件
│   ├── tests/              # 检查 MD 文件
│   ├── treatments/        # 治疗方案 MD 文件
│   ├── relations.yaml     # 疾病-症状、DDX 关系
│   ├── treatment_rules.yaml  # 疾病-药物/检查/治疗 关系
│   └── tags.yaml          # 预置标签库
├── tools/                    # 数据生成/维护脚本
│   ├── gen_from_yaml.py    # 核心：YAML/MD → SQL
│   ├── populate_drug_fields.py
│   └── ...
├── _tools/                   # 跨项目通用静态检查框架
│   └── dev/
│       ├── check.py         # 框架主入口
│       ├── checkers/        # 6 个检查器
│       ├── check_config_schema.json
│       └── ...
├── _knowledge/               # 项目知识库（规则与 SOP）
│   └── rules/static_analysis_rules.md
├── docs/                     # 设计文档
│   └── WORKFLOW_REFORM.md   # 工作流改革方案
├── index.html                # 前端入口 HTML
├── package.json              # 前端依赖与脚本
├── vite.config.ts            # Vite 配置
├── tsconfig.json             # TypeScript 配置
├── check_config.json         # 项目检查配置
├── 启动兽医知识库.bat         # 一键启动脚本
├── 启动知识库.bat
├── CLAUDE.md                 # Claude Code 工作指南
├── README.md
└── LICENSE                   # MIT
```

### 2.3 启动与运行时数据流

```
用户启动 → main.rs::main() → lib.rs::run()
                              │
                              ▼
                Tauri Builder 加载插件（shell、fs）
                              │
                              ▼
                setup hook 异步初始化数据库
                              │
                              ▼
            db::init(app) ───► 连接 SQLite (WAL)
                              │
                              ├── PRAGMA 优化（journal_mode、cache_size...）
                              ├── strip_sql_comments(schema.sql)
                              ├── split_sql_statements → 逐条建表
                              ├── apply_migrations (v1–v9)
                              └── import_seed_data
                                    │
                                    ├── 比较 SEED_DATA_VERSION
                                    ├── 版本不匹配时事务内清空+导入
                                    └── 写入 app_meta('seed_data_version')
                              │
                              ▼
            注册 invoke_handler (27 个 Tauri 命令)
                              │
                              ▼
            启动 WebView2 加载前端 dist / Vite dev server
                              │
                              ▼
            前端 invoke('cmd', {args}) ──► 后端命令 ──► SQL → 返回 JSON
```

---

## 3. 后端模块（Rust / Tauri 2）

### 3.1 模块总览

```
src-tauri/src/
├── main.rs              # 二进制入口，仅调用 vet_knowledge_lib::run()
├── lib.rs               # 库入口：Tauri Builder + 插件 + 命令注册
├── engine.rs            # 诊断推理引擎（纯函数，无 DB 依赖）
├── db/
│   ├── mod.rs           # DB 初始化、PRAGMA、迁移、种子导入
│   └── models.rs        # sqlx FromRow 结构体定义
└── commands/
    ├── mod.rs           # 模块导出声明
    ├── knowledge.rs     # 知识库 CRUD（疾病/症状/药物/检查/病例）
    ├── treatments.rs    # 治疗方案 + 标签系统命令
    ├── diagnose.rs      # 诊断推理命令（编排 engine + 多次 SQL 查询）
    ├── flashcards.rs    # 闪卡系统（SM-2 算法实现）
    ├── search.rs        # 全文搜索（LIKE 实现）
    └── import_export.rs # 数据导入导出（占位，待实现）
```

### 3.2 入口与启动流程

#### `src-tauri/src/main.rs`
- **职责**：二进制入口点，仅调用库入口。
- **关键代码**：
  ```rust
  #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
  fn main() { vet_knowledge_lib::run() }
  ```
- **设计要点**：`windows_subsystem = "windows"` 在 release 模式下隐藏控制台窗口，符合 Windows GUI 应用规范。

#### `src-tauri/src/lib.rs`
- **职责**：Tauri 应用主入口，负责插件加载、DB 初始化、命令注册。
- **关键函数**：`pub fn run()` — 构建 `tauri::Builder`：
  1. 加载插件：`tauri_plugin_shell::init()`、`tauri_plugin_fs::init()`
  2. `setup` 钩子：异步调用 `db::init(&app_handle)`，成功后 `app_handle.manage(db)` 把连接池注入 Tauri 状态供命令使用；失败时调用 `db::write_import_error_log` 写入 `seed_import_error.log` 后 panic
  3. `invoke_handler`：注册 27 个 Tauri 命令（详见 [11.1](#11-tauri-命令清单)）
- **模块声明**：`mod commands; mod db; mod engine;`（三个内部模块）

### 3.3 数据库层 `db/`

#### `db/mod.rs`
- **职责**：SQLite 连接池管理、Schema 初始化、迁移、种子数据导入。
- **类型别名**：`pub type DbPool = Pool<Sqlite>;`
- **关键常量**：`const SEED_DATA_VERSION: i64 = 18;` — 递增此值会触发应用启动时重新导入种子数据。
- **关键函数**：

| 函数 | 作用 |
|------|------|
| `init(app: &AppHandle) -> anyhow::Result<DbPool>` | 数据库初始化主流程：定位 app_data_dir → 创建 `vet_knowledge.db` → 连接池 → PRAGMA 优化 → 加载 schema.sql → apply_migrations → import_seed_data |
| `split_sql_statements(script) -> Vec<&str>` | 状态机式 SQL 分号分割，正确处理单/双引号内分号与 `''` 转义 |
| `strip_sql_comments(script) -> String` | 剥离 SQL 单行 `--` 注释，避免注释语句被分割器跳过 |
| `apply_migrations(pool) -> Result<()>` | 9 个迁移版本（v1–v9），通过 `schema_migrations` 表追踪已应用版本；全新库直接写基线，旧库按版本号顺序回放 |
| `import_seed_data(pool) -> Result<()>` | 通过 `app_meta.seed_data_version` 判断是否需要重导；事务内清空 13 张数据表 → 导入 `001_initial.sql` → 重建 FTS → 写入新版本号 |
| `write_import_error_log(app, err)` | 失败时写入 `seed_import_error.log`，含时间戳、版本号、错误链、排查指引 |

- **PRAGMA 优化（桌面端最佳实践）**：
  ```sql
  PRAGMA journal_mode = WAL;
  PRAGMA synchronous = NORMAL;
  PRAGMA cache_size = -64000;     -- 64MB
  PRAGMA foreign_keys = ON;
  PRAGMA temp_store = MEMORY;
  ```
- **迁移历史**：

| 版本 | 描述 |
|------|------|
| v1 | 初始 schema：所有表 + 索引 + FTS + learning_progress |
| v2 | diseases 加 `urgency_level`，disease_symptom 加 `is_pathognomonic` |
| v3 | 重建 disease_diagnostic 主键含 species |
| v4 | 重建 disease_treatment 主键含 species |
| v5 | 闪卡系统表（flashcards + flashcard_reviews） |
| v6 | diseases 加 `data_source` / `is_deleted` |
| v7 | 标签系统 + 治疗模块 + 疾病/症状/药物四维度增强 |
| v8 | cases_fts 全文搜索虚拟表 |
| v9 | tags 表去掉 UNIQUE(name_zh) 允许同标签名跨分组 |

- **单元测试**：包含 4 个 `split_sql_statements` 测试（基础分割、引号内分号、转义引号、注释处理）。

#### `db/models.rs`
- **职责**：SQLx 模型结构体，对应数据库表结构，供 `query_as` 自动反序列化。
- **派生宏**：`#[derive(Debug, Serialize, Deserialize, FromRow)]`，同时支持 JSON 序列化（给前端）和 SQL 行映射（给 sqlx）。
- **核心结构体**：

| 结构体 | 对应表 | 用途 |
|--------|--------|------|
| `Disease` | diseases | 84 字段疾病实体（含拉丁名、病理生理、生理基础等） |
| `Symptom` | symptoms | 症状实体（含生理基础） |
| `Drug` | drugs | 药物实体（含 mechanism_of_action/pk_pd/adverse_mechanism） |
| `DiagnosticTest` | diagnostic_tests | 检查项（含 cost_estimate/turnaround_time） |
| `Case` | cases | 病例（含 9 个临床字段） |
| `Treatment` | treatments | 治疗方案（含 6 种 therapy_type） |
| `Tag` | tags | 标签（含 emergency_level/clinical_action） |
| `EntityTag` | entity_tags | 多态关联（entity_type + entity_id + tag_id） |
| `DiseaseWithSymptom` | JOIN 结果 | 症状→疾病反向查询的复合结构 |
| `DiseaseSymptom` | disease_symptom | 疾病-症状关联（含 frequency/stage/is_pathognomonic） |
| `DiseaseDdx` | disease_ddx | 鉴别诊断关联 |
| `DiseaseTreatment` | disease_treatment | 疾病-药物关联（含 line/species/notes） |
| `DiseaseDiagnostic` | disease_diagnostic | 疾病-检查关联（含 purpose/evidence_level） |
| `DiseaseTreatmentMap` | disease_treatment_map | 疾病-治疗方案关联 |

### 3.4 诊断推理引擎 `engine.rs`

- **职责**：纯函数式推理引擎，无数据库依赖，输入数据全部由调用方 `diagnose.rs` 准备。
- **核心结构体**：

| 结构体 | 字段 |
|--------|------|
| `DiagnosisInput` | `symptoms: Vec<String>`, `species: String`, `age: Option<f64>`, `breed: Option<String>` |
| `DiagnosisCandidate` | `disease_id`, `disease_name`, `match_score`, `input_coverage`, `matched_symptoms`, `missing_key_symptoms`, `suggested_tests`, `distinguishing_features` |
| `TestSuggestion` | `test_id`, `test_name`, `purpose` |

- **核心函数**：`pub fn infer(input, disease_list, all_disease_symptoms, diagnostics) -> Vec<DiagnosisCandidate>`
- **算法细节**：详见 [10.1](#101-诊断推理引擎算法)。

### 3.5 命令层 `commands/`

所有命令遵循 Tauri 2 模式：
- 函数签名：`pub async fn xxx(pool: tauri::State<'_, DbPool>, ...) -> Result<T, String>`
- 错误处理：`map_err(|e| e.to_string())` 把 sqlx 错误转字符串返回前端
- 通过 `lib.rs` 的 `invoke_handler!` 宏注册后，前端用 `invoke('xxx', { args })` 调用

#### `commands/diagnose.rs` — 诊断推理命令
- **职责**：编排 `engine::infer`，从 DB 拉取数据 → 调用纯函数推理。
- **命令**：`infer_diagnosis(symptoms, species, age, breed)`
- **数据准备流程**：
  1. SQL 查询匹配 species 的疾病列表 `Vec<(id, name)>`
  2. 用 `IN (...)` 占位符批量查询这些疾病的所有症状 → `HashMap<disease_id, Vec<(sym_name, freq, is_pathognomonic)>>`
  3. 同样批量查询这些疾病的诊断检查 → `HashMap<disease_id, Vec<(test_id, purpose)>>`
  4. 调用 `engine::infer(input, &disease_list, &all_disease_symptoms, &diagnostics)` 计算候选
- **注意**：物种过滤用 `LIKE '%species%'` 模糊匹配（species 字段在 SQL 中是 JSON 数组字符串）。

#### `commands/flashcards.rs` — 闪卡系统
- **职责**：SM-2 间隔重复算法的数据库层实现。
- **自定义结构体**：`Flashcard`（含 next_review/review_count/ease_factor）、`ReviewStats`
- **命令清单**：

| 命令 | 作用 |
|------|------|
| `get_due_flashcards(limit)` | 查询今日到期的闪卡（`next_review <= datetime('now')` 或首次复习） |
| `get_all_flashcards(card_type?)` | 管理用全量查询，可选按类型过滤 |
| `generate_flashcards_from_knowledge(card_type)` | 从知识库自动生成闪卡（disease/symptom/drug 三类），跳过已存在的 |
| `create_flashcard(front, back)` | 创建自定义闪卡，ID 格式 `fc_custom_{timestamp}` |
| `delete_flashcard(id)` | 删除闪卡（级联删除复习记录） |
| `review_flashcard(card_id, quality)` | 提交复习评分 0-5，应用 SM-2 算法计算下次间隔（详见 [10.2](#102-sm-2-间隔重复算法)） |
| `get_review_stats()` | 复习统计：total_cards/due_today/reviewed_today/mastered |

- **算法亮点**：`review_flashcard` 完整实现 SM-2 公式：
  - EF 更新：`EF' = max(1.3, EF + (0.1 - (5-q)*(0.08 + (5-q)*0.02)))`
  - quality < 3 时重置 interval = 1 天
  - 首次复习根据 quality 决定 1 天或 6 天

#### `commands/knowledge.rs` — 知识库 CRUD
- **职责**：6 类实体的查询命令 + 复合视图。
- **命令清单**：

| 命令 | 作用 |
|------|------|
| `get_diseases(species?, category?)` | 疾病列表，支持物种/分类过滤（LIKE 模糊匹配） |
| `get_disease_by_id(id)` | 单个疾病详情 |
| `get_disease_symptoms(disease_id)` | 疾病关联症状（按 frequency DESC 排序） |
| `get_disease_ddx(disease_id)` | 疾病鉴别诊断列表 |
| `get_disease_treatments(disease_id)` | 疾病一线/二线/辅助用药 |
| `get_disease_diagnostics(disease_id)` | 疾病推荐检查项 |
| `get_disease_compare(disease_ids)` | 疾病对比视图（聚合多个查询） |
| `get_symptoms()` | 全部症状 |
| `get_symptom_by_id(id)` | 单个症状 |
| `get_diseases_by_symptom(symptom_id, species?)` | 症状→疾病反向查找 |
| `get_drugs(drug_class?)` | 药物列表，支持分类过滤 |
| `get_drug_by_id(id)` | 单个药物 |
| `get_tests()` | 全部检查项 |
| `get_test_by_id(id)` | 单个检查项 |
| `get_cases(species?, difficulty?)` | 病例列表 |
| `get_case_by_id(id)` | 单个病例 |
| `get_case_diseases(case_id)` | 病例关联疾病 |

- **辅助函数**：`row_to_disease`、`row_to_drug`、`row_to_test` — 把 `SqliteRow` 手工映射到结构体（用于 JOIN 查询场景，sqlx `query_as` 无法直接处理）。
- **自定义视图结构体**：`DiseaseCompareView`（聚合 disease + symptoms + treatments + diagnostics + ddx，供对比页面一次拉取）。

#### `commands/treatments.rs` — 治疗方案与标签系统
- **命令清单**：

| 命令 | 作用 |
|------|------|
| `get_treatments(therapy_type?)` | 治疗方案列表 |
| `get_treatment_by_id(id)` | 单个治疗方案 |
| `get_disease_treatment_map(disease_id)` | 疾病关联的综合治疗方案（区别于单药 treatment） |
| `get_tags(tag_group?)` | 标签列表，可按分组过滤 |
| `get_entity_tags(entity_type, entity_id)` | 实体的所有标签 |
| `get_entities_by_tag(tag_id, entity_type)` | 按标签反查实体 ID |
| `add_entity_tag(entity_type, entity_id, tag_id)` | 添加标签关联（INSERT OR IGNORE 幂等） |
| `remove_entity_tag(entity_type, entity_id, tag_id)` | 删除标签关联 |

#### `commands/search.rs` — 全文搜索
- **命令**：`full_text_search(query, limit?)`
- **结构体**：`SearchResult`（entity_type/entity_id/title/snippet/relevance）
- **实现细节**：当前用 `LIKE '%query%'` 跨 4 张表（diseases/symptoms/drugs/cases）分别查询，按 relevance 排序合并。FTS5 虚拟表已在 schema 中创建（`diseases_fts` 等），但搜索命令尚未使用 FTS MATCH 语法。
- **相关性评分**：name_zh 命中 1.0，overview 命中 0.7，symptom 0.8，drug 0.6，case 0.75。

#### `commands/import_export.rs` — 数据导入导出
- **当前状态**：占位文件，仅含一行注释 `// 导入导出功能已禁用 — 等待真实需求确认后实现`。
- **注意**：`lib.rs` 中未注册此模块的命令。

---

## 4. 前端模块（Vue 3 + TypeScript）

### 4.1 模块总览

```
src/
├── App.vue               # 根布局（Sidebar + TopBar + router-view）
├── main.ts               # 应用入口（createApp + Pinia + Router）
├── vite-env.d.ts         # Vite 环境类型
├── assets/
│   └── main.css          # 全局补充样式（滚动条、工具类）
├── components/
│   ├── Sidebar.vue       # 侧边导航（10 个菜单项）
│   └── TopBar.vue        # 顶部搜索栏
├── router/
│   └── index.ts          # 12 条路由定义
├── types/
│   └── index.ts          # 全部 TS 接口（与后端 Rust 模型镜像）
└── views/
    ├── knowledge/        # 知识库页面
    │   ├── Home.vue                  # 首页仪表盘
    │   ├── DiseaseList.vue           # 疾病列表（三维筛选）
    │   ├── DiseaseDetail.vue         # 疾病详情（含标签云）
    │   ├── DiseaseCompare.vue        # 疾病对比
    │   ├── DrugHandbook.vue          # 药物手册（左右分栏）
    │   ├── SymptomExplorer.vue       # 症状→疾病反向查找
    │   └── SymptomChecker.vue        # 症状推理
    ├── learning/         # 学习页面
    │   ├── CaseLibrary.vue           # 病例库
    │   ├── CaseDetail.vue            # 病例详情（章节展开）
    │   ├── CaseStudy.vue             # 病例推理训练（占位）
    │   └── FlashcardStudy.vue        # 闪卡复习
    ├── graph/
    │   └── KnowledgeGraph.vue        # 知识图谱（占位）
    └── game/
        └── GameHome.vue              # 诊断游戏（占位）
```

### 4.2 入口与路由

#### `src/main.ts`
- **职责**：Vue 应用挂载入口。
- **关键代码**：
  ```ts
  import { createApp } from 'vue'
  import { createPinia } from 'pinia'
  import App from './App.vue'
  import router from './router'
  import './assets/main.css'
  const app = createApp(App)
  app.use(createPinia())
  app.use(router)
  app.mount('#app')
  ```

#### `src/router/index.ts`
- **职责**：Vue Router 4 配置，使用 `createWebHistory`。
- **路由列表**：详见 [11.2](#12-路由清单)。
- **设计要点**：全部使用动态 `import()` 懒加载，减小首屏 bundle 体积。

#### `vite.config.ts`
- **职责**：Vite 6 配置。
- **关键配置**：
  - `plugins: [vue()]`
  - `resolve.alias['@'] = 'src'`（路径别名）
  - `server.port = 1420` + `strictPort: true`（Tauri 期望端口）
  - `envPrefix: ['VITE_', 'TAURI_']`

#### `tsconfig.json`
- **关键配置**：`strict: true`、`target: ES2022`、`moduleResolution: bundler`、`noUnusedLocals/Parameters: true`、`paths['@/*'] = ['src/*']`

### 4.3 布局组件

#### `src/App.vue`
- **职责**：根布局，左侧 Sidebar + 右侧主内容（TopBar + router-view）。
- **关键 CSS 变量定义**（`:root`）：
  ```css
  --sidebar-width: 220px;
  --topbar-height: 48px;
  --color-primary: #2563eb;
  --color-bg: #f8fafc;
  --color-surface: #ffffff;
  --color-border: #e2e8f0;
  --color-text: #1e293b;
  --color-success: #16a34a;
  --color-warning: #d97706;
  --color-danger: #dc2626;
  --radius: 8px;
  --shadow: 0 1px 3px rgba(0,0,0,0.1);
  ```
- **字体**：`'Noto Sans SC', -apple-system, sans-serif`（在 `index.html` 通过 Google Fonts 加载）。

#### `src/components/Sidebar.vue`
- **职责**：左侧导航栏，10 个菜单项。
- **菜单项**：首页、疾病百科、症状→疾病、药物手册、症状推理、知识图谱、病例库、闪卡复习、疾病对比、诊断游戏。
- **样式**：使用 `route.path === item.path` 判断 active 状态。

#### `src/components/TopBar.vue`
- **职责**：顶部搜索栏。
- **调用的 invoke 命令**：`full_text_search(query, limit=10)`
- **交互**：输入框 + 搜索按钮 + 下拉结果列表（显示 entity_type 徽章 + title + snippet）。

### 4.4 视图层 `views/`

所有视图遵循统一约定：
- `<script setup lang="ts">` Composition API
- `onMounted` 中通过 `Promise.all` 并发拉取多源数据
- 标签系统统一通过 `get_tags` 拉取后构建 `Map<id, Tag>`，再按 `tag_group` 分组渲染
- 错误用 `try/catch` 包裹，失败时静默或 `console.error`

#### 知识库页面

| 页面 | 文件 | 主要 invoke 命令 | 关键特性 |
|------|------|------------------|----------|
| 首页仪表盘 | `knowledge/Home.vue` | `get_diseases`、`get_symptoms`、`get_drugs`、`get_cases` | 4 列统计卡片 + 8 个分类入口 + 最近 6 条疾病 |
| 疾病列表 | `knowledge/DiseaseList.vue` | `get_diseases(species, category)` | 物种/系统/难度三维筛选，从路由 query 读取预选 |
| 疾病详情 | `knowledge/DiseaseDetail.vue` | `get_disease_by_id`、`get_disease_symptoms`、`get_disease_ddx`、`get_disease_treatment_map`、`get_tags`（并发） | 急诊等级徽章（red/orange/yellow/green）、解剖系统标签云、病理机制标签云、DAMNIT-V 标签、高急迫性警告条（urgency≥4） |
| 疾病对比 | `knowledge/DiseaseCompare.vue` | `get_diseases`、`get_disease_compare(diseaseIds)` | 2-4 疾病选择，5 个对比 section（基本信息/症状矩阵/治疗/检查/DDX），症状矩阵用 ●/★ 频率点 |
| 药物手册 | `knowledge/DrugHandbook.vue` | `get_drugs`、`get_tags`（并发） | 左右分栏（列表 320px + 详情），机制标签云，物种剂量用 `<pre>` 保留格式 |
| 症状检索 | `knowledge/SymptomExplorer.vue` | `get_symptoms`、`get_diseases_by_symptom(symptomId, species)` | 左右双栏，支持路由 query 预选症状（与 DiseaseDetail 联动），核心症状星标 |
| 症状推理 | `knowledge/SymptomChecker.vue` | `get_symptoms`、`infer_diagnosis(symptoms, species, age, breed)`、`get_disease_by_id`（批量） | 症状池点击选择 + 物种下拉 + 推理按钮，结果按 match_score 排序，分数颜色编码（绿/橙/灰），紧急徽章（urgency≥4） |

#### 学习页面

| 页面 | 文件 | 主要 invoke 命令 | 关键特性 |
|------|------|------------------|----------|
| 病例库 | `learning/CaseLibrary.vue` | `get_cases(species, difficulty)`、`full_text_search` | 物种/难度筛选 + 内联搜索，卡片含 meta tag |
| 病例详情 | `learning/CaseDetail.vue` | `get_case_by_id`、`get_case_diseases`、`get_tags`（并发） | 9 个章节展开/折叠（默认展开主诉+诊断），右侧 sticky 侧栏（动物信息+关联疾病+操作入口），diagnosis 章节高亮，跳转 `/cases/:id/study` |
| 病例推理训练 | `learning/CaseStudy.vue` | 无 | 占位页面，开发中 |
| 闪卡复习 | `learning/FlashcardStudy.vue` | `get_review_stats`、`get_due_flashcards(limit=20)`、`review_flashcard(cardId, quality)`、`generate_flashcards_from_knowledge(cardType)` | 4 列统计条 + 3D 翻转闪卡 + 4 级评分按钮（0/2/3/5）+ 完成态庆祝，每个评分按钮显示下次复习间隔提示 |

#### 占位页面
- `graph/KnowledgeGraph.vue` — 知识图谱（"将展示疾病-症状-药物关系网络"）
- `game/GameHome.vue` — 诊断游戏（"将集成 Virtual Vet 诊断游戏"）
- `learning/CaseStudy.vue` — 病例推理训练（"开发中"）

### 4.5 类型定义

#### `src/types/index.ts`
- **职责**：与后端 Rust 模型镜像的 TypeScript 接口。
- **核心接口**：`Tag`、`EntityTag`、`Disease`、`Symptom`、`Drug`、`Treatment`、`DiseaseTreatmentMap`、`DiagnosticTest`、`Case`、`SearchResult`、`DiagnosisCandidate`、`TestSuggestion`。
- **注意差异**：前端 `Disease.tags: string[]`、`Treatment.procedure`（后端为 `procedure_text`）、`Treatment.prognosis_eval`（后端为 `prognosis_assessment`）——这些不一致是 `_tools/dev/checkers/type_check.py` 检查的目标（详见 [7.3](#73-检查器-checkers)）。

---

## 5. 数据层

### 5.1 数据源组织

项目采用 **"Markdown+Frontmatter 数据源 + YAML 关系 + Python 生成 SQL"** 的三层架构：

```
data/*.md (实体)         data/*.yaml (关系)         data/tags.yaml (标签)
   │                          │                          │
   └──────────► tools/gen_from_yaml.py ◄──────────────────┘
                          │
                          ▼
            src-tauri/data/seed/001_initial.sql
                          │
                          ▼  (匹配 schema.sql 契约)
                Tauri SQLite 数据库
```

**设计原则**：
- 实体数据用 Markdown：frontmatter 存结构化字段，body 存长文本章节
- 关系数据用 YAML：多对多关联不适合拆成单个 MD
- 种子数据由脚本生成：避免手写 SQL 易错，便于版本控制

### 5.2 数据库 Schema

完整 schema 定义于 `src-tauri/data/seed/schema.sql`，包含：

#### 核心实体表（6 张）
| 表 | 字段亮点 |
|----|----------|
| `diseases` | id, name_zh/en/latin, category(JSON), species(JSON), body_system, pathogenic_type, epidemiology, urgency_level(1-5), physiological_basis, overview, etiology, pathophysiology, prognosis, difficulty, data_source(seed/user/imported), is_deleted |
| `symptoms` | id, name_zh/en, definition, species_notes, physiological_basis |
| `drugs` | id, name_zh/en, drug_class, mechanism_of_action, pk_pd, adverse_mechanism, indications, contraindications, side_effects, species_dosing |
| `diagnostic_tests` | id, name_zh, category, reference_ranges, interpretation, cost_estimate(REAL), turnaround_time(INTEGER 分钟) |
| `cases` | id, title, species, breed, age(REAL), weight(REAL), 9 个临床字段, difficulty |
| `treatments` | id, name_zh/en, therapy_type(枚举 6 种), principle, procedure_text, physiological_basis, prognosis_assessment |

#### 关系表（6 张）
| 表 | 关联 | 关键字段 |
|----|------|----------|
| `disease_symptom` | 疾病↔症状 | frequency(common/uncommon/rare), stage, is_pathognomonic(0/1) |
| `disease_ddx` | 疾病↔疾病 | distinguishing_feature |
| `disease_treatment` | 疾病↔药物 | line(first/second/adjunctive), species, notes |
| `disease_diagnostic` | 疾病↔检查 | purpose(screening/confirming/monitoring), evidence_level(gold_standard/supportive/exclusionary), expected_result |
| `disease_treatment_map` | 疾病↔治疗方案 | line, species, notes |
| `case_disease` | 病例↔疾病 | — |

#### 标签系统（2 张）
| 表 | 用途 |
|----|------|
| `tags` | 预置标签库，6 个 tag_group（body_system/mechanism/emergency/damnit_v/species/custom），emergency 含 4 个临床决策字段 |
| `entity_tags` | 多态关联（entity_type + entity_id + tag_id），主键三元组 |

#### FTS5 虚拟表（4 张）
`diseases_fts`、`symptoms_fts`、`drugs_fts`、`cases_fts` — 外部内容模式（`content=表名, content_rowid=rowid`）。

#### 学习系统表（3 张）
- `learning_progress` — 间隔重复学习进度（预留）
- `flashcards` — 闪卡（id/front/back/card_type/entity_id/difficulty/created_at）
- `flashcard_reviews` — 复习记录（card_id/quality 0-5/interval_days/ease_factor/next_review）

#### 元数据表
- `schema_migrations` — 迁移版本追踪（version/description/applied_at）
- `app_meta` — 应用元数据键值对（如 `seed_data_version`）

#### 索引（9 个）
覆盖关系表外键、标签查询热点、闪卡复习调度。

### 5.3 YAML 关系文件

#### `data/relations.yaml`
- **职责**：疾病-症状、疾病-疾病（DDX）关系。
- **结构**：
  ```yaml
  disease_symptoms:
    - disease: dis_001
      symptoms:
        - symptom: sym_001
          frequency: common      # common|uncommon|rare
          stage: 全程             # 全程|进展|晚期|发作期|...
          is_pathognomonic: true  # 可选，标记核心症状
  ddx:
    - disease: dis_001
      target: dis_002
      feature: "鉴别要点文本"
  ```

#### `data/treatment_rules.yaml`
- **职责**：疾病-药物、疾病-检查、疾病-治疗方案三类关联。
- **结构**：
  ```yaml
  disease_treatment:
    - disease: dis_001
      drugs:
        - drug: drug_001
          line: first           # first|second|adjunctive
          species: null          # null=通用
          notes: "临床要点"
  disease_diagnostic:
    - disease: dis_001
      tests:
        - test: test_001
          purpose: confirming    # screening|confirming|monitoring
          evidence_level: gold_standard  # gold_standard|supportive|exclusionary
          expected_result: "预期结果描述"
  disease_treatment_map:
    - disease: dis_001
      treatments:
        - treatment: trt_001
          line: first
  ```

#### `data/tags.yaml`
- **职责**：预置标签库（33 个标签，5 个分组）。
- **分组**：
  - `body_system`（11 个）：呼吸/消化/心血管/泌尿/内分泌/神经/皮肤/肌骨/眼耳/生殖/免疫
  - `mechanism`（11 个）：炎症免疫/代谢/血管循环/梗阻/疼痛/感染/中毒/肿瘤/免疫介导/退行性/遗传
  - `emergency`（4 个）：red/orange/yellow/green，每项含 `emergency_level`、`clinical_action`、`textbook_logic`、`typical_scenario`
  - `damnit_v`（7 个）：DAMNIT-V 诊断模型分类
  - `species`（5 个）：犬/猫/马/牛/异宠

### 5.4 Markdown 实体格式

每个实体文件遵循 **frontmatter + body sections** 格式：

#### `data/diseases/dis_001.md`（示例：肺炎）
```markdown
---
id: dis_001
name_zh: 肺炎
name_en: Pneumonia
name_latin: ...
body_system: 呼吸系统
category: ["呼吸系统"]
species: ["犬", "猫"]
pathogenic_type: 细菌性
difficulty: intermediate
urgency_level: 3
epidemiology: ...
physiological_basis: ...
tags: [tag_resp, tag_infect]
---

## 概述
...

## 病因
- 病因1
- 病因2

## 病理生理
...

## 预后
...
```

#### 其他实体格式要点
- **症状**：body 含 `## 定义`、`## 物种特异性`（用 `**犬**:` 加粗格式）
- **药物**：body 含 `## 适应症`、`## 禁忌症`、`## 不良反应`、`## 物种剂量`（`**犬**: dose=..., route=..., frequency=...`）
- **检查**：body 含 `## 参考范围`、`## 结果解读`
- **病例**：body 含 9 个标准章节（主诉/病史/体格检查/实验室检查/影像学/诊断/治疗/转归/学习要点）
- **治疗方案**：body 含 `## 治疗原则`、`## 操作指南`（编号步骤）、`## 生理基础`、`## 预后评估`

---

## 6. 数据生成与维护工具

### 6.1 `tools/gen_from_yaml.py`（核心数据管线）

- **职责**：从 `data/` 下所有 MD 文件和 YAML 关系文件，生成完整的 `001_initial.sql` 种子数据。
- **是连接数据源与 SQL 输出的唯一桥梁**。

#### 关键函数

| 函数 | 作用 |
|------|------|
| `S(s)` / `V(v)` | SQL 字面量转义：单引号双写、NULL 处理、数字直接返回 |
| `parse_list_text(text)` | 把 Markdown `- item` 列表解析为 Python list |
| `parse_species_dict(text)` | 把 `**犬**: xxx` 加粗格式或 `犬：xxx；猫：yyy` 分号格式解析为 dict（自动检测两种语法） |
| `to_json(v)` | list/dict 序列化为 JSON 字符串（`ensure_ascii=False`） |
| `parse_body_sections(content)` | 用正则 `^##\s+(.+?)\n(.*?)(?=\n##\s+|\Z)` 解析 MD body 章节 |
| `load_md_entities(pattern, body_fields)` | 加载某目录下所有 MD，按 `body_fields` 映射将 body section 合并到 frontmatter 字段 |
| `resolve_tag_id(tag_ref)` | 三级解析：预置 id → 预置中文名 → slug 回退 |
| `collect_tags(entity_list, default_group)` | 从实体 tags 字段收集未注册的自定义标签 |
| `INSERT_ROW(table, columns, values)` | 生成单行 INSERT 语句 |

#### 生成流程

```
1. 加载 6 类实体 MD（diseases/symptoms/drugs/tests/cases/treatments）
2. 加载 2 个关系 YAML（relations.yaml + treatment_rules.yaml）
3. 加载 tags.yaml（预置标签库）
4. 收集实体 frontmatter 中未注册的自定义标签
5. 按顺序生成 SQL：
   ├─ 标签 INSERT（tags 表）
   ├─ 疾病 INSERT（diseases 表）
   ├─ 疾病-标签 INSERT（entity_tags 表）
   ├─ 症状 INSERT + 症状-标签
   ├─ 疾病-症状 INSERT（disease_symptom 表）
   ├─ DDX INSERT（disease_ddx 表）
   ├─ 药物 INSERT + 药物-标签
   ├─ 检查 INSERT
   ├─ 疾病-治疗 INSERT（disease_treatment 表）
   ├─ 疾病-诊断 INSERT（disease_diagnostic 表）
   ├─ 治疗 INSERT + 治疗-标签
   ├─ 疾病-治疗方案 INSERT（disease_treatment_map 表）
   ├─ 病例 INSERT
   └─ 病例-疾病 INSERT（case_disease 表，当前硬编码于脚本中）
```

#### 已知技术债
- `case_disease_map` 当前硬编码在 `gen_from_yaml.py` 中（行 541-557），未迁移到 `relations.yaml`。这是 `WORKFLOW_REFORM.md` 计划迁移的对象，会被 `hardcode_check` 检查器报告。

### 6.2 `tools/populate_drug_fields.py`（药物字段补全）

- **职责**：批量回填药物 MD 文件中缺失的 `mechanism_of_action`、`pk_pd`、`adverse_mechanism` 三个字段，跳过已填充的文件。
- **依赖**：同目录下 `drug_fields_data.py`（含 `DRUG_DATA` 字典，独立数据模块）。
- **关键函数**：
  - `process_drug(filepath)`：单文件处理，对比现有 frontmatter，仅写入缺失字段，返回 `(drug_id, added_count, skipped_count)`
  - `main()`：遍历 `data/drugs/drug_*.md`，打印进度和统计
- **属于一次性数据补全工具**，修改后需重新运行 `gen_from_yaml.py` 才能反映到 SQL。

### 6.3 其他工具脚本

| 脚本 | 作用 |
|------|------|
| `tools/_build_data.py` | 数据构建辅助（私有） |
| `tools/_gen_populate.py` | 内部生成脚本 |
| `tools/_make_script.py` | 内部脚本生成器 |
| `tools/_write_data.py` | 数据写入辅助 |
| `tools/_write_json.py` | JSON 写入辅助 |
| `tools/drug_fields.json` | 药物字段数据（JSON 格式） |
| `tools/build-with-subws.ps1` | PowerShell 构建脚本（带子工作区） |

---

## 7. 静态检查框架

### 7.1 框架定位与设计来源

- **设计来源**：`docs/WORKFLOW_REFORM.md`（工作流改革方案）
- **规则规格**：`_knowledge/rules/static_analysis_rules.md`（6 条跨项目通用规则）
- **目标**：消灭"每步无法完全按计划推进"的系统性问题，通过脚本化检查保证跨文件一致性。
- **三个检查时机**：
  1. 提交前自动（pre-commit hook，运行 `--quick`）
  2. 阶段交付前手动（运行 `--full`）
  3. 数据变更后手动 + 脚本

### 7.2 主入口 `_tools/dev/check.py`

- **职责**：跨项目通用检查框架主入口，通过 `importlib` 动态加载 `checkers/` 目录下的检查器模块。
- **命令行参数**：

| 参数 | 作用 |
|------|------|
| `--config PATH` / `-c` | 配置文件路径（.json 或 .toml） |
| `--quick` | 仅运行 route_check + api_check（<5s） |
| `--full` | 运行全部 6 个检查器 |
| `--init` | 生成默认 `check_config.json` |
| `--schema` | 输出 `check_config_schema.json` 内容 |
| `--list` | 列出所有可用检查器 |
| `--install-hook` | 安装 Git pre-commit hook（跨平台，生成 .py + .sh + .bat） |
| `--no-color` | 禁用 ANSI 颜色 |
| 位置参数 `check_name` | 仅运行指定检查器 |

- **核心数据结构**：
  - `CheckResult` dataclass：`passed` / `confirmed`（确定问题）/ `review_needed`（待人工）/ `skipped`
  - `Config` dataclass：`global_settings` / `checks` / `raw`，提供 `ignore_patterns` 属性
- **退出码**：`EXIT_OK=0` / `EXIT_CONFIRMED=1` / `EXIT_REVIEW=2`

### 7.3 检查器 `checkers/`

6 个检查器，每个对应一条规则，统一约定：文件头 docstring 说明规则编号/配置项/误报场景；从 `checkers` 模块导入 `CheckResult`；导出 `run(check_config) -> CheckResult` 函数。

| 检查器 | 规则编号 | 作用 | 关键正则/函数 |
|--------|----------|------|---------------|
| `route_check.py` | LINK-DEAD | 路由死链检测：扫描 `.vue`/`.tsx` 中的 `router-link`、`router.push` 引用，与路由注册文件对比 | `_VUE_ROUTE_PATH`、`_REACT_ROUTE_PATH`；动态路径归一化 `${id}` → `:id` |
| `api_check.py` | CMD-MISMATCH | 前后端命令一致性：扫描 `invoke('xxx')`/`fetch('/api/xxx')`，对比后端注册表 | `_TAURI_COMMAND_DEF`/`_TAURI_COMMAND_NAME`；snake_case ↔ camelCase 转换 |
| `type_check.py` | TYPE-DRIFT | 跨层类型一致性：对比前端 TS interface 与后端 Rust struct 字段（**默认关闭，高误报风险**） | `_TS_INTERFACE`/`_PY_CLASS_FIELD`/`_RUST_STRUCT_FIELD`；`_extract_ts_interface_fields` |
| `empty_check.py` | CMD-STUB | 空实现检测：检查注册函数体是否为空/pass/return None/todo!() | `_EMPTY_BODY_PASS`/`_EMPTY_BODY_PANIC`/`_TODO_ONLY`；`_read_function_body`（追踪括号深度） |
| `hardcode_check.py` | DATA-HARDCODE | 硬编码数据检测：扫描 `.py`/`.js`/`.ts` 中大型 dict/list 字面量 | `_PY_DICT_START`/`_JS_OBJ_START`；`_count_dict_entries`；threshold 默认 5 |
| `dead_code_check.py` | DB-DEAD | 废弃代码检测：从 SQL schema 提取所有表名，对比应用代码中引用情况 | `_SQL_CREATE_TABLE`/`_SQL_ALTER_TABLE`；`extract_sql_tables`；未引用表标记为 CONFIRMED |

**置信度分层**：CONFIRMED（确定）→ HIGH（高度疑似）→ REVIEW_NEEDED（需人工）→ INFO（参考）

### 7.4 配置文件

#### `check_config.json`（项目专用配置）
- **启用 5 个检查器**（type_check 默认关闭）
- **全局配置**：`project=vet-knowledge`、`base_dir=.`、`ignore=["node_modules/**", "dist/**", "target/**", ".git/**"]`
- **各检查器配置**：
  - `route_check`：Vue 框架，扫描 `src/views`、`src/components`
  - `api_check`：Tauri 类型，后端入口 `src-tauri/src/lib.rs`
  - `type_check`（disabled）：前端 `src/types`，后端 `src-tauri/src/db/models.rs`
  - `empty_check`：注册文件 `lib.rs`，实现目录 `commands`
  - `hardcode_check`：扫描 `tools/*.py`，threshold=5，min_entries=3
  - `dead_code_check`：schema=`schema.sql`，代码目录 `src-tauri/src`

#### `_tools/dev/check_config_schema.json`
- **作用**：JSON Schema (draft-07)，用于 IDE 自动补全和配置校验。
- **结构**：顶层 `project`/`base_dir`/`ignore` + 6 个检查器 section，每个 section 含 `enabled` + 特定字段 + `ignore` glob 数组。

### 7.5 规则定义

#### `_knowledge/rules/static_analysis_rules.md`
- **职责**：从 vet-knowledge 实际问题抽象出 6 条独立可复用规则的设计规格文档。
- **6 条规则**：见 [7.3](#73-检查器-checkers) 表格。
- **每条规则结构**：检查目标 / 适用项目类型 / 输入文件 / 检查逻辑（伪代码）/ 输出格式示例 / 退出码 / 误报场景 / 配置参数表。
- **实际发现案例**：`case_disease_map` 硬编码、FTS5 表废弃、Treatment 字段名不一致（procedure_text vs procedure）等。

---

## 8. 依赖关系

### 8.1 前端依赖（package.json）

| 依赖 | 版本 | 类型 | 用途 |
|------|------|------|------|
| `vue` | ^3.5.0 | prod | 前端框架 |
| `vue-router` | ^4.4.0 | prod | 路由 |
| `pinia` | ^2.2.0 | prod | 状态管理 |
| `@tauri-apps/api` | ^2.1.0 | prod | Tauri IPC（`invoke`） |
| `@tauri-apps/cli` | ^2.11.0 | dev | Tauri CLI |
| `@vitejs/plugin-vue` | ^5.2.0 | dev | Vite Vue 插件 |
| `vite` | ^6.0.0 | dev | 构建工具 |
| `typescript` | ^5.6.0 | dev | 类型系统 |
| `vue-tsc` | ^2.2.0 | dev | Vue TS 类型检查 |

**npm scripts**：
- `dev`：`vite`（仅前端开发）
- `build`：`vue-tsc --noEmit && vite build`
- `tauri:dev`：`tauri dev`（前端 + Rust 后端）
- `tauri:build`：`tauri build`（生产构建）

### 8.2 后端依赖（Cargo.toml）

| 依赖 | 版本 | 用途 |
|------|------|------|
| `tauri` | 2 | Tauri 核心 |
| `tauri-plugin-shell` | 2 | Shell 插件 |
| `tauri-plugin-fs` | 2 | 文件系统插件 |
| `serde` | 1 (derive) | 序列化 |
| `serde_json` | 1 | JSON 处理 |
| `sqlx` | 0.8 (sqlite, tokio, migrate) | SQLite 异步驱动 |
| `tokio` | 1 (full) | 异步运行时 |
| `anyhow` | 1 | 错误处理 |
| `thiserror` | 1 | 自定义错误类型 |
| `once_cell` | 1 | 全局静态变量 |
| `chrono` | 0.4 (serde) | 时间处理 |
| `uuid` | 1 (v4) | UUID 生成 |
| `tauri-build` | 2 (build-dep) | Tauri 构建脚本 |

**Cargo features**：`default = []`（无默认特性）

### 8.3 模块间依赖图

```
┌─────────────────────────────────────────────────────────────┐
│                       前端 (Vue/TS)                          │
│  views/* ──invoke──► Tauri IPC ──► commands/*                │
│      ↑                                          │            │
│      │                                          │            │
│  components/  ◄──── router/index.ts ◄── main.ts │            │
│      │                                          │            │
│      └──── types/index.ts ◄──镜像──► db/models.rs            │
└─────────────────────────────────────────────────────────────┘
                                                  │
┌─────────────────────────────────────────────────┼────────────┘
│                                                ▼
│                    后端 (Rust)
│  lib.rs ──► commands/* ──► db/mod.rs ──► SQLite
│                │              │
│                │              ├── db/models.rs (struct)
│                │              ├── schema.sql (建表)
│                │              └── 001_initial.sql (种子)
│                │
│                ├── engine.rs (纯函数)
│                │     ▲
│                │     └── diagnose.rs (编排)
│                │
│                ├── knowledge.rs (CRUD)
│                ├── treatments.rs (治疗+标签)
│                ├── flashcards.rs (SM-2)
│                └── search.rs (LIKE)
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                    数据管线 (Python)
│  data/*.md + data/*.yaml ──► tools/gen_from_yaml.py ──► 001_initial.sql
│                                       │
│                                       └──► 被 db/mod.rs 加载
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                  静态检查框架 (Python)
│  _tools/dev/check.py ──► checkers/*.py (6 个规则)
│         │                        │
│         │                        ├── 检查前端 router ↔ views
│         │                        ├── 检查前端 invoke ↔ 后端 lib.rs
│         │                        ├── 检查 TS interface ↔ Rust struct
│         │                        ├── 检查命令实现非空
│         │                        ├── 检查 Python 硬编码数据
│         │                        └── 检查 SQL schema 表被引用
│         │
│         ├── check_config.json ◄── check_config_schema.json
│         └── .checkignore (可选 glob 忽略)
└─────────────────────────────────────────────────────────────┘
```

---

## 9. 项目运行方式

### 9.1 环境要求

| 项目 | 要求 |
|------|------|
| 操作系统 | Windows 10/11 (x64) |
| 内存 | ≥ 4GB RAM |
| WebView2 | Windows 11 自带；Windows 10 需安装 [WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) |
| Rust | 1.70+（仅开发者构建需要） |
| Node.js | 18+（仅开发者构建需要） |
| pnpm | 推荐的包管理器（也可用 npm） |
| Python | 3.x（仅数据生成需要） |

### 9.2 普通用户：安装运行

1. 前往 [Releases](https://github.com/ningjie333/vet-knowledge/releases) 下载最新 `.exe` 安装包
2. 运行安装程序（NSIS 安装器）
3. 启动应用

**数据库位置**：`%APPDATA%\com.vetknowledge.app\vet_knowledge.db`（Tauri app_data_dir）

**故障排查**：若启动失败，查看 `%APPDATA%\com.vetknowledge.app\seed_import_error.log`，按提示删除该目录后重启。

### 9.3 开发者：从源码构建

```bash
# 1. 克隆仓库
git clone https://github.com/ningjie333/vet-knowledge.git
cd vet-knowledge

# 2. 安装前端依赖
pnpm install
# 或 npm install

# 3. 开发模式（前端热重载 + Rust 后端）
pnpm tauri dev
# 或 npm run tauri:dev

# 4. 仅前端开发（不启动 Rust 后端，invoke 调用会失败）
pnpm dev
# 或 npm run dev

# 5. Rust 编译检查
cd src-tauri && cargo check

# 6. 生产构建
pnpm tauri build
# 或 npm run tauri:build
```

**构建产物位置**：`src-tauri/target/release/bundle/nsis/`

**Windows 一键启动**：双击 `启动兽医知识库.bat` 或 `启动知识库.bat`。

### 9.4 数据变更工作流

> 详见 `CLAUDE.md` 和 `docs/WORKFLOW_REFORM.md`

**新增疾病流程（示例）**：

1. 在 `data/diseases/` 下新建 `dis_xxx.md`（参考现有文件格式，包含 `tags:` 字段）
2. 在 `data/relations.yaml` 的 `disease_symptoms` 下添加症状关联
3. 在 `data/treatment_rules.yaml` 的 `disease_treatment` 下添加药物关联
4. 在 `data/treatment_rules.yaml` 的 `disease_diagnostic` 下添加诊断关联
5. 在 `data/treatment_rules.yaml` 的 `disease_treatment_map` 下添加治疗方案关联
6. 如需新症状/药物/检查/标签，先在对应目录新建 `.md` 文件或 `tags.yaml`
7. 运行 `python tools/gen_from_yaml.py` 重新生成 SQL
8. 递增 `src-tauri/src/db/mod.rs` 中的 `SEED_DATA_VERSION`（当前 18）
9. `cargo check` 验证编译

**关键规则**：
- 实体数据用 Markdown + Frontmatter
- 关系数据用 YAML
- 修改数据后必须重新生成 SQL 并递增版本号
- 数据库版本号定义在 `SEED_DATA_VERSION`，应用启动时自动检测并重导

### 9.5 开发检查工作流

> 详见 `docs/WORKFLOW_REFORM.md`

**pre-commit hook（已安装）**：
```bash
python _tools/dev/check.py --config check_config.json --quick
```
如未安装 hook：
```bash
python _tools/dev/check.py --install-hook
```

**阶段交付前全套检查**：
```bash
python _tools/dev/check.py --config check_config.json --full
```

**新增页面时**：遵循 `_knowledge/workflows/new_page.md` SOP（需在 `router/index.ts` 添加路由 + 在 `Sidebar.vue` 添加菜单项 + 在后端注册 invoke 命令）。

**数据变更时**：遵循 `_knowledge/workflows/data_change.md` SOP（运行 `gen_from_yaml.py` + 递增 `SEED_DATA_VERSION` + `cargo check`）。

---

## 10. 关键设计说明

### 10.1 诊断推理引擎算法

**位置**：`src-tauri/src/engine.rs::infer()`

**输入**：
- `DiagnosisInput`：用户输入的症状列表 + 物种 + 年龄 + 品种
- `disease_list: Vec<(disease_id, disease_name)>`：候选疾病列表（已按 species 过滤）
- `all_disease_symptoms: HashMap<disease_id, Vec<(sym_name, frequency, is_pathognomonic)>>`
- `diagnostics: HashMap<disease_id, Vec<(test_id, purpose)>>`

**算法步骤**：

1. **构建输入症状集合** `input_set`
2. **遍历每个候选疾病**：
   - 取该疾病的症状列表 `sym_list`
   - 对每个症状计算基础权重：
     - `common` → 1.0
     - `uncommon` → 0.6
     - `rare` → 0.3
     - 其他 → 0.5
   - 累加 `total_weight`
   - 如果症状在 `input_set` 中：
     - 若 `is_pathognomonic=1`，权重 × 1.5（核心症状加成），并计入 `pathognomonic_matched`
     - 加入 `matched` 列表
   - 否则若 `base_weight >= 1.0`（即 common 症状），加入 `missing` 列表
3. **过滤无匹配疾病**：`matched.is_empty()` 则跳过
4. **计算双覆盖度**：
   - `disease_coverage = weighted_score / total_weight`（疾病覆盖度：用户输入覆盖了多少疾病权重）
   - `input_coverage = matched.len() / input_set.len()`（输入覆盖度：用户输入中有多少匹配该疾病）
5. **综合评分**：`score = disease_coverage * input_coverage`
6. **核心症状加成**：`score += min(0.05 * pathognomonic_matched, 0.15)`，上限 1.0
7. **阈值过滤**：`score < 0.25` 或 `input_coverage < 0.3` 则丢弃
8. **附加诊断检查**：从 `diagnostics` map 取该疾病的推荐检查
9. **排序截断**：按 `match_score` 降序，取前 10

**输出**：`Vec<DiagnosisCandidate>`，每项含 `match_score`（保留 2 位小数）、`input_coverage`、`matched_symptoms`、`missing_key_symptoms`、`suggested_tests`。

**设计亮点**：
- 纯函数无副作用，便于单元测试
- 双覆盖度机制避免"输入少量症状匹配上症状少的疾病"导致虚高分
- 核心症状 1.5× 加成 + 0.05~0.15 加成，体现临床决策权重
- 阈值过滤减少低质量候选

### 10.2 SM-2 间隔重复算法

**位置**：`src-tauri/src/commands/flashcards.rs::review_flashcard()`

**输入**：`card_id`、`quality`（0-5）

**算法步骤**：

1. **取上次复习记录**：从 `flashcard_reviews` 表取 `interval_days` 和 `ease_factor`
2. **计算新 EF（难度系数）**：
   ```
   EF' = max(1.3, EF + (0.1 - (5-q) * (0.08 + (5-q) * 0.02)))
   ```
   - q=5：EF 增加 0.1
   - q=4：EF 不变
   - q=3：EF 减少 0.14
   - q<3：EF 大幅减少
   - 下限 1.3
3. **计算新间隔**：
   - 若 `quality < 3`（忘记）：interval = 1 天，difficulty = 0.8
   - 若 `old_interval < 1.5`：interval = 6 天，difficulty = 0.5
   - 否则：interval = old_interval × new_ef，difficulty = 0.3
4. **首次复习特殊处理**：
   - q < 3：interval = 1 天
   - q = 3：interval = 1 天
   - q > 3：interval = 6 天
   - 初始 EF = 2.5 + 上述公式
5. **计算下次复习时间**：`next_review = now + interval_days × 86400000ms`
6. **更新数据库**：
   - `UPDATE flashcards SET difficulty = ?`
   - `INSERT INTO flashcard_reviews (card_id, quality, interval_days, ease_factor, next_review)`

**前端评分映射**（`FlashcardStudy.vue`）：
- "完全忘记" → quality=0
- "勉强想起" → quality=2
- "想起来了" → quality=3
- "非常轻松" → quality=5

**统计指标**（`get_review_stats`）：
- `total_cards`：闪卡总数
- `due_today`：今日到期（next_review 为空或 <= now）
- `reviewed_today`：今日已复习（reviewed_at = today）
- `mastered`：已掌握（ease_factor >= 2.3 且 interval_days >= 21）

### 10.3 标签系统

**设计**：多态关联，一张 `entity_tags` 表支持 5 种实体类型（disease/symptom/drug/treatment/case）。

**5 个标签分组**：

| 分组 | 数量 | 用途 | 示例 |
|------|------|------|------|
| `body_system` | 11 | 解剖系统 | 呼吸系统、消化系统、心血管系统 |
| `mechanism` | 11 | 病理机制 | 炎症免疫、代谢、感染 |
| `emergency` | 4 | 急诊四级 | 🔴 red（立即复苏）、🟠 orange（15min）、🟡 yellow（60min）、🟢 green（常规） |
| `damnit_v` | 7 | DAMNIT-V 诊断模型 | 退行性、畸形/先天、代谢性、肿瘤性、感染、中毒、血管 |
| `species` | 5 | 物种 | 犬、猫、马、牛、异宠 |
| `custom` | 动态 | 用户自定义 | 由 `gen_from_yaml.py` 的 `collect_tags` 自动收集 |

**急诊标签字段**（仅 emergency 分组）：
- `emergency_level`：red/orange/yellow/green
- `clinical_action`：临床行动指导
- `textbook_logic`：教科书逻辑
- `typical_scenario`：典型场景

**前端展示**：
- `DiseaseDetail.vue`：急诊等级徽章（颜色编码）+ 三组标签云（body_system/mechanism/damnit_v）
- `SymptomExplorer.vue`：标签芯片按颜色区分
- `DrugHandbook.vue`：机制标签云

### 10.4 数据库迁移与种子版本管理

**双轨版本管理**：

| 表 | 用途 | 字段 |
|----|------|------|
| `schema_migrations` | 追踪结构迁移 | version, description, applied_at |
| `app_meta` | 追踪种子数据版本 | key='seed_data_version', value=整数 |

**Schema 迁移流程**（`apply_migrations`）：
1. 确保 `schema_migrations` 表存在
2. 取 `MAX(version)` 作为 current
3. 若 `current == 0`（全新库）：直接写入 v1-v9 基线，跳过历史回放（schema.sql 已是最新结构）
4. 否则：按版本号顺序回放 v1-v9 迁移块，每个用 `if current < N` 守卫
5. 每个迁移块用 `IF NOT EXISTS` 或 `INSERT OR IGNORE` 保证幂等

**种子数据导入流程**（`import_seed_data`）：
1. 从 `app_meta` 取 `seed_data_version`
2. 若版本不存在或低于 `SEED_DATA_VERSION`：触发重导
3. 事务内：
   - DELETE 13 张数据表（容忍 "no such table" 错误）
   - 逐条执行 `001_initial.sql` 的 INSERT（失败立即返回错误，事务回滚）
   - 重建 FTS 索引
   - 写入新 `seed_data_version`（最后一步，保证仅成功时才记录）
4. 提交事务

**关键设计**：种子导入在事务内，失败自动回滚，`seed_data_version` 不会被写入，下次启动会自动重试。

### 10.5 SQL 安全分割策略

**问题**：SQL 脚本含 `--` 注释和字符串内的分号，简单 `split(';')` 会出错。

**解决方案**（`db/mod.rs`）：

1. **`strip_sql_comments(script)`**：先剥掉所有 `--` 单行注释（行首 `--` 整行删除），避免注释语句被分割器跳过。
2. **`split_sql_statements(script)`**：状态机式分割：
   - 跟踪单引号/双引号状态
   - 仅在引号外部分割分号
   - 处理 `''` 和 `""` 转义（跳过下一个引号）
   - 处理尾部无分号的最后一段

**单元测试覆盖**：
- `test_split_sql_basic`：基础分割
- `test_split_sql_semicolon_in_string`：引号内分号不分割
- `test_split_sql_escaped_quotes`：转义引号处理
- `test_split_sql_empty_and_comments`：注释和空行

---

## 11. 附录

### 11.1 Tauri 命令清单

共 27 个命令，全部注册于 `src-tauri/src/lib.rs` 的 `invoke_handler!` 宏：

| 模块 | 命令 | 参数 | 返回值 |
|------|------|------|--------|
| **knowledge** | `get_diseases` | species?, category? | `Vec<Disease>` |
| | `get_disease_by_id` | id | `Option<Disease>` |
| | `get_symptoms` | — | `Vec<Symptom>` |
| | `get_symptom_by_id` | id | `Option<Symptom>` |
| | `get_diseases_by_symptom` | symptom_id, species? | `Vec<DiseaseWithSymptom>` |
| | `get_drugs` | drug_class? | `Vec<Drug>` |
| | `get_drug_by_id` | id | `Option<Drug>` |
| | `get_tests` | — | `Vec<DiagnosticTest>` |
| | `get_test_by_id` | id | `Option<DiagnosticTest>` |
| | `get_cases` | species?, difficulty? | `Vec<Case>` |
| | `get_case_by_id` | id | `Option<Case>` |
| | `get_case_diseases` | case_id | `Vec<Disease>` |
| | `get_disease_ddx` | disease_id | `Vec<(Disease, String)>` |
| | `get_disease_symptoms` | disease_id | `Vec<(Symptom, String, String, i64)>` |
| | `get_disease_treatments` | disease_id | `Vec<(Drug, String, String, String)>` |
| | `get_disease_diagnostics` | disease_id | `Vec<(DiagnosticTest, String, String, String, String)>` |
| | `get_disease_compare` | disease_ids: Vec<String> | `Vec<DiseaseCompareView>` |
| **treatments** | `get_treatments` | therapy_type? | `Vec<Treatment>` |
| | `get_treatment_by_id` | id | `Option<Treatment>` |
| | `get_disease_treatment_map` | disease_id | `Vec<(Treatment, String, String, String)>` |
| | `get_tags` | tag_group? | `Vec<Tag>` |
| | `get_entity_tags` | entity_type, entity_id | `Vec<Tag>` |
| | `get_entities_by_tag` | tag_id, entity_type | `Vec<String>` |
| | `add_entity_tag` | entity_type, entity_id, tag_id | `()` |
| | `remove_entity_tag` | entity_type, entity_id, tag_id | `()` |
| **search** | `full_text_search` | query, limit? | `Vec<SearchResult>` |
| **diagnose** | `infer_diagnosis` | symptoms, species, age?, breed? | `Vec<DiagnosisCandidate>` |
| **flashcards** | `get_due_flashcards` | limit? | `Vec<Flashcard>` |
| | `get_all_flashcards` | card_type? | `Vec<Flashcard>` |
| | `generate_flashcards_from_knowledge` | card_type | `i64`（生成数量） |
| | `create_flashcard` | front, back | `String`（新 ID） |
| | `delete_flashcard` | id | `()` |
| | `review_flashcard` | card_id, quality | `()` |
| | `get_review_stats` | — | `ReviewStats` |

### 11.2 路由清单

定义于 `src/router/index.ts`，共 12 条路由，全部懒加载：

| 路径 | 名称 | 组件 | 说明 |
|------|------|------|------|
| `/` | home | `knowledge/Home.vue` | 首页仪表盘 |
| `/diseases` | diseases | `knowledge/DiseaseList.vue` | 疾病列表 |
| `/diseases/:id` | disease-detail | `knowledge/DiseaseDetail.vue` | 疾病详情 |
| `/symptom-explorer` | symptom-explorer | `knowledge/SymptomExplorer.vue` | 症状检索 |
| `/drugs` | drugs | `knowledge/DrugHandbook.vue` | 药物手册 |
| `/diagnose` | diagnose | `knowledge/SymptomChecker.vue` | 症状推理 |
| `/graph` | graph | `graph/KnowledgeGraph.vue` | 知识图谱（占位） |
| `/cases` | cases | `learning/CaseLibrary.vue` | 病例库 |
| `/cases/:id` | case-detail | `learning/CaseDetail.vue` | 病例详情 |
| `/cases/:id/study` | case-study | `learning/CaseStudy.vue` | 病例推理训练（占位） |
| `/flashcards` | flashcards | `learning/FlashcardStudy.vue` | 闪卡复习 |
| `/compare` | compare | `knowledge/DiseaseCompare.vue` | 疾病对比 |
| `/game` | game | `game/GameHome.vue` | 诊断游戏（占位） |

### 11.3 数据库表清单

完整 schema 定义于 `src-tauri/data/seed/schema.sql`：

| 类别 | 表名 | 用途 |
|------|------|------|
| **实体** | `diseases` | 疾病 |
| | `symptoms` | 症状 |
| | `drugs` | 药物 |
| | `diagnostic_tests` | 诊断检查 |
| | `cases` | 病例 |
| | `treatments` | 治疗方案 |
| **关系** | `disease_symptom` | 疾病-症状 |
| | `disease_ddx` | 鉴别诊断 |
| | `disease_treatment` | 疾病-药物 |
| | `disease_diagnostic` | 疾病-检查 |
| | `disease_treatment_map` | 疾病-治疗方案 |
| | `case_disease` | 病例-疾病 |
| **标签** | `tags` | 预置标签库 |
| | `entity_tags` | 多态关联 |
| **学习** | `flashcards` | 闪卡 |
| | `flashcard_reviews` | 复习记录 |
| | `learning_progress` | 学习进度（预留） |
| **全文搜索** | `diseases_fts` | 疾病 FTS5 |
| | `symptoms_fts` | 症状 FTS5 |
| | `drugs_fts` | 药物 FTS5 |
| | `cases_fts` | 病例 FTS5 |
| **元数据** | `schema_migrations` | 迁移版本 |
| | `app_meta` | 应用元数据 |

### 11.4 文件路径速查

#### 后端 Rust
- 入口：`src-tauri/src/main.rs` → `src-tauri/src/lib.rs`
- 推理引擎：`src-tauri/src/engine.rs`
- DB 初始化：`src-tauri/src/db/mod.rs`
- 数据模型：`src-tauri/src/db/models.rs`
- 命令实现：`src-tauri/src/commands/{knowledge,treatments,diagnose,flashcards,search,import_export}.rs`
- SQL Schema：`src-tauri/data/seed/schema.sql`
- 种子数据：`src-tauri/data/seed/001_initial.sql`
- Tauri 配置：`src-tauri/tauri.conf.json`
- Rust 依赖：`src-tauri/Cargo.toml`

#### 前端 Vue/TS
- 入口：`index.html` → `src/main.ts` → `src/App.vue`
- 路由：`src/router/index.ts`
- 类型：`src/types/index.ts`
- 布局组件：`src/components/{Sidebar,TopBar}.vue`
- 知识库页面：`src/views/knowledge/*.vue`
- 学习页面：`src/views/learning/*.vue`
- 全局样式：`src/assets/main.css` + `src/App.vue` 的 `<style>`
- Vite 配置：`vite.config.ts`
- TS 配置：`tsconfig.json`
- 前端依赖：`package.json`

#### 数据源
- 疾病：`data/diseases/dis_*.md`
- 症状：`data/symptoms/sym_*.md`
- 药物：`data/drugs/drug_*.md`
- 检查：`data/tests/test_*.md`
- 病例：`data/cases/case_*.md`
- 治疗：`data/treatments/trt_*.md`
- 关系：`data/relations.yaml`、`data/treatment_rules.yaml`
- 标签：`data/tags.yaml`

#### 工具脚本
- 数据生成：`tools/gen_from_yaml.py`
- 药物补全：`tools/populate_drug_fields.py`
- 检查框架：`_tools/dev/check.py`
- 检查器：`_tools/dev/checkers/*.py`
- 检查配置：`check_config.json`
- 配置 Schema：`_tools/dev/check_config_schema.json`

#### 文档
- 项目说明：`README.md`
- Claude 指南：`CLAUDE.md`
- 工作流改革：`docs/WORKFLOW_REFORM.md`
- 静态分析规则：`_knowledge/rules/static_analysis_rules.md`
- 病例库规划：`data/开发文档_病例库规划.md`

---

*本文档由 Code Wiki 生成器基于仓库当前状态自动分析整理。如需更新，请重新运行分析流程。*
