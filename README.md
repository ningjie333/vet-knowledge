# 兽医知识库 / Vet Knowledge Base

面向兽医专业学生与临床医师的 Windows 桌面应用：结构化的疾病 / 症状 / 药物 / 病例知识库，配合基于症状的鉴别诊断推理、SM-2 间隔重复复习与诊断游戏。技术栈为 Tauri 2（Rust）+ Vue 3 + SQLite，全部数据在本地单机运行。

知识库规模（以 `src-tauri/data/seed/001_initial.sql` 的导入行数统计）：**85 疾病 / 44 症状 / 80 药物 / 36 诊断检查 / 15 病例 / 6 治疗方案 / 39 预置标签**。

---

## 功能特性

- **诊断推理引擎**：勾选症状组合 + 物种，按疾病-症状关联频率加权（common 1.0 / uncommon 0.6 / rare 0.3，标志性症状 ×1.5）计算"疾病覆盖度 × 输入覆盖度"，返回按匹配分排序的候选疾病，并列出已匹配症状、缺失的关键症状与症状覆盖度。
- **症状→疾病反向检索**：从症状出发回查关联疾病，可按物种筛选，症状列表本身支持名称搜索。
- **闪卡复习（SM-2）**：改良版 SuperMemo SM-2，按回忆质量 0–5 更新间隔与难度系数 EF，支持从知识库批量生成卡片。
- **疾病对比**：2–4 种疾病并排比较流行病学、病理生理、症状与用药。
- **病例库**：从主诉、检查、化验、影像到诊断、治疗、转归与学习要点分段呈现，并链接到关联疾病条目。
- **全文检索**：顶部搜索框跨疾病 / 症状 / 药物 / 病例四张表做 `LIKE` 模糊匹配并按类型加权排序（当前实现不是 FTS5 全文索引，详见"技术架构"）。
- **诊断游戏**：对接外部 Python 项目 virtual-vet（JSON-RPC over stdio 子进程），需要单独克隆该仓库并设置 `VET_VET_ROOT`，见"开发环境要求"。

---

## 下载与安装

