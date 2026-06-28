use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
pub struct DiagnosisInput {
    pub symptoms: Vec<String>,
    pub species: String,
    pub age: Option<f64>,
    pub breed: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, Serialize)]
pub struct TestSuggestion {
    pub test_id: String,
    pub test_name: String,
    pub purpose: String,
}

pub fn infer(
    input: &DiagnosisInput,
    disease_list: &[(String, String)],
    all_disease_symptoms: &HashMap<String, Vec<(String, String, i64)>>,
    diagnostics: &HashMap<String, Vec<(String, String)>>,
) -> Vec<DiagnosisCandidate> {
    let input_set: std::collections::HashSet<&str> =
        input.symptoms.iter().map(|s| s.as_str()).collect();

    let mut candidates: Vec<DiagnosisCandidate> = Vec::new();

    for (did, dname) in disease_list {
        let sym_list = match all_disease_symptoms.get(did) {
            Some(s) => s,
            None => continue,
        };

        let mut matched = Vec::new();
        let mut missing = Vec::new();
        let mut weighted_score = 0.0;
        let mut total_weight = 0.0;
        let mut pathognomonic_matched = 0;

        for (sym_name, freq, is_pathognomonic) in sym_list {
            let base_weight = match freq.as_str() {
                "common" => 1.0,
                "uncommon" => 0.6,
                "rare" => 0.3,
                _ => 0.5,
            };
            total_weight += base_weight;
            if input_set.contains(sym_name.as_str()) {
                matched.push(sym_name.clone());
                let w = if *is_pathognomonic == 1 {
                    pathognomonic_matched += 1;
                    base_weight * 1.5
                } else {
                    base_weight
                };
                weighted_score += w;
            } else if base_weight >= 1.0 {
                missing.push(sym_name.clone());
            }
        }

        if matched.is_empty() {
            continue;
        }

        let disease_coverage: f64 = if total_weight > 0.0 {
            weighted_score / total_weight
        } else {
            0.0
        };

        let input_coverage: f64 = matched.len() as f64 / input_set.len() as f64;

        let mut score = disease_coverage * input_coverage;

        let pathognomonic_bonus = (pathognomonic_matched as f64 * 0.05).min(0.15);
        score = (score + pathognomonic_bonus).min(1.0);

        if score < 0.25 || input_coverage < 0.3 {
            continue;
        }

        let tests = diagnostics
            .get(did)
            .map(|t| {
                t.iter()
                    .map(|(tid, purpose)| TestSuggestion {
                        test_id: tid.clone(),
                        test_name: tid.clone(),
                        purpose: purpose.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        candidates.push(DiagnosisCandidate {
            disease_id: did.clone(),
            disease_name: dname.clone(),
            match_score: (score * 100.0).round() / 100.0,
            input_coverage: (input_coverage * 100.0).round() / 100.0,
            matched_symptoms: matched,
            missing_key_symptoms: missing,
            suggested_tests: tests,
            distinguishing_features: Vec::new(),
        });
    }

    candidates.sort_by(|a, b| {
        b.match_score
            .partial_cmp(&a.match_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    candidates.truncate(10);
    candidates
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    // === 辅助构造函数 ===

    fn make_input(symptoms: &[&str], species: &str) -> DiagnosisInput {
        DiagnosisInput {
            symptoms: symptoms.iter().map(|s| s.to_string()).collect(),
            species: species.to_string(),
            age: None,
            breed: None,
        }
    }

    fn make_disease_list(diseases: &[(&str, &str)]) -> Vec<(String, String)> {
        diseases
            .iter()
            .map(|(id, name)| (id.to_string(), name.to_string()))
            .collect()
    }

    fn make_symptom_map(
        disease_id: &str,
        symptoms: &[(&str, &str, i64)],
    ) -> HashMap<String, Vec<(String, String, i64)>> {
        let mut map = HashMap::new();
        map.insert(
            disease_id.to_string(),
            symptoms
                .iter()
                .map(|(s, f, p)| (s.to_string(), f.to_string(), *p))
                .collect(),
        );
        map
    }

    // === 正常路径 ===

    #[test]
    fn test_infer_basic_match() {
        let input = make_input(&["咳嗽", "发热"], "犬");
        let disease_list = make_disease_list(&[("dis_001", "肺炎")]);
        let all_disease_symptoms = make_symptom_map(
            "dis_001",
            &[("咳嗽", "common", 0), ("发热", "common", 0)],
        );
        let diagnostics = HashMap::new();

        let result = infer(&input, &disease_list, &all_disease_symptoms, &diagnostics);

        assert!(!result.is_empty());
        assert_eq!(result[0].disease_id, "dis_001");
        assert_eq!(result[0].disease_name, "肺炎");
        // 两个 common 症状全部匹配：disease_coverage=1.0, input_coverage=1.0, score=1.0
        assert!((result[0].match_score - 1.0).abs() < 0.001);
        assert_eq!(result[0].matched_symptoms.len(), 2);
        assert!(result[0].matched_symptoms.contains(&"咳嗽".to_string()));
        assert!(result[0].matched_symptoms.contains(&"发热".to_string()));
    }

    // === 频率权重 ===

    #[test]
    fn test_infer_frequency_common() {
        let input = make_input(&["咳嗽"], "犬");
        let disease_list = make_disease_list(&[("dis_001", "肺炎")]);
        let all_disease_symptoms = make_symptom_map("dis_001", &[("咳嗽", "common", 0)]);

        let result = infer(&input, &disease_list, &all_disease_symptoms, &HashMap::new());

        assert!(!result.is_empty());
        // common weight=1.0, 全匹配 disease_coverage=1.0, input_coverage=1.0
        assert!((result[0].match_score - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_infer_frequency_uncommon() {
        let input = make_input(&["皮疹"], "犬");
        let disease_list = make_disease_list(&[("dis_001", "某病")]);
        // uncommon 症状匹配，但有 common 症状未匹配
        let all_disease_symptoms = make_symptom_map(
            "dis_001",
            &[("发热", "common", 0), ("皮疹", "uncommon", 0)],
        );

        let result = infer(&input, &disease_list, &all_disease_symptoms, &HashMap::new());

        assert!(!result.is_empty());
        // weighted_score = 0.6, total_weight = 1.6
        // disease_coverage = 0.6/1.6 = 0.375
        // input_coverage = 1/1 = 1.0
        // score = 0.375 * 1.0 = 0.375，浮点精度可能导致 0.37 或 0.38
        assert!(
            result[0].match_score >= 0.37 && result[0].match_score <= 0.38,
            "expected ~0.375, got {}",
            result[0].match_score
        );
        // 未匹配的 common 症状进入 missing_key_symptoms
        assert_eq!(result[0].missing_key_symptoms, vec!["发热".to_string()]);
    }

    #[test]
    fn test_infer_frequency_rare() {
        let input = make_input(&["罕见症状"], "犬");
        let disease_list = make_disease_list(&[("dis_001", "某病")]);
        let all_disease_symptoms = make_symptom_map("dis_001", &[("罕见症状", "rare", 0)]);

        let result = infer(&input, &disease_list, &all_disease_symptoms, &HashMap::new());

        assert!(!result.is_empty());
        // rare weight=0.3, 全匹配 disease_coverage=1.0, input_coverage=1.0
        assert!((result[0].match_score - 1.0).abs() < 0.001);
    }

    // === 核心症状加成 ===

    #[test]
    fn test_infer_pathognomonic_bonus() {
        let input = make_input(&["特征症状"], "犬");
        let disease_list = make_disease_list(&[("dis_001", "某病")]);
        // is_pathognomonic=1，应获得 1.5× 权重 + 0.05 加成
        let all_disease_symptoms =
            make_symptom_map("dis_001", &[("特征症状", "common", 1)]);

        let result = infer(&input, &disease_list, &all_disease_symptoms, &HashMap::new());

        assert!(!result.is_empty());
        // weighted_score = 1.0 * 1.5 = 1.5
        // total_weight = 1.0
        // disease_coverage = 1.5
        // input_coverage = 1.0
        // score = 1.5 * 1.0 = 1.5 → min(1.0) = 1.0
        // pathognomonic_bonus = 0.05 → score = min(1.05, 1.0) = 1.0
        assert!((result[0].match_score - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_infer_pathognomonic_bonus_cap() {
        let input = make_input(
            &["特征1", "特征2", "特征3", "特征4"],
            "犬",
        );
        let disease_list = make_disease_list(&[("dis_001", "某病")]);
        // 4 个核心症状，加成应被截断为 0.15
        let all_disease_symptoms = make_symptom_map(
            "dis_001",
            &[
                ("特征1", "common", 1),
                ("特征2", "common", 1),
                ("特征3", "common", 1),
                ("特征4", "common", 1),
            ],
        );

        let result = infer(&input, &disease_list, &all_disease_symptoms, &HashMap::new());

        assert!(!result.is_empty());
        // 4 个 pathognomonic_matched，bonus = min(4*0.05, 0.15) = 0.15
        // base score = 1.5 (disease_coverage) * 1.0 (input_coverage) = 1.5
        // score = min(1.5 + 0.15, 1.0) = 1.0
        assert!((result[0].match_score - 1.0).abs() < 0.001);
    }

    // === 阈值过滤 ===

    #[test]
    fn test_infer_threshold_filter_low_score() {
        let input = make_input(&["症状A"], "犬");
        let disease_list = make_disease_list(&[("dis_001", "某病")]);
        // 5 个 common 症状，只匹配 1 个
        // disease_coverage = 1.0 / 5.0 = 0.2
        // input_coverage = 1.0
        // score = 0.2 < 0.25 → 被过滤
        let all_disease_symptoms = make_symptom_map(
            "dis_001",
            &[
                ("症状A", "common", 0),
                ("症状B", "common", 0),
                ("症状C", "common", 0),
                ("症状D", "common", 0),
                ("症状E", "common", 0),
            ],
        );

        let result = infer(&input, &disease_list, &all_disease_symptoms, &HashMap::new());

        assert!(result.is_empty(), "score 低于 0.25 应被过滤");
    }

    #[test]
    fn test_infer_threshold_filter_low_coverage() {
        let input = make_input(&["症状A", "症状B", "症状C", "症状D"], "犬");
        let disease_list = make_disease_list(&[("dis_001", "某病")]);
        // 4 个输入症状，只匹配 1 个
        // input_coverage = 1/4 = 0.25 < 0.3 → 被过滤
        let all_disease_symptoms =
            make_symptom_map("dis_001", &[("症状A", "common", 0)]);

        let result = infer(&input, &disease_list, &all_disease_symptoms, &HashMap::new());

        assert!(result.is_empty(), "input_coverage 低于 0.3 应被过滤");
    }

    // === 排序 ===

    #[test]
    fn test_infer_sort_descending() {
        let input = make_input(&["咳嗽"], "犬");
        let disease_list =
            make_disease_list(&[("dis_low", "低分"), ("dis_high", "高分")]);
        let mut all_disease_symptoms = HashMap::new();
        // dis_high: 完全匹配
        all_disease_symptoms.insert(
            "dis_high".to_string(),
            vec![("咳嗽".to_string(), "common".to_string(), 0)],
        );
        // dis_low: 匹配但有未匹配 common 症状，分数较低
        all_disease_symptoms.insert(
            "dis_low".to_string(),
            vec![
                ("咳嗽".to_string(), "common".to_string(), 0),
                ("发热".to_string(), "common".to_string(), 0),
            ],
        );

        let result = infer(&input, &disease_list, &all_disease_symptoms, &HashMap::new());

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].disease_id, "dis_high");
        assert!(result[0].match_score >= result[1].match_score);
    }

    // === 空输入 ===

    #[test]
    fn test_infer_empty_input() {
        let input = make_input(&[], "犬");
        let disease_list = make_disease_list(&[("dis_001", "肺炎")]);
        let all_disease_symptoms = make_symptom_map("dis_001", &[("咳嗽", "common", 0)]);

        let result = infer(&input, &disease_list, &all_disease_symptoms, &HashMap::new());

        // 空输入：input_set 为空，不会有 matched，应返回空
        assert!(result.is_empty());
    }

    // === 无匹配 ===

    #[test]
    fn test_infer_no_matching_disease() {
        let input = make_input(&["不存在的症状"], "犬");
        let disease_list = make_disease_list(&[("dis_001", "肺炎")]);
        let all_disease_symptoms = make_symptom_map("dis_001", &[("咳嗽", "common", 0)]);

        let result = infer(&input, &disease_list, &all_disease_symptoms, &HashMap::new());

        assert!(result.is_empty(), "无任何症状匹配时应返回空");
    }

    // === 单疾病 ===

    #[test]
    fn test_infer_single_disease() {
        let input = make_input(&["咳嗽"], "犬");
        let disease_list = make_disease_list(&[("dis_001", "肺炎")]);
        let all_disease_symptoms = make_symptom_map("dis_001", &[("咳嗽", "common", 0)]);

        let result = infer(&input, &disease_list, &all_disease_symptoms, &HashMap::new());

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].disease_id, "dis_001");
    }

    // === 诊断检查附加 ===

    #[test]
    fn test_infer_suggested_tests() {
        let input = make_input(&["咳嗽"], "犬");
        let disease_list = make_disease_list(&[("dis_001", "肺炎")]);
        let all_disease_symptoms = make_symptom_map("dis_001", &[("咳嗽", "common", 0)]);

        let mut diagnostics = HashMap::new();
        diagnostics.insert(
            "dis_001".to_string(),
            vec![
                ("test_001".to_string(), "confirming".to_string()),
                ("test_002".to_string(), "screening".to_string()),
            ],
        );

        let result = infer(&input, &disease_list, &all_disease_symptoms, &diagnostics);

        assert!(!result.is_empty());
        assert_eq!(result[0].suggested_tests.len(), 2);
        assert_eq!(result[0].suggested_tests[0].test_id, "test_001");
        assert_eq!(result[0].suggested_tests[0].purpose, "confirming");
    }

    #[test]
    fn test_infer_no_diagnostics() {
        let input = make_input(&["咳嗽"], "犬");
        let disease_list = make_disease_list(&[("dis_001", "肺炎")]);
        let all_disease_symptoms = make_symptom_map("dis_001", &[("咳嗽", "common", 0)]);

        let result = infer(&input, &disease_list, &all_disease_symptoms, &HashMap::new());

        assert!(!result.is_empty());
        assert!(result[0].suggested_tests.is_empty());
    }

    // === 截断前 10 ===

    #[test]
    fn test_infer_truncate_top10() {
        let input = make_input(&["咳嗽"], "犬");
        // 15 个疾病都匹配
        let disease_list: Vec<(String, String)> = (0..15)
            .map(|i| (format!("dis_{:03}", i), format!("疾病{}", i)))
            .collect();
        let mut all_disease_symptoms = HashMap::new();
        for (id, _) in &disease_list {
            all_disease_symptoms.insert(
                id.clone(),
                vec![("咳嗽".to_string(), "common".to_string(), 0)],
            );
        }

        let result = infer(&input, &disease_list, &all_disease_symptoms, &HashMap::new());

        assert_eq!(result.len(), 10, "应截断为前 10 个候选");
    }

    // === 输入覆盖度 ===

    #[test]
    fn test_infer_input_coverage() {
        let input = make_input(&["咳嗽", "发热", "呼吸困难"], "犬");
        let disease_list = make_disease_list(&[("dis_001", "肺炎")]);
        let all_disease_symptoms = make_symptom_map(
            "dis_001",
            &[("咳嗽", "common", 0), ("发热", "common", 0)],
        );

        let result = infer(&input, &disease_list, &all_disease_symptoms, &HashMap::new());

        assert!(!result.is_empty());
        // matched=2, input_set=3, input_coverage=2/3≈0.67
        assert!((result[0].input_coverage - 0.67).abs() < 0.01);
    }

    // === 疾病不在 symptom map 中 ===

    #[test]
    fn test_infer_disease_not_in_symptom_map() {
        let input = make_input(&["咳嗽"], "犬");
        let disease_list = make_disease_list(&[("dis_001", "肺炎"), ("dis_002", "胃炎")]);
        // 只有 dis_001 有症状数据
        let all_disease_symptoms = make_symptom_map("dis_001", &[("咳嗽", "common", 0)]);

        let result = infer(&input, &disease_list, &all_disease_symptoms, &HashMap::new());

        // dis_002 不在 map 中应被跳过
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].disease_id, "dis_001");
    }

    // === 排序稳定性（NaN 防御）===

    #[test]
    fn test_infer_sort_with_zero_scores() {
        // 验证排序不会因边界值 panic
        let input = make_input(&["咳嗽"], "犬");
        let disease_list = make_disease_list(&[("dis_001", "A"), ("dis_002", "B")]);
        let mut all_disease_symptoms = HashMap::new();
        all_disease_symptoms.insert(
            "dis_001".to_string(),
            vec![("咳嗽".to_string(), "common".to_string(), 0)],
        );
        all_disease_symptoms.insert(
            "dis_002".to_string(),
            vec![("咳嗽".to_string(), "common".to_string(), 0)],
        );

        let result = infer(&input, &disease_list, &all_disease_symptoms, &HashMap::new());

        // 不应 panic，结果应为 2 个
        assert_eq!(result.len(), 2);
    }
}
