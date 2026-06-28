# 测试规范（Testing Standard）

> 本文档定义 vet-knowledge 项目的测试分层、覆盖率要求、命名约定与编写规范。
>
> **设计原则**：测试先行、核心优先、边界覆盖、可重复执行。
>
> **关联文档**：[修复 SOP](./fix_sop.md)、[CI 检查规范](./ci_check_standard.md)、[错误处理规范](./error_handling_standard.md)

---

## 1. 测试分层

### 1.1 四层测试体系

| 层级 | 范围 | 工具 | 执行时机 | 覆盖目标 |
|------|------|------|----------|----------|
| **单元测试** | 单个函数/模块 | Rust `#[test]`、Python `pytest` | 每次提交 | 核心算法 100%、其他 ≥60% |
| **集成测试** | 跨模块协作 | Rust `#[test]` + 临时 SQLite | 阶段交付前 | 关键流程 100% |
| **类型检查** | 前后端类型一致性 | `vue-tsc --noEmit`、`check.py type_check` | 每次提交 | 零类型错误 |
| **静态检查** | 跨文件一致性 | `check.py --full` | 每次提交 | 6 规则全通过 |

### 1.2 测试优先级

| 优先级 | 模块 | 测试要求 |
|--------|------|----------|
| **P0 必测** | `engine.rs`（推理引擎） | 必须单元测试，覆盖率 ≥95% |
| **P0 必测** | `flashcards.rs`（SM-2 算法） | 必须单元测试，覆盖率 ≥95% |
| **P0 必测** | `db/mod.rs`（SQL 分割、迁移） | 必须单元测试，覆盖率 ≥80% |
| **P1 应测** | `commands/diagnose.rs` | 集成测试覆盖主要场景 |
| **P1 应测** | `commands/knowledge.rs` | 集成测试覆盖 CRUD |
| **P1 应测** | `tools/gen_from_yaml.py` | 冒烟测试（生成的 SQL 可被 SQLite 解析） |
| **P2 可测** | 其他 commands | 视情况补测 |
| **P3 免测** | 前端视图层 | 暂不强制，优先保证后端 |

---

## 2. Rust 测试规范

### 2.1 测试组织

```rust
// 在被测模块文件末尾添加测试模块
#[cfg(test)]
mod tests {
    use super::*;

    // === 测试用例分组 ===

    // --- 正常路径 ---
    #[test]
    fn test_infer_basic_match() { ... }

    // --- 边界条件 ---
    #[test]
    fn test_infer_empty_input() { ... }

    // --- 错误路径 ---
    #[test]
    fn test_infer_no_matching_disease() { ... }

    // --- 算法特定 ---
    #[test]
    fn test_infer_pathognomonic_bonus() { ... }
}
```

### 2.2 命名约定

**测试函数命名**：`test_<被测函数>_<场景>`

| 命名模式 | 示例 | 说明 |
|----------|------|------|
| `test_<func>_basic` | `test_infer_basic_match` | 正常路径 |
| `test_<func>_empty_<input>` | `test_infer_empty_input` | 空输入边界 |
| `test_<func>_single_<item>` | `test_infer_single_disease` | 单元素边界 |
| `test_<func>_boundary_<case>` | `test_review_quality_boundary_zero` | 边界值 |
| `test_<func>_no_<scenario>` | `test_infer_no_matching_disease` | 无匹配场景 |
| `test_<func>_<algorithm_feature>` | `test_infer_pathognomonic_bonus` | 算法特性 |
| `test_<func>_<error_condition>` | `test_split_sql_semicolon_in_string` | 错误条件 |

### 2.3 测试用例结构（AAA 模式）

```rust
#[test]
fn test_infer_basic_match() {
    // Arrange（准备）
    let input = DiagnosisInput {
        symptoms: vec!["咳嗽".to_string(), "发热".to_string()],
        species: "犬".to_string(),
        age: None,
        breed: None,
    };
    let disease_list = vec![("dis_001".to_string(), "肺炎".to_string())];
    let mut all_disease_symptoms = HashMap::new();
    all_disease_symptoms.insert(
        "dis_001".to_string(),
        vec![
            ("咳嗽".to_string(), "common".to_string(), 0),
            ("发热".to_string(), "common".to_string(), 0),
        ],
    );
    let diagnostics = HashMap::new();

    // Act（执行）
    let result = infer(&input, &disease_list, &all_disease_symptoms, &diagnostics);

    // Assert（断言）
    assert!(!result.is_empty());
    assert_eq!(result[0].disease_id, "dis_001");
    assert!(result[0].match_score > 0.5);
}
```

