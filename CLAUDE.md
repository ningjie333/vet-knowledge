# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

**兽医知识库** — 基于 Tauri 2.0 的桌面端兽医学习平台。Rust 后端 + Vue 3 前端，SQLite 数据库，Windows 单机运行，无服务端。

数据规模（`src-tauri/data/seed/001_initial.sql` 的 INSERT 行数）：85 疾病 / 44 症状 / 80 药物 / 36 诊断检查 / 15 病例 / 6 治疗方案 / 39 预置标签。

文档分工：`README.md`（用户视角）、`CODE_WIKI.md`（架构，每条论断带出处）、本文件（改动前必看的仓库约定）、`CONTRIBUTING.md`（提交与发布）、`docs/REMEDIATION.md`（整改台账）。

## 构建与运行

```bash
# 启动开发（前端热重载 + Rust 后端）
npm run tauri:dev

# 仅前端开发（此时所有 invoke 调用都会失败）
npm run dev

# Rust 编译检查 / 测试
cd src-tauri && cargo check
cd src-tauri && cargo test

# 前端规范与类型
npm run lint
npx vue-tsc --noEmit

# 生产构建：必须走 Tauri CLI
npx tauri build                # 或 npm run tauri:build
npx tauri build --no-bundle    # 只要 exe，跳过 NSIS 下载
```

两条本机前提：

- Rust 工具链若按非默认方式安装，每条 cargo 命令前需要 `export PATH="$HOME/.cargo/bin:$PATH"`（rustup 装到了 `~/.cargo`）。
- **不要**用 `cargo build --release` 的产物验证界面：`#[cfg(dev)]` 由 Tauri CLI 注入，裸 cargo 产出的 exe 会去连 `devUrl`（http://localhost:1420）并显示 WebView2 错误页。

## 数据管线（重要）

**统一数据源：`data/knowledge.json`**。通过脚本生成 SQL 种子数据：

```bash
# 修改数据后必须重新生成
python tools/gen_from_json.py
```

生成器输出到 `src-tauri/data/seed/001_initial.sql`。数据版本号定义在 `src-tauri/src/db/mod.rs` 的 `const SEED_DATA_VERSION: i64`（当前 **22**），数据变更后必须递增，否则启动时不会重导（判定见 `import_seed_data()`：存的值 **小于**常量才重导）。

### JSON 数据结构（现行格式）

`data/knowledge.json` 是单一数据源，关系内嵌在疾病对象里，独立实体用顶层数组（共七个：`diseases`/`symptoms`/`drugs`/`tests`/`cases`/`treatments`/`drug_interactions`）：

```json
{
  "diseases": [
    {
      "id": "dis_001",
      "name_zh": "肺炎",
      "species": ["犬", "猫"],
      "category": ["呼吸系统"],
      "tags": ["body_respiratory", "mech_infectious", "em_yellow"],
      "symptoms": [{"symptom": "sym_001", "frequency": "common", "stage": "全程", "is_pathognomonic": false}],
      "drugs": [{"drug": "drug_001", "line": "first", "species": "犬", "notes": "..."}],
      "diagnostics": [{"test": "test_006", "purpose": "confirming", "evidence_level": "supportive", "species": "犬", "expected_result": "..."}],
      "treatments": [...],
      "ddx": [...]
    }
  ],
  "drug_interactions": [
    {"drug_a": "drug_003", "drug_b": "drug_012", "severity": "major", "description": "..."}
  ]
}
```

字段名与形状以现有条目为准（`python -c "import json;print(json.load(open('data/knowledge.json',encoding='utf-8'))['diseases'][0].keys())"`）。

**加一种病 = 往 `diseases` 数组加 1 个对象 + 跑 `python tools/gen_from_json.py` + 递增 `SEED_DATA_VERSION`。**

### 旧格式（已废弃，生成器不再读取）

