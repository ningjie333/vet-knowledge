# 贡献指南

适用于 `vet-knowledge` 仓库（Tauri 2 + Rust 后端、Vue 3 前端）。架构细节见 [CODE_WIKI.md](CODE_WIKI.md)，仓库约定见 [CLAUDE.md](CLAUDE.md)。

---

## 一、提交约定

**格式**：Conventional Commits，**类型和 scope 用英文，标题与正文用中文**（`git log` 里 53 个提交全部是这个形态）。

```
<type>(<scope>): <中文摘要，不超过约 60 字>

<中文正文：为什么改、根因是什么、怎么验证的>
```

- **type**：`feat` / `fix` / `docs` / `chore` / `refactor`（当前仓库只用过这五种）。破坏性变更前加 `!`。
- **scope**（可省略）：小写英文模块名，取已有用过的：`db`、`data`、`tags`、`sidecar`、`game`、`build`。新增 scope 时保持同样的粒度，不要用中文。
- **标题**：说清了什么事，不以句号结尾。
- **正文**：改动的原因与验证手段，不是文件清单。参考 `git show 009a4f2`（两处"存量库升级即崩"的根因 + 三个守卫测试 + 变异验证结果）、`git show a3a967d`（多值列四个消费者的各自表现）。这类说明半年后是唯一还能用的证据。
- 一次提交只做一件事。数据内容变更（跑 `tools/gen_from_json.py` + 递增 `SEED_DATA_VERSION`）与代码改动分开提交。

---

## 二、本地门禁清单

**前置**：Rust stable（`x86_64-pc-windows-msvc`）+ VS 2022 Build Tools（C++ 工作负载与 Windows SDK）+ Node 20。若 rustup 未写系统 PATH，先：

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

然后按顺序全部跑绿再提交：

| # | 命令 | 在哪个目录跑 | 说明 |
|---|------|--------------|------|
| 1 | `cargo fmt --check` | `src-tauri/` | 只格式化 Rust；改过的文件可用 `rustfmt <file>` 单文件处理 |
| 2 | `cargo clippy --all-targets -- -D warnings` | `src-tauri/` | **告警数必须为 0**，CI 同样带 `-D warnings`。有意预留的常量/函数用 `#[allow(dead_code)]` + 中文理由保留 |
| 3 | `cargo test` | `src-tauri/` | 含迁移升级回归、种子端到端、SM-2 语义、推理引擎 |
| 4 | `npm run lint` | 仓库根 | ESLint（flat config，`eslint.config.js`） |
| 5 | `npx vue-tsc --noEmit` | 仓库根 | 类型检查；`npm run build` 已包含它，但单独跑更快、报错更聚焦 |

注意第 1–3 条必须在 `src-tauri/` 下执行（Cargo workspace 根就是该目录），第 4–5 条必须在仓库根执行（`package.json` 在这里）。

可选的跨文件一致性检查（不是 CI 的一部分，但能提前抓出死链与命令名漂移）：

```bash
python _tools/dev/check.py --config check_config.json --quick    # route_check + api_check
python _tools/dev/check.py --config check_config.json --full     # 六个检查器全跑
```

`.git/hooks/pre-commit` 是本机用 `python _tools/dev/check.py --install-hook` 装的，hooks 目录不入库，新克隆需要重装一次。

**新增守卫测试时请做变异验证**：把被测的守卫删掉/改坏，测试必须变红，再还原。只写不验证的断言等同没写（`docs/ENGINEERING_STANDARDS.md` 第五节）。

---

## 三、发布流程

版本号一共**六处**，必须同时改；漏掉任何一处会让界面、安装包或 CI 里的版本互相矛盾。

| # | 文件 | 位置 |
|---|------|------|
| 1 | `src-tauri/tauri.conf.json` | 顶层 `"version"`（决定安装包文件名与 `getVersion()` 返回值） |
| 2 | `src-tauri/Cargo.toml` | `[package] version` |
| 3 | `src-tauri/Cargo.lock` | `[[package]] name = "vet-knowledge"` 那一段的 `version` |
| 4 | `package.json` | 顶层 `"version"` |
| 5 | `package-lock.json` | 顶层 `"version"` |
| 6 | `package-lock.json` | `packages[""].version`（第二个 `"version"`） |

