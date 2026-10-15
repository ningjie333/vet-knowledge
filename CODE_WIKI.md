# 兽医知识库 · CODE WIKI

本文只写**代码当前实际做了什么**。每条论断后面给出可核对的出处（文件路径 + 符号名）；无法用代码或仓库内文档核对的说法一律不写。
基准：v0.2.0（tag `v0.2.0` → commit `6790a4e`）之后的工作区。若某处描述的是本轮工程规范化的目标状态（CI、ESLint、共享类型），会显式标注"目标态"并指向 `docs/ENGINEERING_STANDARDS.md` 的对应目标编号。

---

## 1. 系统总览

**形态**：Windows 单机桌面应用，没有服务端、没有账号体系。Tauri 2 壳 + WebView2 渲染 Vue SPA，Rust 进程内直接读写本地 SQLite（`src-tauri/tauri.conf.json` → `bundle.targets: ["nsis"]`、`identifier: "com.vetknowledge.app"`、`productName: "兽医知识库"`）。

**四个运行时部件**（启动顺序见 `src-tauri/src/lib.rs` 的 `run()`）：

| 部件 | 位置 | 职责 |
|------|------|------|
| Vue SPA | `src/` → 构建到 `dist/` | 界面与交互，经 `invoke()` 取数 |
| Rust 命令层 | `src-tauri/src/commands/` | 46 条 `#[tauri::command]`，全部注册在 `lib.rs` 的 `invoke_handler!` |
| SQLite | `%APPDATA%\com.vetknowledge.app\vet_knowledge.db`（release） | 唯一持久层，`src-tauri/src/db/mod.rs` |
| Python sidecar | 外部仓库 virtual-vet | 仅"诊断游戏"用，懒启动，可缺席（`src-tauri/src/sidecar/mod.rs`） |

**数据规模**（按 `src-tauri/data/seed/001_initial.sql` 的 INSERT 行数统计）：
85 diseases、44 symptoms、80 drugs、36 diagnostic_tests、15 cases、6 treatments、39 tags、584 entity_tags、340 disease_symptom、254 disease_treatment、156 disease_diagnostic、53 disease_ddx、15 case_disease、11 drug_interactions、10 disease_treatment_map。

**十个导航入口**（`src/components/Sidebar.vue` 的 `menuItems` ↔ `src/router/index.ts`）：首页、疾病百科、症状→疾病、药物手册、症状推理、知识图谱、病例库、闪卡复习、疾病对比、诊断游戏。其中 `/graph` 指向的 `src/views/graph/KnowledgeGraph.vue` 只有 15 行占位（模板里写着"开发中"），其余九个均有实现。

**已核实"不存在"的东西**（旧文档写过、代码里没有，勿再引用）：

| 说法 | 核实方式 |
|------|----------|
| 数据导入 / 导出功能 | `src-tauri/src/commands/` 只有 knowledge/treatments/search/diagnose/flashcards/game 六个模块（`commands/mod.rs`），`lib.rs` 的注册表里无 `import_*`/`export_*` |
| Pinia 状态层 | `src/main.ts` 有 `app.use(createPinia())`，但 `src/stores/`、`src/composables/` 是空目录 |
| FTS5 全文检索 | schema.sql 无 `*_fts` 表；虚表由迁移 v10 `DROP`（`db/mod.rs`），搜索一直是 LIKE（`commands/search.rs`） |
| WAL 日志模式 | `db::sqlite_options()` 显式 `SqliteJournalMode::Delete` |
| `learning_progress` 表 | 迁移 v11 `DROP`（`db/mod.rs`） |

---

## 2. 前端模块地图

**入口与配置**

- `src/main.ts`：`createApp(App)` → `use(createPinia())` → `use(router)` → `mount('#app')`。Pinia 注册但无 store 可用，实际数据层是各视图里的 `invoke()`。
- `vite.config.ts`：别名 `@` → `src`；`server.port: 1420` + `strictPort`（与 `tauri.conf.json` 的 `build.devUrl` 成对）；`envPrefix: ['VITE_','TAURI_']`。
- `tsconfig.json`：`strict` + `noUnusedLocals` + `noUnusedParameters`，因此 `npx vue-tsc --noEmit` 会把未使用变量判成错误。
- `index.html`：`<link>` 外链 Google Fonts（Noto Sans SC）；离线时回落系统字体，不影响功能。
- `src/router/index.ts`：13 条路由，组件全部 `() => import(...)` 懒加载。`/game` 指向 `GameView.vue`；`src/views/game/GameHome.vue`（15 行占位）**没有被任何地方引用**。