| 目录/文件 | 状态 | 说明 |
| --- | --- | --- |
| `data/diseases/*.md` | DEPRECATED | 目录内有 `DEPRECATED.md` 标记，内容已合并进 knowledge.json |
| `data/symptoms/*.md` | DEPRECATED | 同上 |
| `data/drugs/*.md` | DEPRECATED | 同上 |
| `data/tests/*.md` | DEPRECATED | 同上 |
| `data/cases/*.md` | DEPRECATED | 同上 |
| `data/treatments/*.md` | DEPRECATED | 同上 |
| `data/relations.yaml` | DEPRECATED | 疾病-症状/鉴别诊断关系已内嵌进 knowledge.json |
| `data/treatment_rules.yaml` | DEPRECATED | 疾病-药物/诊断/治疗方案关系已内嵌进 knowledge.json |
| `data/tags.yaml` | **仍在用** | 39 个预置标签，`tools/gen_from_json.py` 仍读取它 |

`tools/gen_from_json.py` **只有 JSON 模式**（MD+YAML 兼容模式已在提交 `1afa989` 删除）。引用了 `data/tags.yaml` 里没有的标签时，生成器直接报错并要求补条目或改引用，不会退化成"中文名=英文 id"的脏标签。

### 标签系统（Tagging）

标签通过 `entity_tags` 多态关联表实现（主键 `entity_type + entity_id + tag_id`），支持一个实体拥有多个不同分组的标签。种子数据里共 **39 个标签 / 584 条 entity_tags**。

**标签分组（tag_group）**：

| 分组 | 数量 | 说明 | 示例 |
|------|------|------|------|
| `body_system` | 11 | 解剖系统 | 呼吸系统、消化系统、心血管系统… |
| `mechanism` | 12 | 病理机制 | 炎症免疫、代谢、血管循环、梗阻、感染… |
| `emergency` | 4 | 急诊四级决策 | 🔴红（立即心肺复苏）🟠橙（15 分钟内介入）🟡黄（60 分钟内介入）🟢绿（常规） |
| `damnit_v` | 7 | DAMNIT-V 诊断模型 | 退行性、畸形/先天、代谢性、肿瘤性… |
| `species` | 5 | 物种 | 犬、猫、马、牛、异宠 |

数量与取值以 `src-tauri/data/seed/001_initial.sql` 的 `INSERT INTO tags` 行为准。`body_system`/`mechanism` 的数目与 `data/tags.yaml` 的词表一致，改词表要同步看这里。

**急诊分级标签字段**（`tags` 表）：`emergency_level`（red/orange/yellow/green，CHECK 约束）、`clinical_action`、`textbook_logic`、`typical_scenario`。

**下发约定（踩过坑，勿改）**：命令层通过 `db/sql_builder.rs` 的 `tags_subquery()` 下发**标签 id**（`json_group_array(tag_id)`），前端拿 id 回查 `get_tags()` 的映射表取 name_zh/color/tag_group。下发中文名会让回查永远落空、标签区依旧空白。`entity_type` 取值必须三处同步：`schema.sql` 的 CHECK、`src-tauri/src/db/entity_types.rs` 的 `ENTITY_TYPES`、`tools/gen_from_json.py` 的 `ENTITY_TAG_SCOPES`。

### 新增疾病流程

1. 往 `data/knowledge.json` 的 `diseases` 数组加一个对象，关系直接内嵌：`symptoms[]` / `drugs[]` / `diagnostics[]` / `treatments[]` / `ddx[]`
2. 需要新症状/药物/检查/治疗方案时，同样往 knowledge.json 的对应顶层数组加条目（`data/*.md` 已废弃，生成器不读）
3. 标签只引用 `data/tags.yaml` 里已有的 id；需要新标签先补 `tags.yaml`
4. 运行 `python tools/gen_from_json.py`（会先做 entity_tags 覆盖校验，缺标签引用会直接报错）
5. 递增 `src-tauri/src/db/mod.rs` 的 `SEED_DATA_VERSION`
6. `cd src-tauri && cargo test`（`test_seed_sql_imports_with_foreign_keys_on` 会在外键开启下全量跑 schema + 种子，漏注册标签当场爆）

## 架构

