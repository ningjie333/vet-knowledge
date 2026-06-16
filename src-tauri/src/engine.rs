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

    candidates.sort_by(|a, b| b.match_score.partial_cmp(&a.match_score).unwrap());
    candidates.truncate(10);
    candidates
}