**组件与视图**

| 文件 | 作用 | 主要命令 |
|------|------|----------|
| `src/App.vue` | 侧边栏 + 顶栏 + `<router-view>` 布局 | — |
| `src/components/Sidebar.vue` | 十个导航项 | — |
| `src/components/TopBar.vue` | 全局搜索框 + 版本号 | `full_text_search`、`getVersion` |
| `src/views/knowledge/Home.vue` | 统计卡片（取数组长度） | `get_diseases`/`get_symptoms`/`get_drugs`/`get_cases` |
| `src/views/knowledge/DiseaseList.vue` | 疾病列表 + 物种/分类筛选 | `get_diseases` |
| `src/views/knowledge/DiseaseDetail.vue` | 详情 + 标签徽章 + 治疗协议 | `get_disease_by_id` 等 |
| `src/views/knowledge/SymptomExplorer.vue` | 症状→疾病反查 | `get_symptoms`、`get_diseases_by_symptom` |
| `src/views/knowledge/SymptomChecker.vue` | 症状推理 UI | `infer_diagnosis`、`get_disease_by_id` |
| `src/views/knowledge/DrugHandbook.vue` | 药物手册（机制/PK-PD/标签） | `get_drugs`、`get_drug_by_id` |
| `src/views/knowledge/DiseaseCompare.vue` | 2–4 疾病并排对比 | `get_disease_compare` |
| `src/views/learning/CaseLibrary.vue` | 病例列表 + 搜索 | `get_cases`、`full_text_search` |
| `src/views/learning/CaseDetail.vue` | 病例分段 + 关联疾病链接 | `get_case_by_id`、`get_case_diseases` |
| `src/views/learning/CaseStudy.vue` | 占位（"推理训练模式开发中"） | — |
| `src/views/learning/FlashcardStudy.vue` | 闪卡复习 | `get_due_flashcards`、`review_flashcard` 等 |
| `src/views/game/GameView.vue` | 诊断游戏 | 11 条 `game_*` |

**类型**：`src/types/index.ts` 是前端唯一的类型来源，对应关系分三处核对：实体与推理 DTO（`Tag`/`Disease`/`Symptom`/`Drug`/`Treatment`/`DiagnosticTest`/`Case`/`SearchResult`/`DiagnosisCandidate`/`TestSuggestion`/`DrugInfo`/`DrugInteraction`）↔ `src-tauri/src/db/models.rs`；`Flashcard`/`ReviewStats` ↔ `commands/flashcards.rs`；`Game*`/`NewSessionResponse` ↔ `commands/game.rs`。另有关系元组别名（`DiseaseSymptomRow`/`DiseaseDrugRow`/`DiseaseDiagnosticRow`/`DiseaseDdxRow`/`DiseaseTreatmentRow`）描述关系表查询的返回形状，`AdvanceJobInfo`/`AdvancePollResult`/`GameReport*` 描述 sidecar 透传的 JSON——`game_advance_async`/`game_poll_status` 在 Rust 侧就是返回 `serde_json::Value`。目标态：视图内不再局部复制实体 interface，一律 `import ... from '@/types'`（`docs/ENGINEERING_STANDARDS.md` G4）。

**调用约定**：Tauri v2 把 JS 侧 camelCase 实参映射到 Rust 侧 snake_case 形参——证据是 `Home.vue` 调 `invoke('get_drugs', { drugClass: null })`，而 `commands/knowledge.rs` 的签名是 `pub async fn get_drugs(pool, drug_class: Option<String>, ...)`。

---

## 3. Rust 命令层

**`src-tauri/src/lib.rs` 的 `run()`**：装 `tauri_plugin_shell` / `_fs` / `_dialog` 三个插件 → `setup` 里 `block_on(db::init(&app))`，成功则 `app.manage(pool)`，失败则 `db::write_import_error_log` + `dialog().message(...).blocking_show()` + `std::process::exit(1)`（不用 `panic!`，依据 `_knowledge/workflows/error_handling_standard.md`）→ `app.manage(sidecar::SidecarManager::new())`（懒启动，构造期不拉起 Python）→ `invoke_handler!` 注册 46 条命令 → `RunEvent::ExitRequested` 时 `sidecar.shutdown()`。

**命令分布**（`grep -c '#\[tauri::command\]' src-tauri/src/commands/*.rs`，与 `lib.rs` 注册项一一对应）：

