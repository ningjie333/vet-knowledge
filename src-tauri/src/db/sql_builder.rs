/// SQL 查询构造器
///
/// 解决动态 WHERE / IN 子句的字符串拼接问题。
/// 返回 SQL 字符串，调用方负责 bind 参数。
///
/// # 示例
/// ```text
/// let sql = SqlBuilder::select("SELECT * FROM diseases")
///     .optional_where(Some("species LIKE ?"))
///     .and_where("is_deleted = 0")
///     .and_in("id", 3)
///     .order_by("name_zh")
///     .build();
/// // => "SELECT * FROM diseases WHERE species LIKE ? AND is_deleted = 0 AND id IN (?,?,?) ORDER BY name_zh"
/// ```
/// （示例用 text 围栏：本 crate 的 `db` 模块是私有的，doc-test 无法 import。）
pub struct SqlBuilder {
    base: String,
    conditions: Vec<String>,
    order_by: Option<String>,
    limit: Option<i64>,
}

impl SqlBuilder {
    pub fn select(base: impl Into<String>) -> Self {
        Self {
            base: base.into(),
            conditions: Vec::new(),
            order_by: None,
            limit: None,
        }
    }

    /// 仅当 cond 非空时加入 WHERE 条件
    pub fn optional_where(mut self, cond: Option<impl Into<String>>) -> Self {
        if let Some(c) = cond {
            self.conditions.push(c.into());
        }
        self
    }

    // 预留：有意的构建器 API 面（生产查询暂未用到，由 test_multiple_conditions 测试覆盖），后续动态查询直接可用。
    /// 加入固定 WHERE 条件
    #[allow(dead_code)]
    pub fn and_where(mut self, cond: impl Into<String>) -> Self {
        self.conditions.push(cond.into());
        self
    }

    // 预留：有意的构建器 API 面（生产查询暂未用到，由 test_and_in* 测试覆盖），后续批量 id 查询直接可用。
    /// 加入 IN 子句，自动生成 "col IN (?,?,?)"，count 个占位符
    #[allow(dead_code)]
    pub fn and_in(mut self, col: &str, count: usize) -> Self {
        if count > 0 {
            self.conditions.push(format!("{} IN ({})", col, placeholders(count)));
        }
        self
    }

    pub fn order_by(mut self, clause: impl Into<String>) -> Self {
        self.order_by = Some(clause.into());
        self
    }

    // 预留：有意的构建器 API 面（生产查询暂未用到，由 test_multiple_conditions 测试覆盖），后续分页/限流查询直接可用。
    #[allow(dead_code)]
    pub fn limit(mut self, n: i64) -> Self {
        self.limit = Some(n);
        self
    }

    pub fn build(self) -> String {
        let mut sql = self.base;
        if !self.conditions.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&self.conditions.join(" AND "));
        }
        if let Some(ob) = self.order_by {
            sql.push_str(" ORDER BY ");
            sql.push_str(&ob);
        }
        if let Some(l) = self.limit {
            sql.push_str(&format!(" LIMIT {}", l));
        }
        sql
    }
}

/// 生成 count 个 "?" 占位符，用逗号分隔
/// count=0 → ""，count=3 → "?,?,?"
pub fn placeholders(count: usize) -> String {
    if count == 0 {
        String::new()
    } else {
        std::iter::repeat_n("?", count).collect::<Vec<_>>().join(",")
    }
}

