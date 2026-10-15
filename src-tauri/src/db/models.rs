use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqliteRow;
use sqlx::FromRow;
use sqlx::Row;

// ── Row 构造函数 ──
// 从 SqliteRow 手动构造实体，用于 JOIN 查询（query() 而非 query_as）。
// 统一放在 models.rs，避免各 commands 模块重复定义。
//
// tags 字段的约定：
//   查询用 sql_builder::tags_subquery() 附加一个关联子查询，
//   把 entity_tags → tags.id 聚合为 JSON 数组文本，列别名固定为 `tags`；
//   这里由 row_tags() 统一解析成 Vec<String>（序列化为 JSON 数组，前端 tags: string[]）。
//   元素是标签 id 而非显示名：前端要拿它回查 get_tags() 取 color / tag_group。
//   没有 tags 列的查询（历史 SQL / `SELECT *`）回退成空数组，不报错。
//
// 为什么 Disease/Symptom/Drug/Treatment 手写 FromRow 而不用 #[derive(FromRow)]：
//   SQLite 没有数组类型，`Vec<String>` 不满足 sqlx 的 Decode，derive 版无法编译；
//   手写 impl 直接复用下面的 row_from_*()，让 query_as 与手动映射走同一条解析路径。

/// 把 json_group_array 产出的 JSON 文本解析为 Vec<String>。
///
/// - `None` / 空串 / `[]` → 空 Vec
/// - 数组里的 `null`（标签 id 缺失）→ 跳过
/// - 非数组或非法 JSON → 空 Vec 并在 stderr 记录（依据 E-08：不允许完全静默吞错）
pub fn parse_tags_json(raw: Option<&str>) -> Vec<String> {
    let Some(text) = raw else {
        return Vec::new();
    };
    let text = text.trim();
    if text.is_empty() {
        return Vec::new();
    }
    match serde_json::from_str::<serde_json::Value>(text) {
        Ok(serde_json::Value::Array(items)) => items
            .into_iter()
            .filter_map(|v| match v {
                serde_json::Value::Null => None,
                serde_json::Value::String(s) => Some(s),
                other => Some(other.to_string()),
            })
            .collect(),
        Ok(other) => {
            eprintln!("[parse_tags_json] tags 列不是 JSON 数组: {}", other);
            Vec::new()
        }
        Err(e) => {
            eprintln!(
                "[parse_tags_json] tags 列 JSON 解析失败: {} | raw: {}",
                e, text
            );
            Vec::new()
        }
    }
}