| 模块 | 条数 | 代表命令 |
|------|------|----------|
| `commands/knowledge.rs` | 18 | `get_diseases`、`get_disease_compare`、`get_diseases_by_symptom`、`get_drug_interactions` |
| `commands/treatments.rs` | 8 | `get_treatments`、`get_tags`、`get_entity_tags`、`add_entity_tag` |
| `commands/search.rs` | 1 | `full_text_search` |
| `commands/diagnose.rs` | 1 | `infer_diagnosis` |
| `commands/flashcards.rs` | 7 | `generate_flashcards_from_knowledge`、`review_flashcard`、`get_review_stats` |
| `commands/game.rs` | 11 | `game_new_session`、`game_advance_async`、`game_diagnosis_hint` |

**错误约定**：一律 `Result<T, String>` + `.map_err(|e| e.to_string())?`（`commands/knowledge.rs` 全篇）。

**查询构造**：`db/sql_builder.rs` 的 `SqlBuilder`（`select`/`optional_where`/`and_where`/`and_in`/`order_by`/`limit`/`build`）+ `placeholders(count)` + `tags_subquery(row_alias, entity_type)`。列表类命令的形态是 `format!("SELECT d.*, {} FROM diseases d", tags_subquery("d", "disease"))` 再交给 `SqlBuilder`（`knowledge.rs` 的 `get_diseases`）。物种/分类筛选用 `LIKE`，因为这两列在 DB 里是 JSON 文本（`models.rs` 的 `Disease.species` 注释）。

**推理引擎**（`src-tauri/src/engine.rs` → `infer()`，纯函数不碰 DB）：频率权重 common 1.0 / uncommon 0.6 / rare 0.3 / 其他 0.5；标志性症状 `is_pathognomonic == 1` 再乘 1.5；`score = 疾病覆盖度 × 输入覆盖度`，每个命中的标志性症状 +0.05（封顶 0.15），`score` 截到 1.0；`score < 0.25` 或 `input_coverage < 0.3` 直接丢弃；按分排序后 `truncate(10)`。注意 `TestSuggestion.test_name` 目前被赋成 `test_id`，`DiagnosisCandidate.distinguishing_features` 恒为 `Vec::new()`——两者都还没有真正的解析实现，界面也未渲染。
`commands/diagnose.rs` 负责喂数据：先按 `d.species LIKE ?` 取候选疾病，再批量取 `disease_symptom` 与 `disease_diagnostic`。

**SM-2**（`commands/flashcards.rs` → `calculate_sm2(quality, old_interval, old_ef) -> Sm2Result`，纯函数）：`EF' = max(1.3, EF + (0.1 - (5-q)*(0.08 + (5-q)*0.02)))`；非首次时 quality<3 → 间隔重置 1 天、difficulty 0.8，old_interval<1.5 → 6 天、0.5，否则 `old_interval × new_ef`、0.3；首次（`old_interval` 或 `old_ef` 为 `None`）quality≤3 → 1 天，quality≥4 → 6 天。

**诊断游戏**（`commands/game.rs`）：11 条里 10 条把请求转成 `sidecar.call("game.<method>", params)`，只有 `game_diagnosis_hint` 在本地跑——它先用 `translate_symptoms()` 读 `{virtual-vet 根}/data/virtual_vet_to_vk_symptom_map.json` 把游戏症状名翻译成知识库症状名，再走 `engine::infer`。

---

## 4. 数据库层（`src-tauri/src/db/`）

### 4.1 连接与 PRAGMA 集中策略

`db/mod.rs` → `sqlite_options(db_url)` 是唯一声明 PRAGMA 的地方：`create_if_missing(true)`、`journal_mode(Delete)`、`synchronous(Full)`、`foreign_keys(true)`、`pragma("cache_size","-64000")`、`pragma("temp_store","MEMORY")`。`create_pool(db_url)` 用 `SqlitePoolOptions::new().max_connections(2).connect_with(sqlite_options(...)?)`，生产与测试共用这一条构造路径。

为什么要集中（`sqlite_options` 与 `apply_migrations` v9 分支的注释、以及 `docs/REMEDIATION.md` 第四节 1）：