### 2.4 断言规范

| 断言类型 | 使用场景 | 示例 |
|----------|----------|------|
| `assert_eq!` | 精确相等 | `assert_eq!(result.len(), 3)` |
| `assert!` | 布尔条件 | `assert!(result.is_empty())` |
| `assert_ne!` | 不相等 | `assert_ne!(result[0].match_score, 0.0)` |
| `assert!(... > ...)` | 范围比较 | `assert!(score > 0.5 && score <= 1.0)` |
| `assert!(result.contains(...))` | 集合包含 | `assert!(matched.contains("咳嗽"))` |

**浮点数比较**：
```rust
// ❌ 错误：浮点直接比较
assert_eq!(score, 0.666);

// ✅ 正确：使用近似比较
assert!((score - 0.666).abs() < 0.001);
```

### 2.5 测试数据构造

**原则**：测试数据自包含，不依赖外部数据库或文件。

```rust
// ❌ 错误：依赖真实数据库
let pool = connect_to_real_db().await;
let result = get_diseases(pool).await;

// ✅ 正确：构造内存数据
let disease_list = vec![
    ("dis_001".to_string(), "肺炎".to_string()),
    ("dis_002".to_string(), "支气管炎".to_string()),
];
```

**复杂测试数据可提取辅助函数**：
```rust
fn make_test_disease(id: &str, name: &str) -> (String, String) {
    (id.to_string(), name.to_string())
}

fn make_test_symptoms(symptoms: &[(&str, &str)]) -> Vec<(String, String, i64)> {
    symptoms.iter().map(|(s, f)| (s.to_string(), f.to_string(), 0)).collect()
}
```

---

## 3. 核心模块测试要求

### 3.1 `engine.rs` 诊断推理引擎（P0）

**必须覆盖的场景**：

| 场景 | 验证点 | 用例数 |
|------|--------|--------|
| 基本匹配 | 输入症状匹配疾病，返回正确候选 | 2 |
| 频率权重 | common/uncommon/rare 不同权重计算 | 3 |
| 核心症状加成 | is_pathognomonic=1 时 1.5× 加成 | 2 |
| 双覆盖度 | disease_coverage × input_coverage | 2 |
| 阈值过滤 | score < 0.25 或 input_coverage < 0.3 丢弃 | 2 |
| 排序截断 | 按 match_score 降序，取前 10 | 2 |
| 空输入 | 症状列表为空时返回空 | 1 |
| 无匹配 | 所有疾病都不匹配时返回空 | 1 |
| 单疾病 | disease_list 只有一个元素 | 1 |
| 诊断检查附加 | suggested_tests 正确填充 | 1 |
| 物种过滤 | species 不匹配的疾病被排除（前置过滤） | 1 |

**目标覆盖率**：≥95%

### 3.2 `flashcards.rs` SM-2 算法（P0）

**必须覆盖的场景**：

| 场景 | 验证点 | 用例数 |
|------|--------|--------|
| 首次复习 quality=0 | interval=1, EF 降低 | 1 |
| 首次复习 quality=3 | interval=1, EF 降低较少 | 1 |
| 首次复习 quality=5 | interval=6, EF 增加 | 1 |
| 非首次 quality<3 | interval 重置为 1, difficulty=0.8 | 1 |
| 非首次 quality=3 | 间隔 <1.5 时 interval=6 | 1 |
| 非首次 quality=5 | interval = old × EF | 1 |
| EF 下限保护 | EF 不低于 1.3 | 1 |
| EF 上限 | EF 增长合理 | 1 |
| quality=2 边界 | quality<3 触发重置 | 1 |
| quality=3 边界 | quality>=3 不重置 | 1 |
| 统计查询 | get_review_stats 正确计算 mastered | 1 |
| 闪卡生成 | generate_flashcards 跳过已存在 | 1 |

**目标覆盖率**：≥95%

### 3.3 `db/mod.rs` SQL 处理（P0）

**必须覆盖的场景**：