### 后端（Rust）

```
src-tauri/src/
├── lib.rs          # Tauri 启动：插件、db::init、SidecarManager、46 条命令注册、退出时关 sidecar
├── engine.rs       # 诊断推理引擎（纯函数 infer，症状→疾病加权评分）
├── db/
│   ├── mod.rs          # 连接选项(PRAGMA)、建表、迁移 v1–v13、种子导入、错误日志
│   ├── models.rs       # DTO + row_from_* + parse_string_list/parse_tags_json
│   ├── sql_builder.rs  # SqlBuilder / placeholders / tags_subquery
│   └── entity_types.rs # ENTITY_TYPES 注册表（entity_type 唯一来源）
├── sidecar/
│   ├── root.rs         # virtual-vet 根目录解析（env VET_VET_ROOT → cwd/../virtual-vet → None）
│   ├── manager.rs      # 懒启动/重启限频/超时的 Python 子进程管理
│   └── protocol.rs     # JSON-RPC 2.0 报文与错误码
└── commands/
    ├── knowledge.rs    # 知识库只读查询（疾病/症状/药物/检查/病例 + 对比 + 药物相互作用）
    ├── treatments.rs   # 治疗方案 + 标签查询/绑定命令（add_entity_tag / remove_entity_tag 是唯一的写操作）
    ├── diagnose.rs     # infer_diagnosis（取数 + 调 engine::infer）
    ├── flashcards.rs   # 闪卡（SM-2：calculate_sm2 纯函数）
    ├── search.rs       # full_text_search（LIKE，不是 FTS5）
    └── game.rs         # 诊断游戏：10 条转发 sidecar + game_diagnosis_hint 本地推理
```

**没有 `import_export.rs`**——数据导入/导出功能不存在，命令也没注册，别在文档或代码里假装它有。