- SQLite 的 PRAGMA **按连接**生效，而池里有 2 条连接。启动后用 `execute(&pool)` 设 PRAGMA 只会命中当时借出的那一条，另一条仍是默认值——同一配置在两条连接上不一致，`cache_size` 这类参数会静默只对一半生效。
- 真正的线上事故是迁移 v9：`PRAGMA foreign_keys = OFF` 打在 A 连接上，`pool.begin()` 的事务却跑在仍开启外键的 B 连接，`DROP TABLE tags` 报 `FOREIGN KEY constraint failed`，凡是数据库停在 v9 之前的用户升级后启动即崩。现在 v9 在**同一条** `pool.acquire()` 到的连接上完成"关外键 → 重建 → 开外键"。
- 用 DELETE 而不是 WAL：WAL 需要 `.db-shm` 共享内存文件，沙箱化环境会拦截并导致 disk I/O error；DELETE 无 SHM 依赖，单用户桌面足够。相应地 `synchronous` 保持 FULL 以保证崩溃安全。
- 守住这件事的三个测试：`test_every_pool_connection_gets_the_same_pragmas`（同时持有两条连接，断言两条的 `foreign_keys=1` 且 `cache_size=-64000`）、`test_upgrade_from_pre_v9_database_keeps_tag_rows`（造一个停在 v8 的旧库跑迁移，断言最终 `MAX(version)=13`、标签行不丢、迁移后外键恢复生效）、`test_legacy_drugs_schema_upgrades_then_accepts_seed`（把 drugs 退回缺 PubChem 列的结构，迁移后必须能吃下真实种子的第一条 drugs INSERT）。

### 4.2 建表与 SQL 解析

`db::init(app)`：库文件位置按 `cfg!(debug_assertions)` 分支——debug 用 `CARGO_MANIFEST_DIR` 的父目录（仓库根）下的 `data/`，release 用 `app.path().app_data_dir()`。随后依次 `schema.sql` → `apply_migrations` → `import_seed_data`。两个 SQL 文件都是 `include_str!` 进二进制的（`src-tauri/data/seed/schema.sql`、`src-tauri/data/seed/001_initial.sql`）。

`strip_sql_comments()` 先删掉行首 `--` 注释，再交给 `split_sql_statements()`；后者是手写的字节状态机，跟踪单/双引号状态（含 `''` 转义）只在引号外按 `;` 切分。顺序不能反：`db/mod.rs` 的注释写明"否则第一个 statement 会以 `--` 开头被跳过 → 表不建"。`test_split_sql_*` 四个单测覆盖引号内分号、`''` 转义、注释与空行。
`execute_tolerant()` 只用在无法写成 `IF NOT EXISTS` 的场景（`ALTER TABLE ADD COLUMN`、RENAME），失败打 `[migration <ctx>]` 到 stderr 但不阻断——幂等的 CREATE/DROP 一律用 `?` 传播错误。

### 4.3 迁移链 v1–v13

迁移表 `schema_migrations(version INTEGER PRIMARY KEY, applied_at, description)` 与元数据表 `app_meta(key PRIMARY KEY, value)` 都由 `apply_migrations()` 开头 `CREATE TABLE IF NOT EXISTS` 建立；当前版本取 `SELECT MAX(version)`。

**基线捷径**：`current == 0`（全新库）时**不回放历史迁移**，直接把下面 13 条描述写进 `schema_migrations` 就返回——因为 schema.sql 已经是最新结构。副作用是"全新库永远测不到迁移分支的 bug"，两个升级即崩缺陷就是这么漏过去的（`docs/REMEDIATION.md` 第四节 1）。

