# 整改清单与验证记录

生成时间：2026-10-15　基线：GitHub master = `93d6a5b`（2026-06-29），本地已推进到 `be29ec3`（领先 13 个提交，**未推送**）

本轮用三个并行改动面完成修复，整合阶段逐项复核。下表"证据层级"很关键：**能跑的**才算验证过，跑不了的如实标注。

---

## 一、已完成（本轮提交）

### 1. 恢复"能编译"——补回从未入库的两个命令 【证据层级：静态符号审计，未经编译器】
`lib.rs` 自 `0a809d7` 起注册 `game_advance_async` / `game_poll_status`，但定义一直只留在工作区，`0a809d7..6e0c092` 共 8 个提交在干净检出下都编译不过（提交 `537dfb9` 补入）。

整合时顺手做了全量对账，防止同类问题：46 个注册命令 **全部**有 `pub` + `#[tauri::command]` 定义、模块路径全部可解析、前端 `invoke()` 的命令全部已注册。

**为什么之前没人发现**：项目的 `api_check` 把"注册名"和"定义名"取了并集（`_tools/dev/checkers/api_check.py:71-86`），结构上就看不见"注册了但没定义"。根因是更前面一层——**这台机器没有 Rust 工具链**（`cargo`/`rustc`/rustup/VS Build Tools 全不存在，`src-tauri/target/` 里的 `.rustc_info.json` 是旧机器留下的），所以 7 月以来的 Rust 代码一次都没编译过。

### 2. 移除写死的私人机器路径 【证据层级：grep 可验证，已验证】
`manager.rs` 与 `game.rs` 各有一份写死的 `C:\Users\<某账户>\Desktop\...`（账户名本身带"无密码"备注，
公开仓库上等于把账户名与这条备注一并公布）。收敛为唯一实现 `sidecar::root::resolve_virtual_vet_root()`：环境变量 `VET_VET_ROOT` → 当前目录同级 `../virtual-vet` 探测 → 解析不出就走各调用点原有的降级分支。`grep -rn "Users" src-tauri/src` 现为 0 命中，新增 5 个单元测试。

⚠️ **未消除**：该字面量仍在 git 历史里（`git log -S` 命中已推送的 `6512386`/`56b650d`/`3699916`
三个提交，另有 `tools/__pycache__` 两个已入库的 .pyc 也含它），公开仓库仍可翻到。要彻底清除必须
改写历史并强推。另外那个账户建议尽快设密码。本文件刻意不再复述该字面量。

⚠️ **功能影响**：同级探测依赖启动时的工作目录。`tauri dev`/`cargo test` 的 cwd 常在 `src-tauri`，此时探测落空（只是报错误导性没了，游戏功能仍不可用）。要开箱即用，需在启动脚本里显式设 `VET_VET_ROOT`——本轮没动，因为那会把你的本机路径写进受版本控制的文件。

### 3. 标签区永远空白 —— 断在四处，全部打通 【证据层级：数据库层可跑，已验证；界面层未验证】
- **生成器**只遍历疾病并硬编码 `'disease'`：现在 disease 425 / drug 72 / symptom 68 / treatment 19，与 `knowledge.json` 引用数逐类相等。
- **后端结构体**根本没有 tags 字段（前端类型却声明了 `tags: string[]`）：四个实体补齐，`Vec<String>` 在 SQLite 无 Decode，故手写 `FromRow` 复用 `row_from_*`。
- **自检工具**把 `tags` 放进了 `type_check.ignore_fields`，使这个不一致永远不被发现：豁免已解除，`python _tools/dev/check.py` 类型一致性现在 0 问题（换回旧 models.rs 立刻报 4 条，证明守卫有效）。
- **契约错位（整合阶段发现并修正）**：后端第一版下发的是标签**中文名**，而前端是拿 id 去回查 `get_tags()` 取颜色和分组——字段虽然有了，标签区**仍然空白**。改为下发 `tags.id` 后实测：疾病 6/6、症状 2/2、苯巴比妥 1/1 全部回查命中，显示为中文名带色块，0 行无法解析。

