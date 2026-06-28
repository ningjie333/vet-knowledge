# 错误处理规范（Error Handling Standard）

> 本文档定义 vet-knowledge 项目 Rust 后端与 Vue 前端的错误处理标准模式，消除错误吞没与静默失败。
>
> **设计原则**：错误可见、分类处理、用户可感知、根因可追溯。
>
> **关联文档**：[修复 SOP](./fix_sop.md)、[CI 检查规范](./ci_check_standard.md)、[测试规范](./testing_standard.md)

---

## 1. 错误处理总原则

### 1.1 五项基本原则

| 编号 | 原则 | 说明 |
|------|------|------|
| E-01 | **错误可见** | 禁止静默吞没，至少 `log` 或返回 `Result` |
| E-02 | **分类处理** | 区分可恢复错误与不可恢复错误 |
| E-03 | **用户可感知** | 前端必须反馈，禁止空白或无反应 |
| E-04 | **根因可追溯** | 错误信息含足够上下文定位问题 |
| E-05 | **失败快速** | 早失败早暴露，避免错误传播 |

### 1.2 错误分类

| 类型 | Rust | 处理策略 |
|------|------|----------|
| **可恢复错误** | `Result<T, E>` | 返回错误，由调用方决定 |
| **不可恢复错误** | `panic!` | 仅用于程序不可能继续的状态 |
| **外部错误** | `Result` + 转换 | 用 `?` 传播，用 `map_err` 转换 |
| **输入验证错误** | `Result` | 返回明确的验证错误信息 |

---

## 2. Rust 后端规范

### 2.1 Tauri 命令错误处理

**标准模式**：

```rust
#[tauri::command]
pub async fn get_disease_by_id(
    pool: tauri::State<'_, DbPool>,
    id: String,
) -> Result<Option<Disease>, String> {
    let row = sqlx::query_as::<_, Disease>("SELECT * FROM diseases WHERE id = ?")
        .bind(id)
        .fetch_optional(&*pool)
        .await
        .map_err(|e| format!("查询疾病失败: {}", e))?;

    Ok(row)
}
```

**要点**：
- 所有命令返回 `Result<T, String>`
- 用 `map_err(|e| format!("操作描述: {}", e))` 添加上下文
- 错误信息包含操作描述 + 原始错误

### 2.2 `.ok()` 使用禁令

**禁止模式**：

```rust
// ❌ 禁止：错误被静默吞没
sqlx::query("ALTER TABLE diseases ADD COLUMN foo TEXT")
    .execute(pool).await.ok();

// ❌ 禁止：用 .ok() 规避错误处理
let result: Option<T> = some_operation().await.ok();
```

**允许的少数场景**（仅限幂等性处理）：

```rust
// ✅ 允许：幂等迁移，配合 IF NOT EXISTS 让 SQL 本身幂等
sqlx::query("ALTER TABLE diseases ADD COLUMN foo TEXT")
    .execute(pool).await
    .map_err(|e| {
        // 仅容忍"列已存在"错误，其他错误必须传播
        if e.to_string().contains("duplicate column") {
            Ok(())
        } else {
            Err(format!("迁移失败: {}", e))
        }
    })?;
```

**推荐模式**：

```rust
// ✅ 推荐：用 IF NOT EXISTS 让 SQL 幂等
sqlx::query("ALTER TABLE diseases ADD COLUMN foo TEXT")
    .execute(pool).await
    .map_err(|e| format!("迁移失败: {}", e))?;
```

### 2.3 `unwrap()` 使用规范

**禁止场景**（生产代码）：

```rust
// ❌ 禁止：排序时 unwrap
b.match_score.partial_cmp(&a.match_score).unwrap()

// ❌ 禁止：数据库查询结果 unwrap
let row: Disease = query.unwrap();

// ❌ 禁止：解析结果 unwrap
let parsed: serde_json::Value = serde_json::from_str(&s).unwrap();
```

**允许场景**（仅限以下情况）：

```rust
// ✅ 允许：常量/字面量，编译期可证明安全
let pi: f64 = 3.14159;
assert!(pi > 0.0);  // 字面量必然为正

// ✅ 允许：测试代码中（失败即测试失败）
#[test]
fn test_something() {
    let result = some_func().unwrap();
    assert_eq!(result, expected);
}

// ✅ 允许：程序启动时配置加载（无法继续则 panic 合理）
let config = load_config().expect("配置文件缺失，无法启动");
```

**替代方案**：

```rust
// 排序：用 unwrap_or 提供默认值
b.match_score.partial_cmp(&a.match_score).unwrap_or(Ordering::Equal)

// 或用 total_cmp（Rust 1.62+，推荐）
b.match_score.total_cmp(&a.match_score)

// 查询：用 fetch_optional 返回 Option
let row: Option<Disease> = query.fetch_optional(pool).await?;

// 解析：用 ? 传播错误
let parsed: serde_json::Value = serde_json::from_str(&s)?;
```