核对命令（六处输出应当是同一个版本号）：

```bash
grep -n '"version"' package.json src-tauri/tauri.conf.json
grep -n '"version": "<新版本>"' package-lock.json          # 应命中 2 行
grep -n '^version = ' src-tauri/Cargo.toml
grep -n -A1 '^name = "vet-knowledge"' src-tauri/Cargo.lock  # 命中段里的 version
```

第 3 处可以手改，也可以在改完 `Cargo.toml` 后跑任意 cargo 命令让 Cargo 自己同步锁文件。前端界面里的版本号不需要改：`src/components/TopBar.vue` 用 `getVersion()` 动态读取，它取的就是第 1 处。

**步骤**：

1. 跑完上面"本地门禁清单"全部五条命令。
2. 六处版本号一起升到 `<新版本>`，提交 `chore: 版本升至 <新版本>`。
3. 确认数据变更是否已经反映：若 `src-tauri/data/seed/001_initial.sql` 变过，`src-tauri/src/db/mod.rs` 的 `SEED_DATA_VERSION` 必须已递增，否则老用户的库不会重导种子。
4. 关掉正在运行的应用（否则第 4 步会报 `拒绝访问 (os error 5)`），构建安装包：

   ```bash
   npx tauri build --bundles nsis
   ```

   产物：`src-tauri/target/release/bundle/nsis/兽医知识库_<新版本>_x64-setup.exe`（`productName` 是中文，所以文件名带中文）。
   这台机器上 NSIS 需要联网下载，直连 GitHub release 资源不可达时会报 `failed to bundle project: timeout: global`，离线铺目录的完整办法见 [docs/REMEDIATION.md 第四节 4](docs/REMEDIATION.md)。临时只要可执行文件用 `npx tauri build --no-bundle`。
   **不要**用 `cargo build --release` 的产物代替：`#[cfg(dev)]` 由 Tauri CLI 注入，裸 cargo 的 exe 会去连 devUrl 并显示空白错误页。
5. 把产物重命名为**英文名**再上传：`vet-knowledge_<新版本>_x64-setup.exe`。GitHub 在上传 release 附件时会把文件名里的非 ASCII 字符剥掉，中文产品名会变成前导下划线的残骸——线上 v0.1.0 的资产名 `_0.1.0_x64-setup.exe` 就是这么来的。
6. 打标签并推送：`git tag -a v<新版本> -m "v<新版本>"` → `git push origin master v<新版本>`。
7. 在 GitHub Releases 建草稿 → 上传第 5 步的英文命名安装包 → 核对文件大小与本地一致 → 发布。种子数据是 `include_str!` 编译进 exe 的，已安装的副本不会自行更新，**只有新安装包能修数据**。
8. 推送后确认 `.github/workflows/ci.yml` 的 `backend` 与 `frontend` 两个 job 都跑绿；CI 不打包，产物只在本地生成。

---

## 四、动手前请避开的坑

- **迁移与 schema 是两套并行真相**：给已有表加列要同时写进 `src-tauri/data/seed/schema.sql` 和 `src-tauri/src/db/mod.rs` 的迁移，还要补进 `current == 0` 的基线数组。历史上漏掉迁移列导致过"老库升级即崩"（迁移 v13）。
- **全新库测不到迁移 bug**：`apply_migrations()` 对 `MAX(version)=0` 的新库直接写基线、不回放射。迁移改动必须用"造一个停在旧版本的库再升级"的回归测试来验（参考 `test_upgrade_from_pre_v9_database_keeps_tag_rows`）。
- **PRAGMA 按连接生效**：连接池有 2 条连接，新增 PRAGMA 只能加在 `db::sqlite_options()` 里，不要用 `execute(&pool)` 事后设置。
- **`entity_type` 三处同步**：`schema.sql` 的 CHECK、`db/entity_types.rs` 的 `ENTITY_TYPES`、`tools/gen_from_json.py` 的 `ENTITY_TAG_SCOPES`。
- **不要提交运行时数据库与截图产物**：`*.db`、`data/`（运行时生成）、`.ui-probe/` 都已在 `.gitignore` 中。