| 版本 | 实际做的事 | 出处（`db/mod.rs`） |
|------|-----------|--------------------|
| v1 | 初始 schema（全表 + 索引 + learning_progress）；分支里只有一条 `INSERT OR IGNORE` 版本记录，没有 DDL | `db/mod.rs` 注释 `── v1:` |
| v2 | `diseases.urgency_level`（1–5 CHECK）与 `disease_symptom.is_pathognomonic` 两列，`execute_tolerant` 容忍重复列名 | `db/mod.rs` 注释 `── v2:` |
| v3 | 重建 `disease_diagnostic`：主键加 `species`（CREATE v2 → INSERT SELECT → DROP → RENAME） | `db/mod.rs` 注释 `── v3:` |
| v4 | 重建 `disease_treatment`：主键加 `species`，链路同 v3 | `db/mod.rs` 注释 `── v4:` |
| v5 | 闪卡表 `flashcards` + `flashcard_reviews`（`quality` 0–5 CHECK、`next_review` NOT NULL）+ `idx_flashcard_reviews_next` | `db/mod.rs` 注释 `── v5:` |
| v6 | `diseases.data_source`（seed/user/imported CHECK）与 `diseases.is_deleted` | `db/mod.rs` 注释 `── v6:` |
| v7 | 标签系统 `tags` + `entity_tags`（`entity_type` CHECK 五值）、`treatments` + `disease_treatment_map`、diseases(5)/symptoms(1)/drugs(3) 共 9 个增强列、三个标签索引 | `db/mod.rs` 注释 `── v7:` |
| v8 | 历史上建过 `cases_fts` 虚表；现在**只写版本记录、不再建表**（被 v10 撤销），注释说明 search.rs 一直用 LIKE | `db/mod.rs` 注释 `── v8:` |
| v9 | 重建 `tags` 去掉 `UNIQUE(name_zh)`；前置条件 `current >= 7`；关/开外键与事务必须在同一条连接 | `db/mod.rs` 注释 `── v9:`，条件现为 `if (7..9).contains(&current)` |
| v10 | `DROP` 四张从未被使用的 FTS5 虚表：`diseases_fts`/`symptoms_fts`/`drugs_fts`/`cases_fts` | `db/mod.rs` 注释 `── v10:` |
| v11 | `DROP TABLE IF EXISTS learning_progress`（死表，无命令读写） | `db/mod.rs` 注释 `── v11:` |
| v12 | `drug_interactions`（`severity` major/moderate/mild CHECK，复合主键）+ 两个索引 | `db/mod.rs` 注释 `── v12:` |
| v13 | 给 `drugs` 补 6 个 PubChem 列（`pubchem_cid`/`molecular_formula`/`molecular_weight`/`smiles`/`inchikey`/`iupac_name`）；先用 `sqlite_master` 与 `pragma_table_info` 探表探列再 `ALTER`，版本号无条件落表 | `db/mod.rs` 注释 `── v13:` |

两点注意：分支在源码里的书写顺序是 …v9 → v12 → v13 → v11 → v10，与版本号无关，因为每个分支都用函数开头读到的同一个 `current` 判断；`schema.sql` 与迁移是**两套并行真相**，新增列若只写进 schema.sql 就会出现 v13 那种"新库有、老库没有"的事故。

### 4.4 种子数据版本机制

`const SEED_DATA_VERSION: i64 = 22;`（`db/mod.rs`）。`import_seed_data()` 读 `app_meta` 里 key=`seed_data_version` 的值：读不到（新库）或 **小于**常量才重导，否则直接返回。
重导是一个事务：先按固定顺序 `DELETE` 15 张表（`entity_tags`、`case_disease`、`disease_diagnostic`、`disease_treatment`、`disease_treatment_map`、`disease_symptom`、`disease_ddx`、`drug_interactions`、`cases`、`diagnostic_tests`、`drugs`、`symptoms`、`diseases`、`treatments`、`tags`），`no such table` 被容忍、其他错误立即 bail；然后逐条执行种子 SQL，任一条失败就用 `?` 返回（事务 drop 自动回滚，**版本号不写** → 下次启动重试）；最后 `INSERT OR REPLACE INTO app_meta` 记录版本并 commit。
失败细节由 `db::write_import_error_log()` 写进 `app_data_dir/seed_import_error.log`（含 `SEED_DATA_VERSION`、错误链、清理目录的提示），这是 release 包唯一可查的现场证据。
`test_seed_sql_imports_with_foreign_keys_on` 把 schema + 种子在外键开启的内存库里全量跑一遍，断言 `PRAGMA foreign_key_check` 无违规、且 disease/symptom/drug/treatment **四类实体都有 entity_tags 行**（症状/药物标签区永远空白的根因）。

### 4.5 标签与多值列的下发约定

`sql_builder::tags_subquery()` 在 SQL 侧用 `json_group_array(tag_id)` 把实体的标签聚成 JSON 文本、列别名固定为 `tags`，**下发的是标签 id 而不是中文名**——前端要用 id 回查 `get_tags()` 建的映射表取 color/tag_group/急诊等级（函数文档注释记录了这次契约错位的来龙去脉）。Rust 侧由 `db/models.rs` 的 `row_tags()` → `parse_tags_json()` 统一解析成 `Vec<String>`；列不存在（有的查询没附子查询）时返回空 Vec。
`diseases.species`、`diseases.category` 这类"SQLite 里没有数组类型所以存 JSON 文本"的列，由 `models.rs::parse_string_list()` 退回顺序解析：JSON 数组 → 逗号分隔 → 单值 → 空，坏 JSON 也保证不把 `[` `"` 泄漏到界面。`Disease`/`Symptom`/`Drug`/`Treatment` 因此**手写** `row_from_*` 而不是 `#[derive(FromRow)]`（`models.rs` 顶部注释说明 `Vec<String>` 不满足 sqlx 的 Decode）；`Case`、`DiagnosticTest`、`Tag` 仍是 derive + `query_as`。

