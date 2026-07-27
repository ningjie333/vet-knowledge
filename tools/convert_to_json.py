#!/usr/bin/env python3
"""一次性转换脚本：将旧 MD+YAML 数据转为统一 JSON (data/knowledge.json)。

转换后结构：
- diseases[]: 每个疾病内嵌 symptoms/drugs/diagnostics/treatments/ddx
- symptoms[]: 全局症状数组
- drugs[]: 全局药物数组
- tests[]: 全局检查数组
- cases[]: 全局病例数组
- treatments[]: 全局治疗方案数组
- drug_interactions[]: 药物相互作用（预留，暂无数据）

运行：python tools/convert_to_json.py
"""

import os
import json
import glob
import yaml
import frontmatter

BASE_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DATA_DIR = os.path.join(BASE_DIR, "data")
OUT = os.path.join(DATA_DIR, "knowledge.json")


def load_md_entities(pattern, body_fields=None):
    """从 MD 目录加载实体列表，按文件名排序。"""
    if body_fields is None:
        body_fields = []
    entities = []
    for path in sorted(glob.glob(pattern)):
        post = frontmatter.load(path)
        meta = dict(post.metadata)
        if body_fields:
            # 解析 body 的 ## 章节
            sections = {}
            import re
            for m in re.finditer(r"^##\s+(.+?)\n(.*?)(?=\n##\s+|\Z)", post.content, re.MULTILINE | re.DOTALL):
                sections[m.group(1).strip()] = m.group(2).strip()
            for section_key, fm_key in body_fields:
                if section_key in sections:
                    meta[fm_key] = sections[section_key]
        entities.append(meta)
    return entities


def parse_list_text(text):
    """将 '- item1\n- item2' 解析为列表。"""
    if not isinstance(text, str):
        return text
    items = []
    for line in text.strip().split("\n"):
        stripped = line.strip()
        if stripped.startswith("- "):
            items.append(stripped[2:].strip())
        elif stripped.startswith("* "):
            items.append(stripped[2:].strip())
    return items if items else text


def to_json_value(v):
    """将值转为 JSON 友好格式。"""
    if v is None:
        return None
    if isinstance(v, str):
        if "\n- " in v or v.startswith("- "):
            parsed = parse_list_text(v)
            if isinstance(parsed, list):
                return parsed
        return v
    if isinstance(v, (list, dict)):
        return v
    return v


