# 工程规范化目标与执行契约（2026-10-15）

本文是本轮"全面工程规范化 + README 对齐"的唯一目标清单与协作契约。
多 agent 并行执行，**文件所有权互不相交**；集成、验收、提交由主会话负责。

## 一、目标（本次要做）

| # | 目标 | 验收标准 |
|---|------|----------|
| G1 | 建立 CI 质量门禁 | `.github/workflows/ci.yml`：后端 job（windows-latest：`cargo fmt --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test`）+ 前端 job（ubuntu-latest：`npm ci`、`npm run lint`、`npm run build`）。推送到 master 后 GitHub Actions 实际跑绿。 |
| G2 | Rust 静态检查清零 | `cargo clippy --all-targets` 警告数为 **0**（不是"少"）。预留协议常量/构建器方法用 `#[allow(dead_code)]`+中文理由保留；重构残留死代码直接删。 |
| G3 | 前端规范配套 | eslint(flat)+prettier 落地，`npm run lint` 退出码 0；`npx vue-tsc --noEmit` 保持 0 错误。 |
| G4 | 前端类型统一 | `DiseaseCompare.vue` / `SymptomExplorer.vue` 不再有局部复制的实体 `interface`，一律引用 `@/types`。 |
| G5 | README 对齐现实 | 三处失实全部修正：①"数据导入/导出"功能不存在→从特性列表移除、移入"路线图"；②"SQLite WAL + FTS5"→按实际写（journal_mode=DELETE、搜索为 LIKE）；③"Pinia"→注册未使用，从技术栈移除。另补下载/安装、开发环境、常用命令、质量门禁。 |
| G6 | 过期文档处置 | `CODE_WIKI.md` 重写为紧凑且逐条可核对的架构文档（每条论断标出处）；`CLAUDE.md` 两处失效引用修正；新增 `CONTRIBUTING.md`（本地门禁+提交约定+发布流程）。 |
| G7 | species_notes 端到端 | 后端把 `species_notes` 解析为结构化数组下发（与 species/category 同层修法），前端正常渲染，共享类型同步，配契约测试。**由主会话实施**（跨 Rust/前端所有权边界）。 |

## 二、非目标（本次明确不做，防止范围蔓延）

- 不实现"数据导入/导出"功能（README 对齐方向是**文档就代码**，不是反过来）；不引入 Pinia 状态层。
- 不做 git 历史重写（旧账户名问题单独等拍板）；不动 10 组重复药物（等内容定夺）。
- 不引入 pre-commit 钩子、发布自动化。

## 三、文件所有权矩阵（越界即返工）

| 负责人 | 拥有 |
|--------|------|
| Agent A（CI） | `.github/workflows/**`、`src-tauri/build.rs`、`src-tauri/.cargo/config.toml`、`.gitignore` |
| Agent B（Rust） | `src-tauri/src/**`（仅此） |
| Agent C（前端） | `src/**`、`package.json`、`package-lock.json`、`eslint.config.js`、`.prettierrc*`、`.prettierignore`、`.editorconfig` |
| Agent D（文档） | `README.md`、`CODE_WIKI.md`、`CLAUDE.md`、`CONTRIBUTING.md`、`docs/**`（**除** `REMEDIATION.md`、`ENGINEERING_STANDARDS.md`） |
| 主会话 | `docs/REMEDIATION.md`、本文、集成期一切跨界修改、所有 commit |

## 四、跨 agent 契约（先定接口，再并行）

1. npm scripts 名称由 C 落地、A 引用：`lint`（eslint .）、`format`。A 的 CI 只调 `npm run lint` / `npm run build`。
2. B **禁止**跑 crate 级 `cargo fmt`（会碰到 A 的 build.rs）；只允许对**自己改过的文件**跑 `rustfmt <file>`。
3. C 的 prettier 必须配 `.prettierignore` 排除 `docs/`、`*.md`、`src-tauri/`，不得格式化他人文件。
4. 任何人不碰 `src-tauri/Cargo.toml`（不加 `[lints]` 段）。
5. 任何人**不 commit、不 push**；改动留在工作区由主会话逐 diff 验收后提交。
6. 本机网络：`api.github.com` 可达；`github.com`/`raw.githubusercontent.com` 直连被墙（如需 HTTP 用 `curl -x http://127.0.0.1:7890`）。npm registry 与 crates.io 可达。
7. Rust 工具链需 `export PATH="$HOME/.cargo/bin:$PATH"`；clippy/rustfmt 组件已装。

## 五、验收方式（主会话执行，绿≠已验证）

- 逐 diff 阅读全部改动；每条目标跑对应门禁命令留证据。
- **变异测试**：新增守卫必须"删掉就红"——CI 引用不存在的 script 要失败、clippy 清零回退一条要变红、eslint 规则违例要红、共享类型断引用要红、SM-2 语义测试改值要红。
- 文档逐条抽查：新 README/CODE_WIKI 引用的路径、命令、表名、迁移号，grep 验证存在。
- 推送后以 `api.github.com` 轮询 Actions 运行结果，确认 CI 真实跑绿（未跑绿不算 G1 完成）。
- species_notes 以真实运行界面截图收尾。