`entity_type` 的取值必须三处同步，否则会撞 CHECK 约束或外键错误：`src-tauri/data/seed/schema.sql` 的 `entity_tags` CHECK、`db/entity_types.rs` 的 `ENTITY_TYPES`（五值：disease/symptom/drug/treatment/case）、`tools/gen_from_json.py` 的 `ENTITY_TAG_SCOPES`。`tags_subquery` 在 debug 构建下对非法值直接 `debug_assert!` panic。

### 4.6 表清单

`schema.sql` 建 **18 张表** + 12 个索引：实体 `diseases`/`symptoms`/`drugs`/`diagnostic_tests`/`cases`/`treatments`/`tags`；关系 `entity_tags`/`disease_symptom`/`disease_ddx`/`disease_treatment`/`disease_diagnostic`/`disease_treatment_map`/`case_disease`/`drug_interactions`；系统 `schema_migrations`/`flashcards`/`flashcard_reviews`。
`app_meta` **不在 schema.sql 里**，由 `apply_migrations()` 用 `CREATE TABLE IF NOT EXISTS` 建立（运行期合计 19 张）。FTS5 虚表与 `learning_progress` 已不存在（见 4.3 的 v10、v11）。

---

## 5. sidecar：virtual-vet 子进程

**根目录解析**（`src-tauri/src/sidecar/root.rs` → `resolve_virtual_vet_root()`，顺序固定）：

1. 环境变量 `VET_VET_ROOT`（常量 `ROOT_ENV`）非空 → 原样采用；
2. 否则探测**当前工作目录的同级目录** `../virtual-vet`（常量 `SIBLING_DIR`），存在则返回其绝对路径；
3. 都没有 → `None`，由调用方走降级分支。

第 2 级依赖 cwd（`npx tauri dev` / `cargo` 可能把 cwd 落在 `src-tauri/`，此时探测不到属正常），打包或非同级布局部署应显式设 `VET_VET_ROOT`。路径折叠用手写 `normalize()`，刻意不用 `fs::canonicalize`——Windows 下它会加 `\\?\` 前缀，Python 子进程处理不了。`root.rs` 的 5 个测试钉住"环境变量优先""无同级目录时返回 None（即结果里不存在任何写死的私人路径）""纯函数形态与真实调用形态一致"。

**进程管理**（`src-tauri/src/sidecar/manager.rs`）：全局单实例、首次 `call()` 才 spawn（懒启动）；`Mutex<Inner>` 保护 stdin/stdout，避免并发写导致响应错位；spawn 后必须先读到 `{"method":"ready",...}` 才算就绪（15 s 超时）；按方法分级超时 `method_timeout()`：`game.new_session` 30 s、`game.diagnose` 15 s、其他 10 s；IO 错误/超时 → `invalidate()`（kill + 清管道），下次调用重启；重启有频率限制，5 s 窗口内 ≥3 次就放弃并报 `sidecar restarting too fast`；stderr 全程 `eprintln` 转发便于排查；`shutdown()` 先发 `shutdown` RPC、等 2 s、超时强杀，`Drop` 用 `start_kill()` 兜底。
环境变量：`VET_VET_PYTHON`（默认 `python`）、`VET_VET_ROOT`、`VET_VET_SIDECAR_LOG`（`manager.rs` 头部注释）。

**协议**（`src-tauri/src/sidecar/protocol.rs`）：JSON-RPC 2.0 over stdio，一行一请求一行一响应，无通知无批量；`build_request()` / `parse_response()`；标准码 `-32700/-32600/-32601/-32602/-32603` 加业务码 `-32001..-32004`（session/case 不存在、动作非法、游戏已结束），由 `RpcErrorCode` 暴露；这些常量目前部分只被预留，故带 `#[allow(dead_code)]`。

**分发**：安装包**不含** virtual-vet，也不含 Python；`npx tauri build` 产出的 exe 在没有 sidecar 的机器上，除"诊断游戏"外其余功能正常（`game_*` 命令会返回 `Failed to start sidecar: ...`）。

---

## 6. capabilities：为什么需要 `default.json`

`src-tauri/capabilities/default.json`：`identifier: "default"`、`windows: ["main"]`、`permissions: ["core:default"]`、`$schema` 指向 `src-tauri/gen/schemas/desktop-schema.json`（该文件由 Tauri 生成，目录内还有 `capabilities.json`、`acl-manifests.json`、`windows-schema.json`）。