顺带堵住一个静默腐化点：生成器原先把词表外的标签登记成"中文名=英文 id"，界面会直接显示 `damnit_parasitic` 这类原文。现在未登记即报错（变异测试：临时塞一个自创标签 → `AssertionError` 指名到具体疾病；移除后通过）。已存在的 7 个自创写法中 6 个归并到既有标签（如 `emergency_red`→`em_red`），"寄生虫"作为新档进 `tags.yaml`。注册表从 45（含 7 个伪造）回到 39。

### 4. 用药冲突警告指错药 【证据层级：数据库层可跑，已验证】
根因不是"两份表谁对谁错"，而是 `knowledge.json` 里有 **10 组同名重复药物**（80 条只有 70 余个唯一名），两份映射表各指一条副本，疾病治疗方案也把它们混着用。

已做：`GameView.vue` 删掉 61 条硬编码字面量，改为 import 唯一 JSON 来源；把指向被弃用副本的引用统一重定向到字段更全的记录——疾病治疗方案 14 处 + 相互作用 3 处（`drug_042`→`drug_016`）。**实测**：苯巴比妥此前按 `drug_016` 查不到任何相互作用（严重冲突警告根本弹不出来），现在能查到 3 条（1 major / 2 moderate）。

**没做，需要你逐组定夺**：10 组重复记录本身一条都没删。被弃用那条里有独有的临床内容（癫痫持续状态的静脉抢救剂量、溴化钾负荷方案、猫禁用理由、PK 数值分歧等），见 `docs/drug-dup-unique-content.md`。`tools/check_drug_map.py` 会持续失败在"存在同名重复药物"这一项，作为待办的可执行提醒。

### 5. 前端构建失败 【证据层级：已验证】
`npm run build` 第一步 `vue-tsc --noEmit` 报 2 个 TS6133（`DrugHandbook.vue` 里 `severityColor`/`severityLabel` 从未被引用，颜色和文案已由模板+CSS 硬编码）。删除后 `npx vue-tsc --noEmit` 退出码 0。

### 6. 种子版本
`SEED_DATA_VERSION` 19 → 22，下次启动会整库重灌（这是设计行为：版本低就单事务清空重放，失败则下次开机重试）。

---

## 二、本轮**没碰**、按优先级排队

| 优先级 | 事项 | 说明 |
|---|---|---|
| ~~P0~~ | ~~装回 Rust 工具链~~ | **已完成**（2026-10-15）：rustc 1.99.0 + VS Build Tools 14.44 + SDK 10.0.26100。装完立刻暴露出下面两个"存量库升级即崩"，见第四节。 |
| ~~P0~~ | ~~推送决策~~ | **已推送**（2026-10-15，`93d6a5b..6786cd3`，20 个提交）。推送前做过隐私审计，见第四节末。 |
| ~~P0~~ | ~~新版安装包~~ | **已产出并核验**（见第四节第 4 条）。但版本号仍是 0.1.0 —— 对外发布前需要你确认是否提版本号。 |
| **P1** | 历史里的私人路径 | 需要改写历史 + 强推才能让公开仓库彻底查不到，属破坏性操作，需你授权。 |
| **P1** | 10 组重复药物合并 | 见 `docs/drug-dup-unique-content.md`，需要你的专业判断。 |
| **P1** | README 的三处失实 | 仍写着"数据导入/导出"（该功能整个不存在，`import_export.rs` 没有文件）、"SQLite WAL + FTS5"（实际是 `journal_mode=DELETE`，FTS5 已被迁移 v10 删除，搜索走 `LIKE`）、"Pinia"（已注册但 `src/stores/`、`src/composables/` 是空目录）。 |
| **P2** | 视图各自复制实体类型 | `DiseaseCompare.vue` / `SymptomExplorer.vue` 里各有一份局部 `interface Disease`，字段类型与 `src/types/index.ts` 脱节 —— 这正是 species/category 那类不一致反复出现的温床。应改为复用共享类型（本轮只把值改对，没做这个重构）。 |
| **P2** | `CODE_WIKI.md` 整体过期 | 1425 行里没有一处 `sidecar`/`virtual-vet`/`VET_VET_ROOT`，仍描述已删的 FTS 重建、`gen_from_yaml.py`、`SEED_DATA_VERSION=18`。`CLAUDE.md` 引用的两个工作流文档不存在。 |
| **P2** | 症状备注仍以 JSON 文本显示 | `symptoms.species_notes` 存的是 `{"犬": "…", "猫": "…"}`，症状检索页把原文直接打成一段文本。本轮只统一了 species / category 两个多值列，这个字段需要配套的展示方式，未动。 |
| **P3** | 27 个注册命令前端从不调用 | 编译器也确认了死代码：13 条 warning（`row_from_tag`、`entity_tags_check_constraint`、7 个未用错误码常量等）。 |
| **P3** | 仓库杂物 | 远端仍带着约 0.97 MB 的 `gen/schemas` 生成物、3 MB `vendor/` 补丁 crate、9 个 `__pycache__/*.pyc`（其中 `gen_sql.cpython-314.pyc` 的源文件已不存在）、259 份已废弃的 Markdown 数据文件。 |
| **P3** | 两处待清理 | `src-tauri/.cargo/config.toml` 是未跟踪的全注释文件（零配置项）；`tools/convert_to_json.py` 有 2 处硬编码自检告警（一次性迁移脚本，非本轮引入，是 `python _tools/dev/check.py` 目前唯一 remaining FAIL）。 |
| **P3** | `.gitignore` 新加的 `/data/` | `knowledge.json` 等已跟踪所以照常提交，但今后 `data/` 下**新**文件不带 `-f` 是加不进去的，容易哪天把事实源漏在库外。 |
| **P3** | 发布包文件名 | 唯一发布的安装包名为 `_0.1.0_x64-setup.exe`，开头多一个下划线 —— 打包时应用名没进文件名。 |

