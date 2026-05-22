# 兽医知识库 / Vet Knowledge Base

> 一个面向兽医专业学生与临床医师的结构化知识学习与诊断推理平台。基于症状的推理引擎驱动差异化诊断，SM-2 间隔重复算法驱动个性化复习计划。

---

## 功能特性

### 诊断推理引擎
输入症状组合（发热、呕吐、腹泻…），引擎结合疾病-症状关联频率、病原性症状权重、物种与年龄上下文，计算加权匹配评分，返回排序的候选疾病列表，并附带推荐检查项目与鉴别要点。

### 闪卡复习（SM-2 算法）
采用改良版 SuperMemo SM-2 间隔重复算法，根据每次复习质量动态调整卡片间隔，优先推送即将遗忘的内容，实现真正意义上的自适应学习。

### 病例学习
从主诉到预后完整理清临床病例的决策链，每条病例关联对应的疾病知识点并支持交叉检索。

### 疾病对比
将 2–4 种疾病并排显示，从流行病学、病理生理、治疗方案多维度对比，辅助鉴别诊断训练。

### 数据导入 / 导出
支持 JSON 格式全量数据导出与导入，便于备份与知识库迁移。

---

## 技术架构

```
┌─────────────────────────────────────────────────┐
│                  Desktop Shell                   │
│              (Tauri 2 / WebView2)              │
├──────────────┬──────────────────────────────────┤
│   Frontend  │          Rust Backend              │
│              │                                   │
│  Vue 3      │  SQLite (WAL + FTS5)              │
│  TypeScript  │  Inference Engine (加权评分)         │
│  Pinia       │  SM-2 Scheduler                  │
│  Vue Router  │                                   │
│  Vite        │                                   │
└──────────────┴──────────────────────────────────┘
```

**前端：** Vue 3 + TypeScript + Vite + Pinia + Vue Router
**后端：** Tauri 2（Rust 编写，内存占用 < 80MB，无 Electron 式 Node.js 运行时）
**数据库：** SQLite（WAL 模式 + FTS5 全文检索）

### SM-2 间隔重复算法原理

复习质量评分 0–5 → 难度系数 EF ∈ [1.3, 2.5+] → 间隔天数 = 上次间隔 × EF → 下次复习时间自动推送。

---

## 系统要求

| 项目 | 要求 |
|------|------|
| 操作系统 | Windows 10/11 (x64) |
| 内存 | ≥ 4GB RAM |
| WebView2 | Windows 11 自带；Windows 10 需安装 [WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) |

---

## 安装指南

### 方式一：下载运行（推荐普通用户）

1. 前往 [Releases](https://github.com/ningjie333/vet-knowledge/releases) 下载最新 `.exe` 安装包
2. 运行安装程序，一路下一步即可

### 方式二：从源码构建（开发者）

**前置依赖：** Rust 1.70+、Node.js 18+、pnpm

```bash
git clone https://github.com/ningjie333/vet-knowledge.git
cd vet-knowledge
pnpm install
pnpm tauri dev      # 开发模式
pnpm tauri build   # 生产构建
```

构建产物位于 `src-tauri/target/release/bundle/nsis/`。

---

## 许可证

MIT License