| 项目 | 说明 |
|------|------|
| 发布页 | <https://github.com/ningjie333/vet-knowledge/releases> |
| 当前版本 | **v0.2.0**（2026-10-15）· [v0.2.0 tag](https://github.com/ningjie333/vet-knowledge/releases/tag/v0.2.0) · 安装包 `vet-knowledge_0.2.0_x64-setup.exe`（约 4.2 MB） |
| 安装 | 运行安装程序，默认装到 `%LOCALAPPDATA%\兽医知识库\` |
| 数据目录 | `%APPDATA%\com.vetknowledge.app\`（`vet_knowledge.db`；启动失败时同目录会多出 `seed_import_error.log`） |

种子 SQL 通过 `include_str!` 编译进 exe（`src-tauri/src/db/mod.rs` 的 `init` / `import_seed_data`），因此**已安装的副本不会自行更新数据**：数据变更只能靠安装新版本，或删掉上面的数据目录后重启触发重新导入。

---

## 技术架构

```
┌───────────────────────────────────────────────────┐
│              Desktop Shell (Tauri 2 / WebView2)    │
├────────────────────┬──────────────────────────────┤
│  Frontend (Vue 3)  │   Rust 后端 (src-tauri)      │
│  TypeScript        │   commands/  46 条 Tauri 命令 │
│  Vue Router        │   engine.rs  诊断推理         │
│  Vite              │   db/        SQLite (sqlx)    │
│                    │   sidecar/   virtual-vet 子进程│
├────────────────────┴──────────────────────────────┤
│  SQLite：journal_mode=DELETE、synchronous=FULL、   │
│  外键开启；搜索为 LIKE（FTS5 虚表已由迁移 v10 删除）│
└───────────────────────────────────────────────────┘
```

**前端**：Vue 3 + TypeScript + Vite + Vue Router。`pinia` 在 `src/main.ts` 注册了 `createPinia()`，但项目当前没有任何 store（`src/stores/`、`src/composables/` 为空目录），各视图直接用 `invoke()` 取数，所以它**不是**实际的数据层。
**后端**：Tauri 2（Rust），SQLx 0.8 异步访问 SQLite；诊断游戏通过 stdin/stdout 的 JSON-RPC 2.0 调用外部 Python 进程。
**数据库**：SQLite 单文件。连接 PRAGMA 集中在 `src-tauri/src/db/mod.rs` 的 `sqlite_options()`——刻意用 **DELETE** 而非 WAL（WAL 需要 `.db-shm` 共享内存文件，沙箱环境会拦截并导致 disk I/O error），并保留 FULL 同步级别保证崩溃安全。

---

## 系统要求

| 项目 | 要求 |
|------|------|
| 操作系统 | Windows 10 / 11（x64） |
| WebView2 | Windows 11 自带；Windows 10 需安装 [WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) |
| 网络 | 运行时不需要（界面字体 Noto Sans SC 走 Google Fonts CDN，离线时回落系统字体） |

---

## 开发环境要求

- **Rust**：stable，宿主三元组 `x86_64-pc-windows-msvc`（MSVC 工具链，不是 GNU）。
- **Visual Studio 2022 Build Tools**：需"使用 C++ 的桌面开发"工作负载 + Windows SDK，提供 `link.exe`。
- **Node.js 20**（CI 固定在 20，见 `.github/workflows/ci.yml`）+ npm。
- **PATH**：若 rustup 按非默认方式安装、未写系统 PATH，每条命令前需 `export PATH="$HOME/.cargo/bin:$PATH"`。
- **依赖补丁**：`src-tauri/Cargo.toml` 的 `[patch.crates-io]` 把若干 crate 指向仓库内的 `vendor/`，用于绕开 Windows 中文用户名路径下 `std` 的 build panic；克隆仓库即带上，无需额外操作。
- **诊断游戏（可选）**：把 virtual-vet 检出到本仓库同级目录，或设置环境变量 `VET_VET_ROOT` 指向它（解析顺序见 `src-tauri/src/sidecar/root.rs`）。未配置时只有"诊断游戏"页报错，其余功能不受影响。
- **NSIS 打包**：`npx tauri build` 会联网向 GitHub release 资源下载 NSIS；网络不可达的机器需要先离线铺好 `%LOCALAPPDATA%\tauri\NSIS`，完整步骤与校验方法见 [docs/REMEDIATION.md 第四节 4](docs/REMEDIATION.md)。只要可执行文件用 `npx tauri build --no-bundle`。

---

## 常用命令

```bash
npm install                # 前端依赖

npm run dev                # 仅前端（Vite，端口 1420；此时 invoke 调用全部失败）
npm run tauri:dev          # 桌面窗口 + Rust 后端（开发模式）
npm run lint               # ESLint
npm run build              # vue-tsc --noEmit && vite build（类型检查是构建的一部分）

cd src-tauri
cargo test                 # Rust 单元测试（含数据库迁移、种子导入、SM-2、推理引擎）
cargo clippy --all-targets -- -D warnings

npx tauri build --bundles nsis   # 生产安装包 → src-tauri/target/release/bundle/nsis/
```

开发模式的数据库落在仓库根 `data/vet_knowledge.db`，发布版落在 `%APPDATA%\com.vetknowledge.app\`（分支逻辑见 `db::init` 里的 `cfg!(debug_assertions)`）。

---

## 质量门禁

推送与 Pull Request 会由 [`.github/workflows/ci.yml`](.github/workflows/ci.yml) 跑后端 `cargo fmt --check` / `cargo clippy --all-targets -- -D warnings` / `cargo test` 与前端 `npm ci` / `npm run lint` / `npm run build`；本地提交前先按 [CONTRIBUTING.md](CONTRIBUTING.md) 的门禁清单自查一遍。

---

## 文档

| 文件 | 内容 |
|------|------|
| [CODE_WIKI.md](CODE_WIKI.md) | 架构与现实对照：前端模块地图、Rust 命令层、迁移链、sidecar、构建打包 |
| [CLAUDE.md](CLAUDE.md) | 给 AI 编码助手的仓库速查（数据管线、命令注册位置、检查流程） |
| [CONTRIBUTING.md](CONTRIBUTING.md) | 提交约定、本地门禁清单、发布流程 |
| [docs/REMEDIATION.md](docs/REMEDIATION.md) | 整改台账与实测记录（含离线 NSIS 打包办法） |

## 路线图

以下是**尚未实现**、但被反复问到的能力：

- **数据导入 / 导出**：当前没有任何导入/导出命令（`src-tauri/src/commands/` 下无对应模块，`lib.rs` 的 `invoke_handler!` 也未注册），备份只能直接拷贝数据库文件。
- **诊断结果增强**：推理引擎已算出 `suggested_tests`（推荐检查）但界面未渲染，`distinguishing_features`（鉴别要点）目前是空数组，两处都在 `src-tauri/src/engine.rs` 的 `infer()` 里。
- **知识图谱可视化**：`src/views/graph/KnowledgeGraph.vue` 仍是占位页。
- **病例推理训练**：`src/views/learning/CaseStudy.vue` 仍是"开发中"占位页。
- **诊断游戏随包分发**：现在依赖开发者本机的 virtual-vet 检出。

---

## 许可证

MIT License，见 [LICENSE](LICENSE)。