/// entity_tags 关联子查询：把某个实体的**标签 id** 聚合成 JSON 数组文本，列别名固定为 `tags`。
///
/// 与 `models::row_tags()` / `models::parse_tags_json()` 配套使用：
/// SQL 只负责产出 JSON 文本，Rust 侧统一解析成 `Vec<String>`，
/// 序列化给前端就是 `tags: string[]`（而不是逗号分隔字符串）。
///
/// 为什么下发 id 而不是 tags.name_zh：前端拿到后要回查 `get_tags()` 建的映射表去取
/// color / tag_group / 急诊等级（见 DrugHandbook.vue 的 mechanismTags 与 tag.color、
/// DiseaseDetail.vue 的 tagMap.get(tagId)）。下发名字会让那次回查永远落空，
/// 标签区依旧空白 —— 这是本次修复里踩到并纠正的契约错位。
///
/// 排序与 `commands::treatments::get_entity_tags` 保持一致：`tag_group, name_zh`。
/// 实体没有任何标签时，`json_group_array` 返回 `[]`（不是 NULL）。
///
/// # 参数
/// - `row_alias`：主表在查询里的别名，其主键列必须叫 `id`（如 `"d"` → `d.id`）
/// - `entity_type`：必须来自 `entity_types::ENTITY_TYPES`，调用点全部是代码里的字面量。
///   debug 构建下非法值直接 panic；release 下退化成"查不到行 → 空标签"。
///   刻意不返回空字符串：空串会拼出 `SELECT d.*,  FROM ...` 这种语法错误，把 programmer
///   error 推给运行期的数据库。
///
/// # 示例（同样是 text 围栏）
/// ```text
/// let sel = tags_subquery("d", "disease");
/// // => "(SELECT json_group_array(tag_id) FROM (SELECT tg.id AS tag_id
/// //     FROM entity_tags et JOIN tags tg ON tg.id = et.tag_id
/// //     WHERE et.entity_type = 'disease' AND et.entity_id = d.id
/// //     ORDER BY tg.tag_group, tg.name_zh)) AS tags"
/// let sql = format!("SELECT d.*, {} FROM diseases d", sel);
/// ```
pub fn tags_subquery(row_alias: &str, entity_type: &str) -> String {
    debug_assert!(
        crate::db::entity_types::is_valid_entity_type(entity_type),
        "tags_subquery 收到未注册的 entity_type: {}",
        entity_type
    );
    format!(
        "(SELECT json_group_array(tag_id) FROM \
         (SELECT tg.id AS tag_id FROM entity_tags et JOIN tags tg ON tg.id = et.tag_id \
          WHERE et.entity_type = '{}' AND et.entity_id = {}.id \
          ORDER BY tg.tag_group, tg.name_zh)) AS tags",
        entity_type, row_alias
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_all() {
        let sql = SqlBuilder::select("SELECT * FROM diseases").build();
        assert_eq!(sql, "SELECT * FROM diseases");
    }

    #[test]
    fn test_optional_where_some() {
        let sql = SqlBuilder::select("SELECT * FROM diseases")
            .optional_where(Some("species LIKE ?"))
            .build();
        assert_eq!(sql, "SELECT * FROM diseases WHERE species LIKE ?");
    }

    #[test]
    fn test_optional_where_none() {
        let sql = SqlBuilder::select("SELECT * FROM diseases")
            .optional_where(None::<&str>)
            .build();
        assert_eq!(sql, "SELECT * FROM diseases");
    }

    #[test]
    fn test_multiple_conditions() {
        let sql = SqlBuilder::select("SELECT * FROM diseases")
            .and_where("is_deleted = 0")
            .and_where("urgency_level >= 3")
            .order_by("name_zh")
            .limit(10)
            .build();
        assert_eq!(
            sql,
            "SELECT * FROM diseases WHERE is_deleted = 0 AND urgency_level >= 3 ORDER BY name_zh LIMIT 10"
        );
    }

    #[test]
    fn test_and_in() {
        let sql = SqlBuilder::select("SELECT * FROM diseases")
            .and_in("id", 3)
            .build();
        assert_eq!(sql, "SELECT * FROM diseases WHERE id IN (?,?,?)");
    }

    #[test]
    fn test_and_in_zero() {
        let sql = SqlBuilder::select("SELECT * FROM diseases")
            .and_in("id", 0)
            .build();
        assert_eq!(sql, "SELECT * FROM diseases");
    }

    #[test]
    fn test_placeholders() {
        assert_eq!(placeholders(0), "");
        assert_eq!(placeholders(1), "?");
        assert_eq!(placeholders(3), "?,?,?");
    }

    // ── tags_subquery：标签链路（entity_tags → tags.id → JSON 文本） ──

    #[test]
    fn test_tags_subquery_all_entity_types() {
        // 四类可打标签的实体都要能拼出子查询（历史 bug：只有 disease 走通）
        for (alias, entity_type) in [
            ("d", "disease"),
            ("s", "symptom"),
            ("dr", "drug"),
            ("t", "treatment"),
            ("c", "case"),
        ] {
            let sel = tags_subquery(alias, entity_type);
            assert!(sel.contains(&format!("et.entity_type = '{}'", entity_type)));
            assert!(sel.contains(&format!("et.entity_id = {}.id", alias)));
            assert!(sel.ends_with(") AS tags"), "{} → {}", entity_type, sel);
            assert!(sel.contains("json_group_array"));
            // 前端契约：下发 tags.id 供回查 color/tag_group，绝不能是 name_zh
            assert!(
                sel.contains("json_group_array(tag_id)") && sel.contains("tg.id AS tag_id"),
                "{} → {} 必须聚合标签 id",
                entity_type,
                sel
            );
            assert!(
                !sel.contains("json_group_array(name_zh)"),
                "{} → {} 聚合了显示名，前端 tagMap.get() 会永远落空",
                entity_type,
                sel
            );
        }
    }

    #[test]
    #[should_panic(expected = "未注册的 entity_type")]
    fn test_tags_subquery_panics_on_unknown_entity_type() {
        // 自创命名会撞上 schema.sql 的 entity_tags CHECK 约束，debug 构建下立刻炸
        tags_subquery("d", "vaccine");
    }

    #[test]
    fn test_tags_subquery_composes_with_builder() {
        let sql = SqlBuilder::select(format!(
            "SELECT dr.*, {} FROM drugs dr",
            tags_subquery("dr", "drug")
        ))
        .optional_where(Some("drug_class LIKE ?"))
        .order_by("name_zh")
        .build();
        assert!(sql.starts_with("SELECT dr.*, (SELECT json_group_array"));
        assert!(sql.contains("FROM drugs dr WHERE drug_class LIKE ? ORDER BY name_zh"));
    }
}
