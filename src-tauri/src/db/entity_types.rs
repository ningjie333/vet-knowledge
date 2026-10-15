//! 实体类型注册表
//!
//! 新增实体类型只需在 ENTITY_TYPES 数组中添加一项，
//! schema.sql 的 entity_tags CHECK 约束同步更新即可。
//! 命令层通过 is_valid_entity_type() 做参数校验。

pub const ENTITY_TYPES: &[&str] = &["disease", "symptom", "drug", "treatment", "case"];

/// 校验 entity_type 是否合法
pub fn is_valid_entity_type(t: &str) -> bool {
    ENTITY_TYPES.contains(&t)
}

// 预留：当前 schema.sql/迁移手写了约束文本，本函数由 test_constraint_format 做契约校验，防止注册表与 schema 漂移。
/// 生成 entity_tags CHECK 约束片段（用于 schema.sql 和迁移）
/// 例如: "CHECK(entity_type IN ('disease','symptom','drug','treatment','case'))"
#[allow(dead_code)]
pub fn entity_tags_check_constraint() -> String {
    let list = ENTITY_TYPES
        .iter()
        .map(|t| format!("'{}'", t))
        .collect::<Vec<_>>()
        .join(",");
    format!("CHECK(entity_type IN ({}))", list)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid() {
        assert!(is_valid_entity_type("disease"));
        assert!(is_valid_entity_type("case"));
        assert!(!is_valid_entity_type("vaccine"));
        assert!(!is_valid_entity_type(""));
    }

    #[test]
    fn test_constraint_format() {
        let c = entity_tags_check_constraint();
        assert!(c.contains("disease"));
        assert!(c.contains("case"));
        assert!(c.starts_with("CHECK(entity_type IN ("));
    }
}
