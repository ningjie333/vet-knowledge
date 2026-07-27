/// SQL 查询构造器
///
/// 解决动态 WHERE / IN 子句的字符串拼接问题。
/// 返回 SQL 字符串，调用方负责 bind 参数。
///
/// # 示例
/// ```
/// let sql = SqlBuilder::select("SELECT * FROM diseases")
///     .optional_where(Some("species LIKE ?"))
///     .and_where("is_deleted = 0")
///     .and_in("id", 3)
///     .order_by("name_zh")
///     .build();
/// // => "SELECT * FROM diseases WHERE species LIKE ? AND is_deleted = 0 AND id IN (?,?,?) ORDER BY name_zh"
/// ```
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

    /// 加入固定 WHERE 条件
    pub fn and_where(mut self, cond: impl Into<String>) -> Self {
        self.conditions.push(cond.into());
        self
    }

    /// 加入 IN 子句，自动生成 "col IN (?,?,?)"，count 个占位符
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
        std::iter::repeat("?").take(count).collect::<Vec<_>>().join(",")
    }
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
}
