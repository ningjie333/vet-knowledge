# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

**兽医知识库** — 基于 Tauri 2.0 的桌面端兽医学习平台。Rust 后端 + Vue 3 前端，SQLite 数据库。

数据规模：50 疾病 / 45 症状 / 60 药物 / 36 诊断检查 / 15 病例 / 6 治疗协议。

## 构建与运行

```bash
# 启动开发（前端 + Rust 后端）
npm run tauri:dev

# 仅前端开发
npm run dev

# Rust 编译检查
cd src-tauri && cargo check

# 生产构建
npm run tauri:build
```

## 数据管线（重要）

**实体数据用 Markdown+Frontmatter，关系数据用 YAML**。通过脚本生成 SQL 种子数据：

```bash
# 修改数据后必须重新生成
python tools/gen_from_yaml.py
```

生成器输出到 `src-tauri/data/seed/001_initial.sql`。数据库版本号定义在 `src-tauri/src/db/mod.rs` 的 `SEED_DATA_VERSION`，数据变更后必须递增。

### 数据文件职责

**实体（Markdown + Frontmatter）**：每个实体一个 `.md` 文件，frontmatter 存结构化字段，body 存长文本。

| 目录 | 文件数 | frontmatter 字段 | body 章节 |
| --- | --- | --- | --- |
| `data/diseases/` | 50 | id, name_zh, name_en, name_latin, category, species, difficulty, urgency_level, pathogenic_type, epidemiology, body_system, physiological_basis, **tags[]** | 概述, 病因, 病理生理, 预后 |
| `data/symptoms/` | 45 | id, name_zh, name_en, physiological_basis, **tags[]** | 定义, 物种特异性 |
| `data/drugs/` | 60 | id, name_zh, name_en, drug_class, mechanism_of_action, pk_pd, adverse_mechanism, **tags[]** | 适应症, 禁忌症, 不良反应, 物种剂量 |
| `data/tests/` | 36 | id, name_zh, category, cost_estimate, turnaround_time | 参考范围, 结果解读 |
| `data/cases/` | 15 | id, title, species, breed, age, weight, difficulty | 主诉, 病史, 体格检查, 实验室检查, 影像学, 诊断, 治疗, 转归, 学习要点 |
| `data/treatments/` | 6 | id, name_zh, name_en, therapy_type, **tags[]** | 治疗原则, 操作指南, 生理基础, 预后评估 |

**关系（YAML）**：多对多关联，不适合拆成 MD。

| 文件 | 内容 |
|------|------|
| `data/relations.yaml` | 疾病-症状关联（frequency: common/uncommon/rare，is_pathognomonic）+ 鉴别诊断（ddx） |
| `data/treatment_rules.yaml` | 疾病-药物关联（line: first/second/adjunctive）+ 疾病-诊断关联（purpose, evidence_level）+ 疾病-治疗方案关联 |
| `data/tags.yaml` | 预置标签（48个），含 tag_group / emergency_level / clinical_action 等字段 |

### 标签系统（Tagging）

标签通过 `entity_tags` 多态关联表实现，支持一个实体拥有多个不同分组的标签。

**标签分组（tag_group）**：

| 分组 | 说明 | 示例 |
|------|------|------|
| `body_system` | 解剖系统（11种） | 呼吸系统、消化系统、心血管系统... |
| `mechanism` | 病理机制（11种） | 炎症免疫、代谢、血管循环、梗阻、感染... |
| `emergency` | 急诊四级决策 | 🔴红(立即复苏) 🟠橙(15min介入) 🟡黄(60min介入) 🟢绿(常规) |
| `damnit_v` | DAMNIT-V诊断模型 | 退行性、畸形/先天、代谢性、肿瘤性... |
| `species` | 物种 | 犬、猫、马、牛、异宠 |

**急诊分级标签字段**：`emergency_level`（red/orange/yellow/green）、`clinical_action`（临床行动指导）、`textbook_logic`（教科书逻辑）、`typical_scenario`（典型场景）。

### 新增疾病流程

1. 在 `data/diseases/` 下新建 `dis_xxx.md`（参考现有文件格式，包含 `tags:` 字段）
2. 在 `relations.yaml` 的 `disease_symptoms` 下添加症状关联
3. 在 `treatment_rules.yaml` 的 `disease_treatment` 下添加药物关联
4. 在 `treatment_rules.yaml` 的 `disease_diagnostic` 下添加诊断关联
5. 在 `treatment_rules.yaml` 的 `disease_treatment_map` 下添加治疗方案关联
6. 如需新症状/药物/检查/标签，先在对应目录新建 `.md` 文件或 `tags.yaml`
7. 运行 `python tools/gen_from_yaml.py`
8. 递增 `SEED_DATA_VERSION`
9. `cargo check` 验证