**数据库**：SQLite，`journal_mode=DELETE`（刻意不用 WAL：WAL 的 `.db-shm` 会被沙箱拦成 disk I/O error）、`synchronous=FULL`、外键开启、`cache_size=-64000`、`temp_store=MEMORY`。这些 PRAGMA **全部集中在 `db/mod.rs` 的 `sqlite_options()`**，因为 PRAGMA 按连接生效而池里有 2 条连接，用 `execute(&pool)` 设只会命中一条（详见该函数注释与 `CODE_WIKI.md` 4.1）。库文件位置按构建模式分支：debug 用仓库根 `data/vet_knowledge.db`，release 用 `%APPDATA%\com.vetknowledge.app\`。
迁移通过 `schema_migrations` 表追踪，种子数据版本通过 `app_meta` 表追踪。**当前迁移版本 v13，`SEED_DATA_VERSION` 22。**

**核心数据表**：
- 实体表：`diseases`, `symptoms`, `drugs`, `diagnostic_tests`, `cases`, `treatments`
- 关系表：`disease_symptom`, `disease_ddx`, `disease_treatment`, `disease_diagnostic`, `disease_treatment_map`, `case_disease`, `drug_interactions`
- 标签系统：`tags`（预置标签）, `entity_tags`（多态关联：entity_type + entity_id + tag_id）
- 学习：`flashcards`, `flashcard_reviews`；系统：`schema_migrations`, `app_meta`
- 已删除、不要再引用：4 张 FTS5 虚表（v10）、`learning_progress`（v11）

**诊断引擎**：`engine.rs` 的 `infer()`。频率权重 common=1.0 / uncommon=0.6 / rare=0.3（其他 0.5），标志性症状 `is_pathognomonic=1` 再乘 1.5；总分 = 疾病覆盖度 × 输入覆盖度，每命中一个标志性症状 +0.05（封顶 0.15）；`score<0.25` 或 `input_coverage<0.3` 丢弃；按分排序取前 10。`TestSuggestion.test_name` 目前直接等于 test_id，`distinguishing_features` 恒为空数组——界面也没有渲染这两项，别在文档里宣称"附带推荐检查与鉴别要点"。

**闪卡系统**：`commands/flashcards.rs` 的 `calculate_sm2()`（quality 0–5），EF 下限 1.3；quality<3 时间隔重置为 1 天。规则细节以函数文档注释和它的 24 个单测为准。

**治疗方案模块**：`treatments` 表存储完整治疗协议（含生理基础、预后评估），通过 `disease_treatment_map` 与疾病关联（line: first/second/adjunctive）。

### 前端（Vue 3）

```
src/
├── main.ts           # createApp + createPinia + router（Pinia 注册但无 store，见下）
├── App.vue           # 布局（侧边栏 + 主内容区）
├── components/
│   ├── Sidebar.vue   # 侧边导航（menuItems 十项）
│   └── TopBar.vue    # 顶部栏：全局搜索 + 动态版本号（getVersion()）
├── views/
│   ├── knowledge/
│   │   ├── Home.vue              # 统计卡片
│   │   ├── DiseaseList.vue       # 疾病列表（物种/分类筛选）
│   │   ├── DiseaseDetail.vue     # 详情：急诊徽章、系统/机制/DAMNIT-V 标签、治疗协议
│   │   ├── SymptomExplorer.vue   # 症状→疾病反向查找（注意：没有 SymptomList.vue）
│   │   ├── DrugHandbook.vue      # 药物手册：机制标签、作用机制 / PK-PD / 不良反应机制
│   │   ├── SymptomChecker.vue    # 症状推理
│   │   └── DiseaseCompare.vue    # 疾病对比
│   ├── learning/
│   │   ├── CaseLibrary.vue       # 病例列表 + 搜索
│   │   ├── CaseDetail.vue        # 病例分段 + 关联疾病
│   │   ├── CaseStudy.vue         # 占位（推理训练"开发中"）
│   │   └── FlashcardStudy.vue    # 闪卡复习
│   ├── graph/KnowledgeGraph.vue  # 占位页（15 行，"开发中"）
│   └── game/
│       ├── GameView.vue          # 实际诊断游戏界面（/game 路由指向它）
│       └── GameHome.vue          # 占位且未被任何文件引用
├── router/index.ts   # 13 条路由，组件全懒加载
├── types/index.ts    # 前端唯一类型来源：实体 Tag/Disease/Symptom/Drug/Treatment/DiagnosticTest/Case，
│                     # 推理 SearchResult/DiagnosisCandidate/TestSuggestion/DrugInteraction，
│                     # 关系元组 DiseaseSymptomRow 等 export type，游戏 Game*/Advance*
├── stores/           # 空目录
└── composables/      # 空目录
```

`src/stores/` 与 `src/composables/` 是**空目录**：`src/main.ts` 里 `app.use(createPinia())` 只是注册，全仓库没有任何 store。写文档或做技术选型时不要把 Pinia 算作现有数据层。

**前端 API 调用**：各视图直接 `invoke('command_name', { params })`，没有中间的 api/service 封装层。参数名用 **camelCase**，Tauri 会映射到 Rust 侧的 snake_case 形参（例：`invoke('get_drugs', { drugClass })` ↔ `get_drugs(drug_class: Option<String>)`）。

**前端标签展示**（都靠 `get_tags()` 建 id→Tag 映射表再回查，所以后端必须下发 id）：
- `DiseaseDetail.vue`：急诊等级徽章（`tag_group === 'emergency'`，含 `clinical_action`）、`body_system` / `mechanism` / `damnit_v` 三组标签、治疗协议卡片
- `DrugHandbook.vue`：`mechanismTags`（机制标签）+ 详情面板展示 `mechanism_of_action` / `pk_pd` / `adverse_mechanism`
- `CaseDetail.vue`：同样取 `get_tags()` 建映射
- `SymptomExplorer.vue`：**当前不渲染标签**（样式里留着 `.tag-chip`/`.card-tags` 但没有对应模板），只按 `category`/`frequency`/`stage`/`difficulty` 打徽章

## 代码规范

- 前端：`<script setup lang="ts">`，Composition API；实体类型一律 `import ... from '@/types'`，不在 `.vue` 里局部复制 interface（G4 的收敛方向）
- 后端：Tauri 2.0 命令模式，`Result<T, String>` 错误处理；`Vec<String>` 这类数组字段用 `db/models.rs` 的 `row_from_*` 手写映射（`#[derive(FromRow)]` 编不过，因为 SQLite 没有数组类型）
- YAML（仅 `data/tags.yaml` 仍是活的输入）：2 空格缩进，列表项 `- key: value`
- 数据库迁移：在 `db/mod.rs` 的 `apply_migrations` 里加 `if current < N` 块，**同时**把 `(N, "描述")` 补进 `current == 0` 的基线数组，否则全新库不会带上这条迁移。给已有表加列时，schema.sql 和迁移两处都要写——只写 schema.sql 就是 v13 那次"老库缺列、种子导入即崩"的成因
- 预留但暂不使用的常量/函数用 `#[allow(dead_code)]` + 中文理由保留（CI 的 clippy 是 `-D warnings`，不能靠静音忽略）