/// 把 diseases.species 这类"以 JSON 文本存放的多值列"解析成真数组。
///
/// 根因：后端一直下发 JSON 文本，前端各写各的 —— 列表页 JSON.parse 才对，
/// 详情页直接 v-for 字符串（Vue 按字符迭代 → 一格一个引号），对比页把原文当文本打印，
/// 症状检索页 split(',') 会切出 `["犬"` 这种脏值。统一在 Rust 侧解析掉。
/// 退回顺序：JSON 数组 → 逗号分隔 → 单值 → 空。
pub fn parse_string_list(raw: Option<&str>) -> Vec<String> {
    let Some(text) = raw.map(str::trim).filter(|t| !t.is_empty()) else {
        return Vec::new();
    };
    if text.starts_with('[') {
        if let Ok(serde_json::Value::Array(items)) = serde_json::from_str::<serde_json::Value>(text)
        {
            return items
                .into_iter()
                .filter_map(|v| {
                    v.as_str()
                        .map(str::to_string)
                        .or_else(|| Some(v.to_string()))
                })
                .filter(|s| !s.trim().is_empty())
                .collect();
        }
        eprintln!("[parse_string_list] 不是合法 JSON 数组: {}", text);
    }
    text.split(',')
        .map(|s| {
            s.trim()
                .trim_matches(|c| c == '"' || c == '[' || c == ']')
                .trim()
        })
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

/// 读取行里的 `tags` 列并解析为标签 id 列表；列不存在时返回空 Vec。
///
/// 列不存在是合法场景：不是每条查询都附加了 tags_subquery（例如只用 `SELECT *` 的历史 SQL）。
#[inline]
pub fn row_tags(r: &SqliteRow) -> Vec<String> {
    match r.try_get::<Option<String>, _>("tags") {
        Ok(raw) => parse_tags_json(raw.as_deref()),
        Err(sqlx::Error::ColumnNotFound(_)) => Vec::new(),
        Err(e) => {
            eprintln!("[row_tags] tags 列读取失败: {}", e);
            Vec::new()
        }
    }
}

#[inline]
pub fn row_from_disease(r: &SqliteRow) -> Disease {
    Disease {
        id: r.get("id"),
        name_zh: r.get("name_zh"),
        name_en: r.get("name_en"),
        name_latin: r.get("name_latin"),
        category: parse_string_list(r.get::<Option<String>, _>("category").as_deref()),
        species: parse_string_list(r.get::<Option<String>, _>("species").as_deref()),
        body_system: r.get("body_system"),
        pathogenic_type: r.get("pathogenic_type"),
        epidemiology: r.get("epidemiology"),
        overview: r.get("overview"),
        etiology: r.get("etiology"),
        pathophysiology: r.get("pathophysiology"),
        physiological_basis: r.get("physiological_basis"),
        prognosis: r.get("prognosis"),
        difficulty: r.get("difficulty"),
        urgency_level: r.get("urgency_level"),
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
        tags: row_tags(r),
    }
}

#[inline]
pub fn row_from_symptom(r: &SqliteRow) -> Symptom {
    Symptom {
        id: r.get("id"),
        name_zh: r.get("name_zh"),
        name_en: r.get("name_en"),
        definition: r.get("definition"),
        species_notes: r.get("species_notes"),
        physiological_basis: r.get("physiological_basis"),
        tags: row_tags(r),
    }
}

#[inline]
pub fn row_from_drug(r: &SqliteRow) -> Drug {
    Drug {
        id: r.get("id"),
        name_zh: r.get("name_zh"),
        name_en: r.get("name_en"),
        drug_class: r.get("drug_class"),
        mechanism_of_action: r.get("mechanism_of_action"),
        pk_pd: r.get("pk_pd"),
        indications: r.get("indications"),
        contraindications: r.get("contraindications"),
        side_effects: r.get("side_effects"),
        adverse_mechanism: r.get("adverse_mechanism"),
        species_dosing: r.get("species_dosing"),
        pubchem_cid: r.get("pubchem_cid"),
        molecular_formula: r.get("molecular_formula"),
        molecular_weight: r.get("molecular_weight"),
        smiles: r.get("smiles"),
        inchikey: r.get("inchikey"),
        iupac_name: r.get("iupac_name"),
        tags: row_tags(r),
    }
}

#[inline]
pub fn row_from_test(r: &SqliteRow) -> DiagnosticTest {
    DiagnosticTest {
        id: r.get("id"),
        name_zh: r.get("name_zh"),
        category: r.get("category"),
        reference_ranges: r.get("reference_ranges"),
        interpretation: r.get("interpretation"),
        cost_estimate: r.get("cost_estimate"),
        turnaround_time: r.get("turnaround_time"),
    }
}

#[inline]
pub fn row_from_treatment(r: &SqliteRow) -> Treatment {
    Treatment {
        id: r.get("id"),
        name_zh: r.get("name_zh"),
        name_en: r.get("name_en"),
        therapy_type: r.get("therapy_type"),
        principle: r.get("principle"),
        procedure_text: r.get("procedure_text"),
        physiological_basis: r.get("physiological_basis"),
        prognosis_assessment: r.get("prognosis_assessment"),
        created_at: r.get("created_at"),
        tags: row_tags(r),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Disease {
    pub id: String,
    pub name_zh: String,
    pub name_en: Option<String>,
    pub name_latin: Option<String>,
    /// 分类列表（DB 同样是 JSON 文本，如 ["泌尿系统", "急诊"]，见 parse_string_list）
    pub category: Vec<String>,
    /// 物种列表（DB 存 JSON 文本，这里解析成数组，见 parse_string_list）
    pub species: Vec<String>,
    pub body_system: Option<String>,
    pub pathogenic_type: Option<String>,
    pub epidemiology: Option<String>,
    pub overview: Option<String>,
    pub etiology: Option<String>,
    pub pathophysiology: Option<String>,
    pub physiological_basis: Option<String>,
    pub prognosis: Option<String>,
    pub difficulty: Option<String>,
    pub urgency_level: Option<i64>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    /// 标签 id 列表（entity_tags JOIN tags.id），与前端 Disease.tags: string[] 对应。
    /// 由 tags_subquery() 附加的 `tags` 列提供；缺列时为空数组。
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symptom {
    pub id: String,
    pub name_zh: String,
    pub name_en: Option<String>,
    pub definition: Option<String>,
    pub species_notes: Option<String>,
    pub physiological_basis: Option<String>,
    /// 标签 id 列表，与前端 Symptom.tags: string[] 对应。
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Drug {
    pub id: String,
    pub name_zh: String,
    pub name_en: Option<String>,
    pub drug_class: Option<String>,
    pub mechanism_of_action: Option<String>,
    pub pk_pd: Option<String>,
    pub indications: Option<String>,
    pub contraindications: Option<String>,
    pub side_effects: Option<String>,
    pub adverse_mechanism: Option<String>,
    pub species_dosing: Option<String>,
    pub pubchem_cid: Option<i64>,
    pub molecular_formula: Option<String>,
    pub molecular_weight: Option<f64>,
    pub smiles: Option<String>,
    pub inchikey: Option<String>,
    pub iupac_name: Option<String>,
    /// 标签 id 列表，与前端 Drug.tags: string[] 对应。
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DiagnosticTest {
    pub id: String,
    pub name_zh: String,
    pub category: Option<String>,
    pub reference_ranges: Option<String>,
    pub interpretation: Option<String>,
    pub cost_estimate: Option<f64>,
    pub turnaround_time: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Case {
    pub id: String,
    pub title: String,
    pub species: Option<String>,
    pub breed: Option<String>,
    pub age: Option<f64>,
    pub weight: Option<f64>,
    pub chief_complaint: Option<String>,
    pub history: Option<String>,
    pub physical_exam: Option<String>,
    pub lab_results: Option<String>,
    pub imaging: Option<String>,
    pub diagnosis: Option<String>,
    pub treatment: Option<String>,
    pub outcome: Option<String>,
    pub learning_points: Option<String>,
    pub difficulty: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Treatment {
    pub id: String,
    pub name_zh: String,
    pub name_en: Option<String>,
    pub therapy_type: Option<String>,
    pub principle: Option<String>,
    pub procedure_text: Option<String>,
    pub physiological_basis: Option<String>,
    pub prognosis_assessment: Option<String>,
    pub created_at: Option<String>,
    /// 标签 id 列表，与前端 Treatment.tags: string[] 对应。
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Tag {
    pub id: String,
    pub name_zh: String,
    pub name_en: Option<String>,
    pub tag_group: String,
    pub emergency_level: Option<String>,
    pub clinical_action: Option<String>,
    pub textbook_logic: Option<String>,
    pub typical_scenario: Option<String>,
    pub color: Option<String>,
    pub created_at: Option<String>,
}

// ── FromRow 实现 ──
// 带 tags 的四个实体复用 row_from_*()：query_as 与手动映射走同一条解析路径，
// tags 列缺失时由 row_tags() 回退成空 Vec（不报错）。

impl<'r> FromRow<'r, SqliteRow> for Disease {
    fn from_row(row: &'r SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(row_from_disease(row))
    }
}

impl<'r> FromRow<'r, SqliteRow> for Symptom {
    fn from_row(row: &'r SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(row_from_symptom(row))
    }
}

impl<'r> FromRow<'r, SqliteRow> for Drug {
    fn from_row(row: &'r SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(row_from_drug(row))
    }
}

impl<'r> FromRow<'r, SqliteRow> for Treatment {
    fn from_row(row: &'r SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(row_from_treatment(row))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiseaseWithSymptom {
    pub disease: Disease,
    pub frequency: String,
    pub stage: String,
    pub is_pathognomonic: i64,
}

// ── DTO（数据传输对象，不映射数据库表） ──
// 诊断推理引擎的输出结构，由 engine.rs 构造，通过 diagnose 命令返回前端

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSuggestion {
    pub test_id: String,
    pub test_name: String,
    pub purpose: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosisCandidate {
    pub disease_id: String,
    pub disease_name: String,
    pub match_score: f64,
    pub input_coverage: f64,
    pub matched_symptoms: Vec<String>,
    pub missing_key_symptoms: Vec<String>,
    pub suggested_tests: Vec<TestSuggestion>,
    pub distinguishing_features: Vec<String>,
}

// 搜索命令的返回结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub entity_type: String,
    pub entity_id: String,
    pub title: String,
    pub snippet: String,
    pub relevance: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::sql_builder::tags_subquery;
    use crate::db::{split_sql_statements, strip_sql_comments};
    use sqlx::sqlite::SqlitePoolOptions;

    fn parse(raw: Option<&str>) -> Vec<String> {
        parse_tags_json(raw)
    }

    fn strings(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    // ── 多值列（species 这类 JSON 文本）解析 ──

    #[test]
    fn test_parse_string_list_accepts_json_array_and_legacy_shapes() {
        assert_eq!(
            parse_string_list(Some(r#"["犬", "猫"]"#)),
            strings(&["犬", "猫"])
        );
        assert_eq!(parse_string_list(Some(r#"["犬"]"#)), strings(&["犬"]));
        assert_eq!(parse_string_list(Some("犬,猫")), strings(&["犬", "猫"]));
        assert_eq!(parse_string_list(Some("犬")), strings(&["犬"]));
        assert!(parse_string_list(None).is_empty());
        assert!(parse_string_list(Some("   ")).is_empty());
        assert!(parse_string_list(Some("[]")).is_empty());
    }

    #[test]
    fn test_parse_string_list_never_emits_json_punctuation() {
        // 历史故障：详情页直接 v-for 这个字符串，Vue 按字符迭代 → 界面上出现
        // 一格一个 `"` / `[` / `,` 的碎标签。半截 JSON 也不能吐出 `["犬"` 这种脏值。
        for raw in [r#"["犬","""#, r#"["犬", "猫""#, r#""犬""#] {
            let got = parse_string_list(Some(raw));
            for item in &got {
                assert!(
                    !item.contains('[') && !item.contains(']') && !item.contains('"'),
                    "解析结果残留 JSON 符号: {:?} ← {:?}",
                    got,
                    raw
                );
            }
        }
        assert_eq!(parse_string_list(Some(r#"["犬","""#)), strings(&["犬"]));
    }

    // ── 纯解析测试 ──

    #[test]
    fn test_parse_tags_json_array_of_strings() {
        assert_eq!(
            parse(Some(r#"["呼吸系统","感染"]"#)),
            strings(&["呼吸系统", "感染"])
        );
    }

    #[test]
    fn test_parse_tags_json_empty_cases_are_empty_vec() {
        // 空标签实体：json_group_array 产出 []；NULL / 空串同样解析成空 Vec
        assert!(parse(Some("[]")).is_empty());
        assert!(parse(None).is_empty());
        assert!(parse(Some("")).is_empty());
        assert!(parse(Some("   ")).is_empty());
    }

    #[test]
    fn test_parse_tags_json_skips_null_items() {
        assert_eq!(parse(Some(r#"["a",null,"b"]"#)), strings(&["a", "b"]));
    }

    #[test]
    fn test_parse_tags_json_rejects_bad_input() {
        // 逗号字符串 / 裸字符串不是数组 → 空 Vec（会打 stderr，不静默）
        // 注：这两个用例会看到 [parse_tags_json] 的 stderr 输出，属于预期。
        assert!(parse(Some("呼吸系统,感染")).is_empty());
        assert!(parse(Some(r#""呼吸系统""#)).is_empty());
        assert!(parse(Some("{not json")).is_empty());
    }

    // ── JSON 形状：与前端 src/types/index.ts 的 tags: string[] 对齐 ──

    fn disease_with(tags: Vec<String>) -> Disease {
        Disease {
            id: "dis_t1".into(),
            name_zh: "测试疾病".into(),
            name_en: None,
            name_latin: None,
            category: Vec::new(),
            species: Vec::new(),
            body_system: None,
            pathogenic_type: None,
            epidemiology: None,
            overview: None,
            etiology: None,
            pathophysiology: None,
            physiological_basis: None,
            prognosis: None,
            difficulty: None,
            urgency_level: None,
            created_at: None,
            updated_at: None,
            tags,
        }
    }

    fn symptom_with(tags: Vec<String>) -> Symptom {
        Symptom {
            id: "sym_t1".into(),
            name_zh: "咳嗽".into(),
            name_en: None,
            definition: None,
            species_notes: None,
            physiological_basis: None,
            tags,
        }
    }

    fn drug_with(tags: Vec<String>) -> Drug {
        Drug {
            id: "drug_t1".into(),
            name_zh: "阿莫西林".into(),
            name_en: None,
            drug_class: None,
            mechanism_of_action: None,
            pk_pd: None,
            indications: None,
            contraindications: None,
            side_effects: None,
            adverse_mechanism: None,
            species_dosing: None,
            pubchem_cid: None,
            molecular_formula: None,
            molecular_weight: None,
            smiles: None,
            inchikey: None,
            iupac_name: None,
            tags,
        }
    }

    fn treatment_with(tags: Vec<String>) -> Treatment {
        Treatment {
            id: "trt_t1".into(),
            name_zh: "抗菌方案".into(),
            name_en: None,
            therapy_type: None,
            principle: None,
            procedure_text: None,
            physiological_basis: None,
            prognosis_assessment: None,
            created_at: None,
            tags,
        }
    }

    #[test]
    fn test_tags_field_serializes_as_json_array_on_all_entities() {
        // 四个实体都必须暴露 tags 键，且形状是 JSON 数组（不是逗号字符串）
        let with_tags = [
            serde_json::to_value(disease_with(strings(&["呼吸系统", "感染"]))).unwrap(),
            serde_json::to_value(symptom_with(strings(&["呼吸系统"]))).unwrap(),
            serde_json::to_value(drug_with(strings(&["感染"]))).unwrap(),
            serde_json::to_value(treatment_with(strings(&["感染"]))).unwrap(),
        ];
        for v in &with_tags {
            assert!(v.get("tags").is_some(), "返回体里没有 tags 键");
            assert!(v["tags"].is_array(), "tags 不是 JSON 数组: {}", v["tags"]);
            assert_ne!(v["tags"], serde_json::json!("呼吸系统,感染"));
        }
        assert_eq!(
            with_tags[0]["tags"],
            serde_json::json!(["呼吸系统", "感染"])
        );

        // 无标签实体 → "tags": []（前端直接渲染空数组，不会出现 undefined）
        let empty = [
            serde_json::to_value(disease_with(vec![])).unwrap(),
            serde_json::to_value(symptom_with(vec![])).unwrap(),
            serde_json::to_value(drug_with(vec![])).unwrap(),
            serde_json::to_value(treatment_with(vec![])).unwrap(),
        ];
        for v in &empty {
            assert_eq!(v["tags"], serde_json::json!([]), "{}", v);
        }
    }

    // ── 种子数据：四类实体的 entity_tags 都要生成（根因：生成器只写了 disease）──

    #[test]
    fn test_seed_sql_has_entity_tags_for_every_taggable_type() {
        let seed = include_str!("../../data/seed/001_initial.sql");
        let rows: Vec<&str> = seed
            .lines()
            .filter(|l| l.starts_with("INSERT INTO entity_tags"))
            .collect();
        let mut total = 0usize;
        // case 目前数据源里没有 tags，允许 0 行；其余四类必须 > 0
        for entity_type in ["disease", "symptom", "drug", "treatment"] {
            let prefix = format!(
                "INSERT INTO entity_tags (entity_type, entity_id, tag_id) VALUES ('{}',",
                entity_type
            );
            let n = rows
                .iter()
                .filter(|r| r.starts_with(prefix.as_str()))
                .count();
            assert!(
                n > 0,
                "种子 SQL 里没有 {} 的 entity_tags 行（标签区会永远空白）",
                entity_type
            );
            total += n;
        }
        assert_eq!(total, rows.len(), "entity_tags 出现了未预期的 entity_type");
    }

    // ── 端到端：真实 schema.sql + tags_subquery + row_from_* ──

    async fn setup_pool() -> sqlx::SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("内存 SQLite 连接失败");
        let schema = strip_sql_comments(include_str!("../../data/seed/schema.sql"));
        for stmt in split_sql_statements(&schema) {
            let t = stmt.trim();
            if !t.is_empty() {
                sqlx::query(t)
                    .execute(&pool)
                    .await
                    .expect("schema.sql 执行失败");
            }
        }
        for stmt in [
            "INSERT INTO tags (id, name_zh, tag_group) VALUES ('body_resp','呼吸系统','body_system')",
            "INSERT INTO tags (id, name_zh, tag_group) VALUES ('mech_inf','感染','mechanism')",
            "INSERT INTO diseases (id, name_zh) VALUES ('dis_t1','测试疾病')",
            "INSERT INTO diseases (id, name_zh) VALUES ('dis_t2','无标签疾病')",
            "INSERT INTO symptoms (id, name_zh) VALUES ('sym_t1','咳嗽')",
            "INSERT INTO drugs (id, name_zh) VALUES ('drug_t1','阿莫西林')",
            "INSERT INTO treatments (id, name_zh, therapy_type) VALUES ('trt_t1','抗菌方案','药物疗法')",
            "INSERT INTO entity_tags (entity_type, entity_id, tag_id) VALUES ('disease','dis_t1','mech_inf')",
            "INSERT INTO entity_tags (entity_type, entity_id, tag_id) VALUES ('disease','dis_t1','body_resp')",
            "INSERT INTO entity_tags (entity_type, entity_id, tag_id) VALUES ('symptom','sym_t1','body_resp')",
            "INSERT INTO entity_tags (entity_type, entity_id, tag_id) VALUES ('drug','drug_t1','mech_inf')",
            "INSERT INTO entity_tags (entity_type, entity_id, tag_id) VALUES ('treatment','trt_t1','body_resp')",
        ] {
            sqlx::query(stmt).execute(&pool).await.expect("测试种子插入失败");
        }
        pool
    }

    /// 拼装 commands 层用的查询：主表全部列 + tags 子查询
    fn select_one(table: &str, alias: &str, entity_type: &str) -> String {
        format!(
            "SELECT {}.*, {} FROM {} {} WHERE {}.id = ?",
            alias,
            tags_subquery(alias, entity_type),
            table,
            alias,
            alias
        )
    }

    #[tokio::test]
    async fn test_row_from_all_four_entity_types_parse_tags() {
        let pool = setup_pool().await;

        let row = sqlx::query(&select_one("diseases", "d", "disease"))
            .bind("dis_t1")
            .fetch_one(&pool)
            .await
            .unwrap();
        // 排序：tag_group, name_zh → body_system 在 mechanism 前
        // 元素是 tags.id（不是 name_zh）：前端要拿它回查 get_tags() 取 color/tag_group，
        // 下发显示名会让那次回查永远落空、标签区照旧空白。
        assert_eq!(
            row_from_disease(&row).tags,
            strings(&["body_resp", "mech_inf"])
        );

        let row = sqlx::query(&select_one("symptoms", "s", "symptom"))
            .bind("sym_t1")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(row_from_symptom(&row).tags, strings(&["body_resp"]));

        let row = sqlx::query(&select_one("drugs", "dr", "drug"))
            .bind("drug_t1")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(row_from_drug(&row).tags, strings(&["mech_inf"]));

        let row = sqlx::query(&select_one("treatments", "t", "treatment"))
            .bind("trt_t1")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(row_from_treatment(&row).tags, strings(&["body_resp"]));
    }

    #[tokio::test]
    async fn test_entity_without_tags_yields_empty_vec() {
        let pool = setup_pool().await;
        let d = row_from_disease(
            &sqlx::query(&select_one("diseases", "d", "disease"))
                .bind("dis_t2")
                .fetch_one(&pool)
                .await
                .unwrap(),
        );
        assert!(d.tags.is_empty());
        assert_eq!(
            serde_json::to_value(&d).unwrap()["tags"],
            serde_json::json!([])
        );
    }

    #[tokio::test]
    async fn test_query_without_tags_column_still_works() {
        let pool = setup_pool().await;
        // 不含 tags 列的历史 SQL：手动映射与 query_as 都不能崩，tags 为空
        let row = sqlx::query("SELECT * FROM diseases WHERE id = ?")
            .bind("dis_t1")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!(row_from_disease(&row).tags.is_empty());

        let d = sqlx::query_as::<_, Disease>("SELECT * FROM diseases WHERE id = ?")
            .bind("dis_t1")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!(d.tags.is_empty());

        // 带 tags 列时 query_as（手写 FromRow）同样解析出标签
        let d = sqlx::query_as::<_, Disease>(&select_one("diseases", "d", "disease"))
            .bind("dis_t1")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(d.tags, strings(&["body_resp", "mech_inf"]));
    }

    #[tokio::test]
    async fn test_null_tags_column_yields_empty_vec() {
        let pool = setup_pool().await;
        let row = sqlx::query("SELECT d.*, NULL AS tags FROM diseases d WHERE d.id = ?")
            .bind("dis_t1")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!(row_from_disease(&row).tags.is_empty());
    }
}