---

## 三、一处数据存疑（请你复核，我没有改）

保留下来的苯巴比妥（`drug_016`）标签只有"疼痛信号"一项；曲马多、溴化钾的保留记录同样只有这一项，而它们各自的重复副本 `tags` 为空。巴比妥类归到"疼痛信号"维度看起来不像本意，可能是早期打标签时的错位。

---

## 四、装回工具链之后才发现的事（2026-10-15 下午）

工具链装上后第一次真编译 + 第一次真跑应用，暴露出 4 件**在此之前无法知道**的事。

### 1. 两个"存量库升级即崩"（对已装过 0.1.0 的用户是致命的） 【已修复，实机验证】
拿一个停在 v8 的真实旧库启动，第一次崩在 `FOREIGN KEY constraint failed`，修好后第二次崩在
`table drugs has no column named pubchem_cid`。两个都是"全新安装永远测不出、老用户一升级必踩"：

- **v9 迁移的连接错位**：`PRAGMA foreign_keys = OFF` 用 `execute(pool)` 发出，而 SQLite 的 PRAGMA
  按连接生效、池里有 2 条连接 → 事务实际跑在另一条仍开外键的连接上，重建 tags 时
  `DROP TABLE tags` 被子表引用挡住。改为在同一条 acquire 到的连接上完成 关→重建→开。
- **drugs 的 6 个 PubChem 列从来没有迁移**：只写在 schema.sql 里，种子 SQL 按当前列名 INSERT，
  老库缺列直接失败。新增 v13 补列（先查 `sqlite_master` / `pragma_table_info`，表或列已存在则跳过）。
  全库结构比对确认漂移面只有这一张表。

修复后实机验证：旧库启动**不再产生错误日志**，库从 504 KB 涨到 740 KB，
`seed_data_version=22`、迁移到 13、四类 entity_tags（425/72/68/19）、80 种药、11 条相互作用、
外键检查零违规，苯巴比妥按 `drug_016` 可查到 3 条相互作用。

### 2. 多值列以 JSON 文本下发，4 个消费者各写各的 【已修复，截图确认】
`diseases.species` 存 `["犬", "猫"]`、`diseases.category` 存 `["泌尿系统", "急诊"]`，后端原样发字符串：
疾病列表页 `JSON.parse` 是唯一写对的一处；详情页 `v-for` 遍历字符串 → Vue 按**字符**迭代，界面上出现
一格一个 `[` `"` `,` 的碎标签；对比页把 JSON 原文当文本打印；症状检索页 `split(',')` 会切出 `["犬"`
这种脏值塞进物种筛选下拉。改为后端统一解析成真数组（`parse_string_list`），前端只消费数组。