### 2.4 `panic!` 使用规范

**禁止场景**：

```rust
// ❌ 禁止：用户可见的错误用 panic
pub fn run() {
    let db = db::init(&app).expect("DB init failed");  // 用户看到闪退
}
```

**允许场景**：

```rust
// ✅ 允许：程序启动时无法恢复的状态（但应提供错误信息）
// 用 expect 提供上下文
let db = db::init(&app).expect("数据库初始化失败，请检查 app_data_dir 权限");

// ✅ 允许：不可能到达的分支
match status {
    Status::Active => { ... }
    Status::Inactive => { ... }
    _ => unreachable!("未知状态: {:?}", status),  // 程序逻辑错误
}
```

**推荐改进**（DB 初始化失败）：

```rust
// ✅ 推荐：用 dialog 弹窗提示，而非 panic
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let db = match db::init(app.handle()) {
                Ok(pool) => pool,
                Err(e) => {
                    tauri::api::dialog::message(
                        Some(app.get_window_window().unwrap()),
                        "启动失败",
                        format!("数据库初始化失败: {}\n\n请删除 %APPDATA%\\com.vetknowledge.app 后重启", e)
                    );
                    app.exit(1);
                    return Ok(());
                }
            };
            app.manage(db);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Tauri 启动失败");
}
```

### 2.5 错误上下文规范

**错误信息必须包含**：

```rust
// ❌ 错误：无上下文
.map_err(|e| e.to_string())?;

// ✅ 正确：含操作 + 实体 + 原始错误
.map_err(|e| format!("查询疾病(id={})失败: {}", id, e))?;

// ✅ 正确：含操作 + SQL 上下文
.map_err(|e| format!("执行迁移 v{} 失败: {}", version, e))?;
```

**错误信息模板**：
```
<操作描述>(<关键参数>): <原始错误>
```

示例：
- `查询疾病(id=dis_001)失败: no such table: diseases`
- `执行迁移 v3(重建 disease_diagnostic)失败: duplicate column name: species`
- `导入种子数据失败: SQL 解析错误在第 42 行: ...`

### 2.6 迁移错误处理规范

**当前问题**：`db/mod.rs` 中 34 处 `.ok()` 吞没迁移错误。

**修复模式**：

```rust
// ❌ 当前：错误被吞没
sqlx::query("ALTER TABLE diseases ADD COLUMN name_latin TEXT")
    .execute(pool).await.ok();

// ✅ 修复方式一：用 IF NOT EXISTS（首选）
sqlx::query("ALTER TABLE diseases ADD COLUMN name_latin TEXT")
    .execute(pool).await
    .map_err(|e| format!("迁移 v2(diseases.name_latin)失败: {}", e))?;

// ✅ 修复方式二：容忍特定错误（当 SQL 不支持 IF NOT EXISTS 时）
sqlx::query("CREATE INDEX idx_xxx ON ...")
    .execute(pool).await
    .map_err(|e| -> anyhow::Result<()> {
        if e.to_string().contains("already exists") {
            Ok(())  // 幂等，索引已存在
        } else {
            Err(anyhow::anyhow!("迁移 v3(idx_xxx)失败: {}", e))
        }
    })?;
```

---

## 3. Vue 前端规范

### 3.1 invoke 错误处理

**标准模式**：

```typescript
// ✅ 标准：try/catch + 用户反馈
async function loadDiseases() {
  loading.value = true
  try {
    diseases.value = await invoke<Disease[]>('get_diseases', { species: null, category: null })
  } catch (e) {
    showError('加载疾病列表失败', e)
  } finally {
    loading.value = false
  }
}
```

**禁止模式**：

```typescript
// ❌ 禁止：静默 catch
try {
  results.value = await invoke('infer_diagnosis', {...})
} catch (e) { console.error(e) }  // 用户看到空白

// ❌ 禁止：无 catch
diseases.value = await invoke('get_diseases')  // 异常未处理

// ❌ 禁止：只 console
catch (e) { console.error(e) /* 无 UI 反馈 */ }
```

### 3.2 错误反馈机制

**统一错误提示组件**（建议封装）：

```typescript
// src/utils/notify.ts
const notifications = ref<Notification[]>([])

export function showError(title: string, error: unknown) {
  const message = error instanceof Error ? error.message : String(error)
  notifications.value.push({
    type: 'error',
    title,
    message,
    timestamp: Date.now()
  })
  // 自动消失
  setTimeout(() => notifications.value.shift(), 5000)
}

export function showSuccess(title: string) {
  notifications.value.push({ type: 'success', title, timestamp: Date.now() })
  setTimeout(() => notifications.value.shift(), 3000)
}
```

**使用示例**：

```typescript
import { showError, showSuccess } from '@/utils/notify'

async function reviewCard(quality: number) {
  try {
    await invoke('review_flashcard', { cardId: currentCard.value?.id, quality })
    showSuccess('复习记录已保存')
  } catch (e) {
    showError('复习提交失败', e)
  }
}
```

