# 兽医知识库 / Vet Knowledge Base

> 一个面向兽医专业学生与临床医师的结构化知识学习与诊断推理平台。基于症状的推理引擎驱动差异化诊断，SM-2 间隔重复算法驱动个性化复习计划。

[English](#english) | [中文](#中文)

---

## 功能特性 / Features

### 诊断推理引擎
输入症状组合（发热、呕吐、腹泻…），引擎结合疾病-症状关联频率、病原性症状权重、物种与年龄上下文，计算加权匹配评分，返回排序的候选疾病列表，并附带推荐检查项目与鉴别要点。

### 知识图谱
将疾病、症状、药物、检查项目之间的多维关系映射为可视化网络，支持按系统（呼吸/泌尿/消化…）筛选与折叠。

### 闪卡复习（SM-2 算法）
采用改良版 SuperMemo SM-2 间隔重复算法，根据每次复习质量动态调整卡片间隔，优先推送即将遗忘的内容，实现真正意义上的自适应学习。

### 病例学习
从主诉到预后完整理清临床病例的决策链，每条病例关联对应的疾病知识点并支持交叉检索。

### 疾病对比
将 2–4 种疾病并排显示，从流行病学、病理生理、治疗方案多维度对比，辅助鉴别诊断训练。

### 数据导入 / 导出
支持 JSON 格式全量数据导出与导入，便于备份与知识库迁移。

---

## 技术架构 / Architecture

```
┌─────────────────────────────────────────────────┐
│                  Desktop Shell                   │
│              (Tauri 2 / WebView2)               │
├──────────────┬──────────────────────────────────┤
│   Frontend   │          Rust Backend             │
│              │                                  │
│  Vue 3       │  SQLite (WAL + FTS5)            │
│  TypeScript  │  Inference Engine (加权评分)      │
│  Pinia       │  SM-2 Scheduler                  │
│  Vue Router  │  Migration-based Schema Versioning │
│  Vite        │                                  │
└──────────────┴──────────────────────────────────┘
```

### 前端 / Frontend
- **Vue 3** (Composition API + `<script setup>`)
- **TypeScript** 5.x — 类型安全全覆盖
- **Vite 6** — 毫秒级 HMR，构建产物 gzip ≤ 150kb
- **Pinia** — 轻量级状态管理（比 Vuex 更符合直觉）
- **Vue Router 4** — 懒加载路由，按需加载各模块
- **Noto Sans SC** — 思源黑体优化中文字体渲染

### 后端 / Backend
- **Tauri 2** — 用 Rust 编写系统级桌面应用，内存占用 < 80MB，无 Electron 式 Node.js 运行时
- **Rust** — 编译型语言，内存安全，零 GC 停顿
- **SQLite (sqlx)** — WAL 模式保证并发读写；FTS5 虚拟表提供极速全文检索（疾病 / 症状 / 药物 / 病例）
- **自定义推理引擎** — 基于症状频率权重（common/uncommon/rare）+ 核心症状（pathognomonic）加成 + 输入覆盖度计算综合评分

### 数据模型
```
Diseases ←→ DiseaseSymptom ←→ Symptoms
  ↓            ↓               ↓
DiseaseDDX   DiseaseDiagnostic  DiagnosticTests
  ↓            ↓               ↓
Diseases   DiseaseTreatment    Drugs
```

### 间隔重复 / SM-2 Algorithm
复习质量评分 0–5 → 难度系数 EF ∈ [1.3, 2.5+] → 间隔天数 = 上次间隔 × EF → 下次复习时间自动推送。EF 低于 1.3 的卡片会被降低难度重新学习。

### 数据库迁移
Schema 采用幂等迁移脚本（schema_migrations 表追踪版本），支持从任意历史版本平滑升级到最新版本，数据完全保留。

---

## 系统要求 / System Requirements

| 项目 | 要求 |
|------|------|
| 操作系统 | Windows 10/11 (x64) |
| 内存 | ≥ 4GB RAM |
| 磁盘 | ≥ 200MB 可用空间 |
| WebView2 | Windows 11 自带；Windows 10 需安装 [WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) |
| Rust | ≥ 1.70（仅开发构建需要） |
| Node.js | ≥ 18.x（仅开发构建需要） |

---

## 安装指南 / Installation

### 方式一：直接下载运行（推荐普通用户）

1. 前往 [Releases](https://github.com/ningjie333/vet-knowledge/releases) 下载最新版本的 `.exe` 安装包（NSIS 安装程序）
2. 运行安装程序，一路下一步即可
3. 安装完成后从开始菜单或桌面快捷方式启动

> 无需安装 Node.js 或 Rust，运行时通过 WebView2 加载前端，Rust 原生二进制提供全部后端逻辑。

### 方式二：从源码构建（开发者）

#### 前置依赖

```bash
# 1. 安装 Rust（Windows）
# 访问 https://rustup.rs，选择默认安装
rustup default stable

# 2. 安装 Node.js 18+
# 访问 https://nodejs.org 或使用 winget
winget install OpenJS.NodeJS.LTS

# 3. 安装 pnpm（可选，npm / yarn 亦可）
npm install -g pnpm

# 4. 安装 Rust 工具链目标
rustup target add x86_64-pc-windows-msvc
```

#### 构建步骤

```bash
# 克隆项目
git clone https://github.com/ningjie333/vet-knowledge.git
cd vet-knowledge

# 安装前端依赖
pnpm install

# 开发模式运行（热重载 + Rust 后端）
pnpm tauri dev

# 生产构建（生成 installer）
pnpm tauri build
```

构建产物位于 `src-tauri/target/release/bundle/nsis/` 目录下，为 `.exe` 安装包。

#### 常见构建问题

| 问题 | 解决方案 |
|------|---------|
| `tauri: command not found` | 先运行 `pnpm install`，确保 `.cargo/bin` 在 PATH 中 |
| WebView2 缺失 | 下载 [WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) |
| 链接器报错（LNK） | 确保已运行 `rustup target add x86_64-pc-windows-msvc` |
| sqlite3-sys 构建失败 | 安装 [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio)，勾选"使用 C++ 的桌面开发" |

---

## 数据存储位置

所有数据（SQLite 数据库、用户设置）保存在操作系统用户数据目录：

| OS | 路径 |
|----|------|
| Windows | `%APPDATA%\com.vetknowledge.app\` |
| macOS | `~/Library/Application Support/com.vetknowledge.app/` |
| Linux | `~/.local/share/com.vetknowledge.app/` |

---

## 项目结构 / Project Structure

```
vet-knowledge/
├── src/                          # Vue 3 前端源码
│   ├── main.ts                   # 应用入口
│   ├── App.vue                   # 根组件
│   ├── router/index.ts           # 路由配置（14 条路由）
│   ├── components/               # 可复用组件（Sidebar、TopBar…）
│   └── views/                    # 页面视图
│       ├── knowledge/            # 知识库模块（疾病、症状、药物…）
│       ├── learning/             # 学习模块（闪卡、病例）
│       ├── graph/                # 知识图谱
│       └── game/                 # 诊断游戏
│
├── src-tauri/                    # Rust 后端源码
│   ├── src/
│   │   ├── lib.rs                # Tauri 应用入口、命令注册
│   │   ├── main.rs               # 可执行入口
│   │   ├── engine.rs             # 诊断推理引擎（加权评分算法）
│   │   ├── db/
│   │   │   ├── mod.rs            # 数据库连接池、迁移、PRAGMA 优化
│   │   │   └── models.rs         # 全部数据模型（12 个结构体）
│   │   └── commands/             # Tauri IPC 命令（知识查询、搜索、诊断…）
│   ├── data/seed/               # 初始种子数据（SQL 格式）
│   ├── tauri.conf.json           # Tauri 配置
│   └── Cargo.toml                # Rust 依赖
│
├── vite.config.ts               # Vite 构建配置
├── tsconfig.json                # TypeScript 配置
└── package.json                 # Node.js 依赖
```

---

## 开发路线图 / Roadmap

- [x] 疾病 / 症状 / 药物数据管理
- [x] 症状推理诊断引擎
- [x] SM-2 闪卡复习系统
- [x] 全文搜索（FTS5）
- [x] 疾病对比
- [x] 数据导入 / 导出
- [ ] 知识图谱可视化
- [ ] 诊断游戏模块
- [ ] 移动端适配

---

## 许可证 / License

MIT License

---

## English

# Vet Knowledge Base

A structured veterinary knowledge learning and diagnostic reasoning platform for veterinary students and clinical practitioners. Features symptom-driven differential diagnosis engine and SM-2 spaced repetition algorithm for personalized learning.

### Tech Stack

| Layer | Technology |
|-------|------------|
| Desktop Shell | Tauri 2 (Rust, WebView2) |
| Frontend | Vue 3, TypeScript, Vite, Pinia, Vue Router |
| Database | SQLite with WAL mode + FTS5 full-text search |
| Inference | Custom weighted scoring engine (symptom frequency × pathognomonic boost) |
| Scheduler | SM-2 spaced repetition algorithm |

### Features

- **Diagnostic Reasoning Engine** — Enter symptom combinations; engine returns ranked candidate diseases with recommended lab tests
- **Flashcard System** — SM-2 algorithm schedules reviews based on your recall quality, not fixed intervals
- **Case Library** — Clinical cases from chief complaint to prognosis with learning points
- **Disease Comparison** — Side-by-side multi-dimensional comparison of 2–4 diseases
- **Full-Text Search** — SQLite FTS5 searches across diseases, symptoms, drugs, and cases simultaneously
- **Import / Export** — JSON backup and restore

### Installation (Developer)

```bash
# Prerequisites: Rust 1.70+, Node.js 18+, pnpm
git clone https://github.com/ningjie333/vet-knowledge.git
cd vet-knowledge
pnpm install
pnpm tauri dev      # development mode
pnpm tauri build    # production build
```

Executable will be at `src-tauri/target/release/bundle/nsis/*.exe`.

### Data Location

| OS | Path |
|----|------|
| Windows | `%APPDATA%\com.vetknowledge.app\` |
| macOS | `~/Library/Application Support/com.vetknowledge.app/` |
| Linux | `~/.local/share/com.vetknowledge.app/` |