| 场景 | 验证点 | 用例数 |
|------|--------|--------|
| 基础分割 | 多语句正确分割 | 1 |
| 引号内分号 | 不在引号内分割 | 1 |
| 转义引号 | `''` 和 `""` 正确处理 | 1 |
| 注释剥离 | `--` 注释被移除 | 1 |
| 空行处理 | 空行和纯空白行被忽略 | 1 |
| 尾部无分号 | 最后一条语句无分号也能处理 | 1 |

**目标覆盖率**：≥80%

---

## 4. Python 测试规范

### 4.1 测试组织

```
tools/
├── gen_from_yaml.py
├── test_gen_from_yaml.py     # 测试文件与被测文件同目录
└── conftest.py               # pytest 配置
```

### 4.2 命名约定

- 测试文件：`test_<被测模块>.py`
- 测试函数：`test_<功能>_<场景>`
- 测试类：`Test<模块名>`

### 4.3 冒烟测试要求（`gen_from_yaml.py`）

```python
# tools/test_gen_from_yaml.py
import sqlite3
import subprocess

def test_generated_sql_is_valid():
    """生成的 SQL 必须能被 SQLite 解析"""
    # Act
    result = subprocess.run(
        ["python", "tools/gen_from_yaml.py"],
        capture_output=True, text=True
    )
    assert result.returncode == 0

    # 用内存 SQLite 执行生成的 SQL
    conn = sqlite3.connect(":memory:")
    with open("src-tauri/data/seed/schema.sql") as f:
        conn.executescript(f.read())
    with open("src-tauri/data/seed/001_initial.sql") as f:
        conn.executescript(f.read())
    conn.close()
```

---

## 5. 测试执行

### 5.1 本地执行

```bash
# Rust 测试
cd src-tauri && cargo test

# Rust 测试（带输出）
cd src-tauri && cargo test -- --nocapture

# 单个测试模块
cd src-tauri && cargo test engine::tests

# 单个测试函数
cd src-tauri && cargo test test_infer_basic_match

# Python 测试
pytest tools/ -v
```

### 5.2 测试覆盖率

```bash
# Rust 覆盖率（需安装 cargo-tarpaulin）
cd src-tauri && cargo tarpaulin --out Html --engine units

# Python 覆盖率
pytest tools/ --cov=tools --cov-report=html
```

### 5.3 测试在 CI 中的位置

测试执行是 [CI 检查规范](./ci_check_standard.md) 闭环验证的必经步骤：

```
check.py --full  →  cargo test  →  vue-tsc --noEmit  →  cargo check  →  闭环完成
```

---

## 6. 测试用例审查清单

提交包含测试的 PR 前，逐项确认：

- [ ] 测试函数命名遵循 `test_<func>_<scenario>` 约定
- [ ] 每个测试用例遵循 AAA 模式（Arrange/Act/Assert）
- [ ] 正常路径、边界条件、错误路径均有覆盖
- [ ] 测试数据自包含，不依赖外部状态
- [ ] 测试可独立运行，不依赖其他测试的执行顺序
- [ ] 断言信息清晰，失败时能快速定位
- [ ] 测试执行时间合理（单个测试 <1s）
- [ ] 无 `unwrap()` 在测试代码外的生产代码中
- [ ] 测试覆盖了所有分支（if/else/match）
- [ ] 边界值测试覆盖（0、空、单元素、最大值）

---

## 7. 禁止行为

| 编号 | 禁止行为 | 理由 |
|------|----------|------|
| T-01 | 禁止测试依赖真实数据库 | 测试不可重复 |
| T-02 | 禁止测试间存在依赖顺序 | 执行顺序变化会失败 |
| T-03 | 禁止用 `unwrap()` 替代断言 | 失败信息不清晰 |
| T-04 | 禁止浮点数直接相等比较 | 精度问题导致 flaky |
| T-05 | 禁止测试函数无断言 | 等于没测试 |
| T-06 | 禁止跳过失败的测试（`#[ignore]`）而不记录原因 | 问题被掩盖 |
| T-07 | 禁止先修复后补测试 | 测试无法证明 Bug 存在 |
| T-08 | 禁止测试数据包含真实用户数据 | 隐私风险 |

---

## 8. 相关文档

- [修复 SOP](./fix_sop.md)
- [CI 检查规范](./ci_check_standard.md)
- [错误处理规范](./error_handling_standard.md)
- [静态分析规则](../rules/static_analysis_rules.md)