### 3.3 加载状态规范

**所有异步操作必须有 loading 状态**：

```typescript
const loading = ref(false)
const error = ref<string | null>(null)

async function loadData() {
  loading.value = true
  error.value = null
  try {
    data.value = await invoke('...')
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e)
  } finally {
    loading.value = false
  }
}
```

**模板中必须处理三种状态**：

```vue
<template>
  <div v-if="loading" class="loading">加载中...</div>
  <div v-else-if="error" class="error">
    加载失败：{{ error }}
    <button @click="loadData">重试</button>
  </div>
  <div v-else>
    <!-- 正常内容 -->
  </div>
</template>
```

### 3.4 Promise.all 错误处理

**当前问题**：多个页面用 `Promise.all` 并发拉取，一个失败导致全部失败。

**改进模式**：

```typescript
// ✅ 用 Promise.allSettled 隔离失败
const [diseaseRes, symptomsRes, tagsRes] = await Promise.allSettled([
  invoke('get_disease_by_id', { id }),
  invoke('get_disease_symptoms', { diseaseId }),
  invoke('get_tags')
])

if (diseaseRes.status === 'fulfilled') {
  disease.value = diseaseRes.value
} else {
  showError('加载疾病详情失败', diseaseRes.reason)
}

if (symptomsRes.status === 'fulfilled') {
  symptoms.value = symptomsRes.value
} else {
  showError('加载症状失败', symptomsRes.reason)
}
```

---

## 4. 错误日志规范

### 4.1 Rust 日志

**当前状态**：项目未集成 `log` crate，仅 `db::write_import_error_log` 写文件。

**建议引入**：

```rust
// Cargo.toml
[dependencies]
log = "0.4"
env_logger = "0.11"

// main.rs
env_logger::init();

// 使用
log::error!("数据库初始化失败: {}", e);
log::warn!("迁移 v{} 跳过（已存在）: {}", version, e);
log::info!("种子数据导入完成，版本: {}", version);
log::debug!("查询疾病(id={})返回 {} 行", id, rows.len());
```

### 4.2 前端日志

```typescript
// ✅ 正确：分级日志
console.error('加载失败:', error)     // 错误
console.warn('字段缺失:', field)      // 警告
console.info('操作完成:', action)     // 信息
console.debug('调试:', detail)        // 调试

// ❌ 错误：用 console.log 代替 error
console.log('error:', e)
```

---

## 5. 错误处理审查清单

代码审查时逐项确认：

### Rust 后端
- [ ] 无新增 `.ok()` 吞没错误（违反 E-01）
- [ ] 无新增裸 `unwrap()` 在生产代码（违反 E-05）
- [ ] Tauri 命令返回 `Result<T, String>`
- [ ] `map_err` 错误信息含操作描述 + 关键参数
- [ ] 迁移块用 `IF NOT EXISTS` 或精确错误匹配，非 `.ok()`
- [ ] `panic!` 仅用于程序不可能继续的状态
- [ ] 启动失败有用户可见的错误提示（dialog 或日志文件）

### Vue 前端
- [ ] 所有 `invoke` 调用有 try/catch
- [ ] catch 块有用户可见的反馈（非仅 console）
- [ ] 异步操作有 loading 状态
- [ ] 错误状态有 UI 展示 + 重试按钮
- [ ] `Promise.all` 改用 `Promise.allSettled` 隔离失败
- [ ] 错误信息使用分级日志（error/warn/info）

---

## 6. 禁止行为清单

| 编号 | 禁止行为 | 理由 | 替代方案 |
|------|----------|------|----------|
| E-01 | `.ok()` 吞没错误 | 掩盖根因 | `map_err` + 传播 |
| E-02 | 裸 `unwrap()` 生产代码 | panic 风险 | `unwrap_or` / `?` / `expect` |
| E-03 | 静默 `catch {}` | 用户无反馈 | `showError(title, e)` |
| E-04 | `catch` 仅 `console.error` | 用户无感知 | UI 反馈 + console |
| E-05 | 无上下文错误信息 | 难以定位 | `format!("操作(参数): {}", e)` |
| E-06 | `Promise.all` 全失败 | 一个失败全部失败 | `Promise.allSettled` |
| E-07 | 无 loading 状态 | 用户以为卡死 | `loading.value` + UI |
| E-08 | `panic!` 用户可见错误 | 应用闪退 | dialog 提示 |
| E-09 | 关闭检查器规避错误 | 治标不治本 | 修复误报后保持开启 |
| E-10 | `console.log` 代替 `error` | 日志级别混乱 | `console.error` |

---

## 7. 相关文档

- [修复 SOP](./fix_sop.md)
- [CI 检查规范](./ci_check_standard.md)
- [测试规范](./testing_standard.md)
- [静态分析规则](../rules/static_analysis_rules.md)