def main():
    print("=" * 60)
    print("兽医知识库 — MD+YAML → JSON 转换")
    print("=" * 60)

    # ── 1. 加载独立实体 ──────────────────────────────
    print("\n[1/4] 加载独立实体 (MD)...")

    symptoms = load_md_entities(os.path.join(DATA_DIR, "symptoms", "*.md"), [
        ("定义", "definition"),
        ("物种特异性", "species_notes"),
        ("生理基础", "physiological_basis"),
    ])
    print(f"  症状: {len(symptoms)}")

    drugs = load_md_entities(os.path.join(DATA_DIR, "drugs", "*.md"), [
        ("适应症", "indications"),
        ("禁忌症", "contraindications"),
        ("不良反应", "side_effects"),
        ("不良反应机制", "adverse_mechanism"),
        ("物种剂量", "species_dosing"),
        ("作用机制", "mechanism_of_action"),
        ("药代动力学", "pk_pd"),
    ])
    print(f"  药物: {len(drugs)}")

    tests = load_md_entities(os.path.join(DATA_DIR, "tests", "*.md"), [
        ("参考范围", "reference_ranges"),
        ("结果解读", "interpretation"),
    ])
    print(f"  检查: {len(tests)}")

    treatments = load_md_entities(os.path.join(DATA_DIR, "treatments", "*.md"), [
        ("治疗原则", "principle"),
        ("操作指南", "procedure_text"),
        ("生理基础", "physiological_basis"),
        ("预后评估", "prognosis_assessment"),
    ])
    print(f"  治疗方案: {len(treatments)}")

    cases = load_md_entities(os.path.join(DATA_DIR, "cases", "*.md"), [
        ("主诉", "chief_complaint"),
        ("病史", "history"),
        ("体格检查", "physical_exam"),
        ("实验室检查", "lab_results"),
        ("影像学", "imaging"),
        ("诊断", "diagnosis"),
        ("治疗", "treatment"),
        ("转归", "outcome"),
        ("学习要点", "learning_points"),
    ])
    print(f"  病例: {len(cases)}")

    # ── 2. 加载疾病（含 body 章节） ──────────────────
    print("\n[2/4] 加载疾病 (MD)...")

    diseases_raw = load_md_entities(os.path.join(DATA_DIR, "diseases", "*.md"), [
        ("概述", "overview"),
        ("病因", "etiology"),
        ("病理生理", "pathophysiology"),
        ("生理基础", "physiological_basis"),
        ("预后", "prognosis"),
    ])
    print(f"  疾病: {len(diseases_raw)}")

    # ── 3. 加载 YAML 关系 ────────────────────────────
    print("\n[3/4] 加载关系 (YAML)...")

    with open(os.path.join(DATA_DIR, "relations.yaml"), "r", encoding="utf-8") as f:
        relations = yaml.safe_load(f)
    with open(os.path.join(DATA_DIR, "treatment_rules.yaml"), "r", encoding="utf-8") as f:
        treatment_rules = yaml.safe_load(f)

    # 疾病-症状关系: {dis_id: [{symptom, frequency, stage, is_pathognomonic}]}
    disease_symptoms_map = {}
    for did, entries in relations.get("disease_symptoms", {}).items():
        disease_symptoms_map[did] = []
        for e in entries:
            disease_symptoms_map[did].append({
                "symptom": e["symptom"],
                "frequency": e.get("frequency", "common"),
                "stage": e.get("stage", ""),
                "is_pathognomonic": e.get("is_pathognomonic", False),
            })

    # 鉴别诊断: [{disease, target, feature}]
    ddx_list = relations.get("ddx", [])
    ddx_map = {}
    for ddx in ddx_list:
        did = ddx["disease"]
        if did not in ddx_map:
            ddx_map[did] = []
        ddx_map[did].append({
            "ddx": ddx["target"],
            "feature": ddx.get("feature", ""),
        })

    # 疾病-药物治疗: [{disease, drug, line, species, notes}]
    disease_drugs_map = {}
    for r in treatment_rules.get("disease_treatment", []):
        did = r["disease"]
        if did not in disease_drugs_map:
            disease_drugs_map[did] = []
        disease_drugs_map[did].append({
            "drug": r["drug"],
            "line": r.get("line", "first"),
            "species": r.get("species", ""),
            "notes": r.get("notes", ""),
        })

    # 疾病-诊断检查: [{disease, test, purpose, evidence_level, species, expected_result}]
    disease_diagnostics_map = {}
    for r in treatment_rules.get("disease_diagnostic", []):
        did = r["disease"]
        if did not in disease_diagnostics_map:
            disease_diagnostics_map[did] = []
        disease_diagnostics_map[did].append({
            "test": r["test"],
            "purpose": r.get("purpose", "screening"),
            "evidence_level": r.get("evidence_level", "supportive"),
            "species": r.get("species", ""),
            "expected_result": r.get("expected_result", ""),
        })

    # 疾病-治疗方案: [{disease, treatment, line, species, notes}]
    disease_treatments_map = {}
    for r in treatment_rules.get("disease_treatment_map", []):
        did = r["disease"]
        if did not in disease_treatments_map:
            disease_treatments_map[did] = []
        disease_treatments_map[did].append({
            "treatment": r["treatment"],
            "line": r.get("line", "first"),
            "species": r.get("species", ""),
            "notes": r.get("notes", ""),
        })

    print(f"  疾病-症状关系: {sum(len(v) for v in disease_symptoms_map.values())}")
    print(f"  鉴别诊断: {len(ddx_list)}")
    print(f"  疾病-药物: {sum(len(v) for v in disease_drugs_map.values())}")
    print(f"  疾病-诊断: {sum(len(v) for v in disease_diagnostics_map.values())}")
    print(f"  疾病-治疗方案: {sum(len(v) for v in disease_treatments_map.values())}")

    # ── 4. 组装疾病（内嵌关系） ──────────────────────
    print("\n[4/4] 组装 JSON...")

    diseases = []
    for d in diseases_raw:
        did = d["id"]
        disease_obj = {
            "id": did,
            "name_zh": d.get("name_zh", ""),
            "name_en": d.get("name_en"),
            "name_latin": d.get("name_latin"),
            "category": to_json_value(d.get("category")),
            "species": to_json_value(d.get("species")),
            "body_system": d.get("body_system"),
            "pathogenic_type": d.get("pathogenic_type"),
            "epidemiology": d.get("epidemiology"),
            "urgency_level": d.get("urgency_level"),
            "difficulty": d.get("difficulty", "intermediate"),
            "overview": d.get("overview"),
            "etiology": to_json_value(d.get("etiology")),
            "pathophysiology": d.get("pathophysiology"),
            "physiological_basis": d.get("physiological_basis"),
            "prognosis": d.get("prognosis"),
            "tags": d.get("tags", []),
            "symptoms": disease_symptoms_map.get(did, []),
            "drugs": disease_drugs_map.get(did, []),
            "diagnostics": disease_diagnostics_map.get(did, []),
            "treatments": disease_treatments_map.get(did, []),
            "ddx": ddx_map.get(did, []),
        }
        diseases.append(disease_obj)

    # ── 5. 清理独立实体的 None 字段 ──────────────────
    def clean_entity(e):
        """移除 None 值和空字符串，保留结构清晰。"""
        return {k: v for k, v in e.items() if v is not None and v != "" and v != []}

    symptoms_clean = [clean_entity(s) for s in symptoms]
    drugs_clean = [clean_entity(d) for d in drugs]
    tests_clean = [clean_entity(t) for t in tests]
    treatments_clean = [clean_entity(t) for t in treatments]
    cases_clean = [clean_entity(c) for c in cases]

    # ── 6. 输出 ──────────────────────────────────────
    output = {
        "diseases": diseases,
        "symptoms": symptoms_clean,
        "drugs": drugs_clean,
        "tests": tests_clean,
        "cases": cases_clean,
        "treatments": treatments_clean,
        "drug_interactions": [],
    }

    os.makedirs(DATA_DIR, exist_ok=True)
    with open(OUT, "w", encoding="utf-8") as f:
        json.dump(output, f, ensure_ascii=False, indent=2)

    print(f"\n[OK] Conversion complete: {OUT}")
    print(f"   Diseases: {len(diseases)}")
    print(f"   Symptoms: {len(symptoms_clean)}")
    print(f"   Drugs: {len(drugs_clean)}")
    print(f"   Tests: {len(tests_clean)}")
    print(f"   Cases: {len(cases_clean)}")
    print(f"   Treatments: {len(treatments_clean)}")
    print(f"   Drug interactions: 0 (reserved)")


if __name__ == "__main__":
    main()