## 架构

### 后端（Rust）

```
src-tauri/src/
├── lib.rs          # Tauri 启动，注册所有命令，初始化 DB 和推理引擎
├── engine.rs       # 诊断推理引擎（症状→疾病评分算法）
├── db/
│   ├── mod.rs      # 数据库初始化、迁移、种子数据导入
│   └── models.rs   # SQLx 模型结构体
└── commands/
    ├── knowledge.rs    # 知识库查询（疾病/症状/药物/检查/病例 CRUD）
    ├── treatments.rs   # 治疗方案 + 标签系统命令
    ├── diagnose.rs     # 诊断推理命令
    ├── flashcards.rs   # 闪卡系统（SM-2 间隔重复算法）
    ├── search.rs       # 全文搜索
    └── import_export.rs # 数据导入导出
```

**数据库**：SQLite WAL 模式，存储在应用数据目录。迁移通过 `schema_migrations` 表管理，种子数据版本通过 `app_meta` 表追踪。当前迁移版本：v7，种子数据版本：10。

**核心数据表**：
- 实体表：`diseases`, `symptoms`, `drugs`, `diagnostic_tests`, `cases`, `treatments`
- 关系表：`disease_symptom`, `disease_ddx`, `disease_treatment`, `disease_diagnostic`, `disease_treatment_map`, `case_disease`
- 标签系统：`tags`（预置标签）, `entity_tags`（多态关联：entity_type + entity_id + tag_id）

**诊断引擎**：基于症状频率权重（common=1.0, uncommon=0.6, rare=0.3）+ 核心症状 1.5x 加成，计算疾病覆盖度 × 输入覆盖度综合分。

**闪卡系统**：SM-2 算法，quality 0-5 评分，自动计算下次复习间隔。支持从知识库自动生成和手动创建。

**治疗方案模块**：`treatments` 表存储完整治疗协议（含生理基础、预后评估），通过 `disease_treatment_map` 与疾病关联（line: first/second/adjunctive）。

### 前端（Vue 3）

```
src/
├── App.vue           # 布局（侧边栏 + 主内容区）
├── components/
│   ├── Sidebar.vue   # 侧边导航
│   └── TopBar.vue    # 顶部栏
├── views/
│   ├── knowledge/    # 知识库页面
│   │   ├── Home.vue
│   │   ├── DiseaseList.vue / DiseaseDetail.vue（含标签云、急诊徽章、治疗协议）
│   │   ├── SymptomList.vue / SymptomExplorer.vue（症状→疾病反向查找，含标签）
│   │   ├── DrugHandbook.vue（含标签、作用机制、PK/PD展示）
│   │   ├── SymptomChecker.vue（症状推理）
│   │   └── DiseaseCompare.vue（疾病对比）
│   ├── learning/
│   │   ├── CaseLibrary.vue
│   │   └── FlashcardStudy.vue（闪卡复习）
│   ├── graph/        # 知识图谱
│   └── game/         # 诊断游戏
├── router/index.ts
└── types/index.ts    # TypeScript 类型定义（含 Tag, Treatment, EntityTag 等）
```

**前端 API 调用**：统一使用 `invoke('command_name', { params })` 调用 Tauri 命令。

**前端标签展示**：
- DiseaseDetail.vue：急诊等级徽章（颜色区分）、解剖系统标签云、病理机制标签云、DAMNIT-V标签、治疗协议卡片
- SymptomList.vue：标签芯片（按颜色区分 body_system / mechanism）
- DrugHandbook.vue：作用机制标签、详情面板展示 mechanism_of_action / pk_pd / adverse_mechanism

## 代码规范

- 前端：`<script setup lang="ts">`，Composition API
- 后端：Tauri 2.0 命令模式，`Result<T, String>` 错误处理
- YAML：2 空格缩进，列表项 `- key: value`
- 数据库迁移：在 `db/mod.rs` 的 `apply_migrations` 中添加新迁移块

## 注意事项

- 修改数据后必须运行 `gen_from_yaml.py` 并递增 `SEED_DATA_VERSION`
- 新增 Tauri 命令需在 `lib.rs` 的 `invoke_handler!` 中注册
- 新增前端页面需在 `router/index.ts` 添加路由并在 `Sidebar.vue` 添加菜单项
- `drug_051`（马罗匹坦）条目曾出现格式问题，编辑 `data/drugs/drug_051.md` 时注意 frontmatter 格式
- 为现有数据文件添加标签：`python tools/tag_existing_data.py`（仅对没有 tags 的文件写入）