### 3. 标签区在真应用里确实显示出来了 【截图确认】
疾病详情页出现"红色 / 中毒 / 中毒·创伤"等带色标签块与"高急迫性疾病"提示条；症状检索页相关疾病卡片的
标签从 `["免疫系统"]` 变成干净的"免疫系统"。首页统计 85 疾病 / 44 症状 / 80 药物。
截图留在 `.ui-probe/`（未入库，内含数据库副本，可删）。

### 4. 安装包：已产出（离线补齐 NSIS 工具链）
`npx tauri build` 主程序构建一直正常，卡点是打包器要联网取 NSIS，而本机访问不到 GitHub 的
release 资源（`failed to bundle project: timeout: global`）。解法记录如下，下次直接照做：

1. Tauri CLI 校验 `%LOCALAPPDATA%\tauri\NSIS` 下 **17 个固定路径**是否存在，缺任一项就
   **删掉整个目录**再重新下载 —— 所以"只补一个文件"必然失败，必须一次铺满。
2. 需要两份官方产物：
   - `nsis-3.11.zip`（`github.com/tauri-apps/binary-releases/releases/download/nsis-3.11/`）
   - `Plugins/x86-unicode/additional/nsis_tauri_utils.dll`（`github.com/tauri-apps/nsis-tauri-utils`
     的 `nsis_tauri_utils-v0.5.3` release；这个插件不在 NSIS 包里，是单独发布的）
3. 两者都可通过可达的 GitHub 代理下载。完整性用 **CLI 二进制里硬编码的 SHA-1** 校验，比任何
   第三方来源都权威：在 `node_modules/@tauri-apps/cli-win32-x64-msvc/*.node` 里搜 URL 即可看到
   紧跟其后的期望哈希（本次两文件均逐字节吻合）。
4. 一次性解压 zip（去掉 `nsis-3.11/` 前缀）+ 放入插件，再跑 `npx tauri build --bundles nsis`。
5. 产物：`src-tauri/target/release/bundle/nsis/兽医知识库_0.1.0_x64-setup.exe`（4.02 MB）。
   文件名带上了产品名，顺带修掉了旧发布包 `_0.1.0_x64-setup.exe` 前导下划线的问题。

核验：静默安装后，安装目录里的 exe 与构建产物**仅差 3 字节**（`UNK` → `NSS`，即 Tauri 写入的
打包类型标记），版本号 0.1.0；从安装位置启动后疾病详情页的标签与物种胶囊显示正常。
注意：打包前必须完全退出应用，否则 exe 被占用会报 `拒绝访问 (os error 5)`。

### 我先前判断错、后被实测推翻的三条（留档避免再犯）
1. **"这台机器没有 WebView2"** —— 错。独立运行库在 `C:\Program Files (x86)\Microsoft\EdgeWebView\Application`
   （154/155），我当时查的注册表键在这台机器上恰好是空的。
2. **"这个应用装完从未打开过"** —— 错。数据目录标识是 `com.vetknowledge.app`（无连字符），
   我当时按带连字符的名字找，没找到就下了结论。
3. **"池里两条连接外键强度不一致"** —— 错。sqlx 的连接选项默认就开外键；我做了变异测试
   （退回旧写法）才发现测试没红，于是把测试改钉 `cache_size`（非默认值，旧写法确实只命中一条连接：
   实测两条连接分别是 -2000 与 -64000）。

### 验证手段记录（下次不必重新摸索）
- `cargo test --lib` 77 绿；每个新守卫都做变异（删守卫→必须红→还原→绿）。
- 前端 `npx vue-tsc --noEmit` 退出码 0；`python _tools/dev/check.py` 除 2 条既有告警外全过。
- 看真界面：`npx tauri build --no-bundle`（`cargo build --release` 单独跑会让窗口去连 devUrl，
  因为 `#[cfg(dev)]` 由 tauri CLI 决定）→ PrintWindow 抓窗口自身像素（普通截屏会被别的窗口压住）→
  点击前需模拟一次 ALT 解除前台锁，坐标换算约为 屏幕 y = 界面 y - 45。