需要它的唯一原因是插件类前端 API：项目此前没有 capability 清单，`@tauri-apps/api` 的 `getVersion()` 被 ACL 静默拒绝、返回空，右上角只显示一个光秃秃的 "v"（提交 `714a445` 的说明与 `default.json` 的 `description` 字段同记此事）。应用自己 `#[tauri::command]` 的 IPC 不受这条 ACL 约束，所以加之前其余功能看起来都正常——这也是它容易被漏掉的原因。`TopBar.vue` 用 `getVersion()` 动态显示版本，因此界面上的版本就是安装包里的真实版本（取自 `tauri.conf.json` 的 `version`）。

---

## 7. 构建与打包

**脚本与配置的对接**（`package.json` scripts ↔ `tauri.conf.json` build 段）：`npm run tauri:dev` → `beforeDevCommand: "npm run dev"`（Vite 监听 1420）+ `devUrl: "http://localhost:1420"`；`npm run tauri:build` → `beforeBuildCommand: "npm run build"`（`vue-tsc --noEmit && vite build`）+ `frontendDist: "../dist"`。

**坑：dev/prod 开关由 Tauri CLI 决定，不是 profile。** 单独 `cargo build --release` 编译链接都没问题，但产出的 exe 会去导航 `devUrl`，屏幕上直接是 WebView2 的"无法访问此页面"——因为 `#[cfg(dev)]` 由 `tauri-build`/CLI 注入（`docs/REMEDIATION.md` 第四节"验证手段记录"）。因此构建必须走 `npx tauri build`；只要二进制用 `npx tauri build --no-bundle`，产物在 `src-tauri/target/release/vet-knowledge.exe`。

**NSIS 打包需要联网**。`bundle.targets: ["nsis"]`，CLI 会向 GitHub release 资源下载 NSIS，直连被墙的机器报 `failed to bundle project: timeout: global`。本机可行的离线办法（17 个固定路径校验、缺一项就整目录重下、需要 `nsis-3.11.zip` + `nsis_tauri_utils-v0.5.3` 两份产物、用 CLI 二进制里硬编码的 SHA-1 校验完整性）记录在 [docs/REMEDIATION.md 第四节 4](docs/REMEDIATION.md)。打包前必须完全退出应用，否则 exe 被占用会报 `拒绝访问 (os error 5)`。

**产物与发布命名**：安装包按 `productName` 生成，形如 `兽医知识库_0.2.0_x64-setup.exe`；GitHub 在上传时会把附件名里的非 ASCII 字符剥掉（v0.1.0 的线上资产就叫 `_0.1.0_x64-setup.exe`，前导下划线即残骸），所以发布时统一用英文名 `vet-knowledge_<version>_x64-setup.exe`。版本号共**六处**需要同步，清单见 [CONTRIBUTING.md](CONTRIBUTING.md)。

**依赖补丁**：`src-tauri/Cargo.toml` 的 `[patch.crates-io]` 把 `rustc_version`/`proc-macro2`/`quote`/`serde`/`serde_core`/`thiserror`/`zmij`/`anyhow`/`erased-serde`/`typeid`/`camino`/`autocfg` 指到仓库内 `vendor/`（394 个受版本控制的文件）。原因是 Windows 下用户名含中文时，`std::sys::process::output()` 在 `read_to_end` 返回 `code: 0` 的错误上 `unwrap()` panic，直接炸掉构建脚本（`Cargo.toml` 内注释 + 提交 `57f7a7e`）。`vendor/` 缺失会导致 `cargo` 无法解析依赖。

**dev 构建加速**：`Cargo.toml` 的 `[profile.dev] debug = 1`（行表）把 link.exe 链接时间从 ~8 s 降到 ~3 s；`src-tauri/.cargo/config.toml` 目前只有注释（历史上为绕 rustc ICE 关过 incremental，现已恢复默认）。

**CI 不打包**：`.github/workflows/ci.yml` 明确不跑 `tauri build`，理由是打包要下载 NSIS 且 CI 不需要产物。

---

## 8. 测试与质量门禁