## 注意事项

- 修改数据后必须跑 `python tools/gen_from_json.py` 并递增 `SEED_DATA_VERSION`，两件事缺一不可
- 新增 Tauri 命令需在 `lib.rs` 的 `invoke_handler!` 中注册（只加 `#[tauri::command]` 不会生效）
- 新增前端页面需在 `router/index.ts` 添加路由并在 `Sidebar.vue` 的 `menuItems` 添加菜单项，否则会被 `route_check` 报死链
- 多值列（`diseases.species` / `diseases.category`）在 DB 里是 JSON 文本，后端用 `models.rs::parse_string_list()` 解析成数组后才下发，前端只消费数组；新增同类列请复用同一函数，不要在视图里 `split(',')`
- 药物编号有 10 组重复待逐组定夺，清单与独有内容见 `docs/drug-dup-unique-content.md`；游戏侧的 id 映射是 `data/game_drug_to_vk_id.json`（`GameView.vue` 直接 import），核对脚本 `python tools/check_drug_map.py`
- `data/drugs/drug_051.md` 之类 Markdown 已废弃（`drug_051` = 马罗匹坦 / Maropitant）；改内容只改 `data/knowledge.json`，别再动 `.md`

## 开发检查流程（必须遵守）

> 规则来源：`docs/WORKFLOW_REFORM.md`（问题诊断）、`_knowledge/rules/static_analysis_rules.md`（逐条规则定义）、`_knowledge/workflows/ci_check_standard.md`（Quick/Full/Single 三档模式与触发时机）

**每次 commit 前（本机装了 pre-commit hook 时自动跑；`.git/hooks/` 不入版本库，新克隆需手动装一次）：**
```bash
python _tools/dev/check.py --config check_config.json --quick
# 未安装 hook 时：python _tools/dev/check.py --install-hook
```

**新增页面 / 改路由时**：`route_check` + `api_check` 会验证模板里的路由与 `invoke('...')` 命令名是否存在（配置见 `check_config.json`，分别指向 `src/router/index.ts` 与 `src-tauri/src/lib.rs`）；规则文本在 `_knowledge/rules/static_analysis_rules.md`。

**改错误处理 / 加测试时**：分别看 `_knowledge/workflows/error_handling_standard.md`（E-04 根因可追溯、E-08 禁止把用户可见错误留给 `panic!`）与 `_knowledge/workflows/testing_standard.md`（分层与命名约定）。

**修 Bug / 技术债**：按 `_knowledge/workflows/fix_sop.md` 的三阶段闭环走。

**阶段交付前全套检查：**
```bash
python _tools/dev/check.py --config check_config.json --full   # 六个检查器全跑
cd src-tauri && cargo fmt --check
cd src-tauri && cargo clippy --all-targets -- -D warnings
cd src-tauri && cargo test
npm run lint && npx vue-tsc --noEmit
```
以上五条就是 `CONTRIBUTING.md` 的本地门禁清单，也是 `.github/workflows/ci.yml` 在 CI 上跑的内容。