**Rust**：`cd src-tauri && cargo test`。测试与被测代码同文件（`#[cfg(test)] mod tests`），分布：`db/mod.rs`（SQL 分割 4 个 + PRAGMA/升级回归 4 个）、`db/models.rs`（`parse_string_list`/`parse_tags_json` 等 12 个）、`db/sql_builder.rs`（10 个）、`db/entity_types.rs`（2 个，含注册表与 schema CHECK 不漂移的契约校验）、`engine.rs`（18 个）、`commands/flashcards.rs`（24 个，含 SM-2 首次复习 quality 0–5 的逐值表）、`commands/game.rs`（2 个）、`sidecar/root.rs`（5 个）。数量随改动浮动，以 `cargo test` 输出为准。守卫测试的编写要求（新增守卫必须"删掉就红"）见 `docs/ENGINEERING_STANDARDS.md` 第五节。

**前端**：`npm run build` 里已含 `vue-tsc --noEmit`（`package.json` 的 `build` 脚本）；`npm run lint` 跑 `eslint .`，配置是 ESLint 9 flat config `eslint.config.js`（`js` + `typescript-eslint` + `eslint-plugin-vue` essential + `eslint-config-prettier`），格式类规则统一交给 prettier（`npm run format` / `npm run format:check`，`.prettierignore` 排除了 `docs/`、`*.md`、`src-tauri/`）。

**CI**（`.github/workflows/ci.yml`）：`backend` job 跑 `windows-latest`，三步依次是 `cargo fmt --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test`，均 `working-directory: src-tauri`；`frontend` job 跑 `ubuntu-latest` + Node 20，三步是 `npm ci`、`npm run lint`、`npm run build`。触发条件是 push 到 `master` 与任意 PR，`permissions: contents: read`，同分支旧运行会被 `concurrency` 取消。

**跨文件一致性检查（Python）**：`python _tools/dev/check.py --config check_config.json --quick|--full|<name>`，六个检查器 `route_check`/`api_check`/`type_check`/`empty_check`/`hardcode_check`/`dead_code_check`（`_tools/dev/checkers/`），规则定义在 `_knowledge/rules/static_analysis_rules.md`，三档执行模式定义在 `_knowledge/workflows/ci_check_standard.md`。`check_config.json` 把它们指向 `src/router/index.ts`、`src-tauri/src/lib.rs`、`src/types`、`src-tauri/src/db/models.rs` 等真实文件。注意 `.git/hooks/pre-commit` 是本机用 `--install-hook` 装的，hooks 目录不入版本库，新克隆需重跑一次。

**本地提交前的完整清单**：见 [CONTRIBUTING.md](CONTRIBUTING.md)。

---

## 9. 数据管线（内容侧）

**单一数据源是 `data/knowledge.json`**，七个顶层数组：`diseases`(85)/`symptoms`(44)/`drugs`(80)/`tests`(36)/`cases`(15)/`treatments`(6)/`drug_interactions`(11)。关系内嵌在疾病对象里，不另设关系文件：`symptoms[]`（`{symptom, frequency, stage, is_pathognomonic}`）、`drugs[]`（`{drug, line, species, notes}`）、`diagnostics[]`（`{test, purpose, evidence_level, species, expected_result}`）、`treatments[]`、`ddx[]`。

**生成器** `python tools/gen_from_json.py`（608 行）→ 覆盖写 `src-tauri/data/seed/001_initial.sql`。要点：
- `S()` / `V()` 做 SQL 字面量转义，顺手剔除 `\x00`（NUL 会截断 SQLite 字符串）；
- 预置标签从 `data/tags.yaml` 读取（39 条：body_system 11 / mechanism 12 / species 5 / damnit_v 7 / emergency 4）；引用了 tags.yaml 里没有的标签会**直接报错**并要求补条目或改引用，而不是生成"中文名=英文 id"的脏标签；
- `verify_entity_tags()` 在写文件前断言 knowledge.json 里每个带 tags 的实体都落成了 entity_tags 行，并统计条数（`[VERIFY]` 输出）。

**内容变更后必须**：重跑生成器 → 递增 `db/mod.rs` 的 `SEED_DATA_VERSION` → `cargo test`（种子端到端那条会立刻暴露外键/列名问题）。

**已废弃**：`data/diseases/`、`data/symptoms/`、`data/drugs/`、`data/tests/`、`data/cases/`、`data/treatments/` 里的逐实体 Markdown（每个目录都放了 `DEPRECATED.md` 标记）以及 `data/relations.yaml`、`data/treatment_rules.yaml`——生成器不再读取它们，只有 `data/tags.yaml` 仍是活的输入。`src-tauri/src/db/mod.rs` 的 `include_str!` 只吃 `src-tauri/data/seed/` 下的两个文件。
