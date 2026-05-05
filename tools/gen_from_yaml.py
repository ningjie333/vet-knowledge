#!/usr/bin/env python3
"""兽医知识库 SQL 种子数据生成器 — 从 YAML 数据源生成

读取 data/ 下所有 YAML 文件，生成 src-tauri/data/seed/001_initial.sql
添加新数据只需编辑 YAML 文件，然后重新运行此脚本即可。
"""

import os
import datetime
import yaml

BASE_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DATA_DIR = os.path.join(BASE_DIR, 'data')
SEED_DIR = os.path.join(BASE_DIR, 'src-tauri', 'data', 'seed')
OUT = os.path.join(SEED_DIR, '001_initial.sql')


def S(s):
    """转义字符串为 SQL 文本字面量"""
    if s is None:
        return 'NULL'
    return "'" + str(s).replace("'", "''") + "'"


def V(v):
    """任意值 → SQL 字面量"""
    if v is None:
        return 'NULL'
    if isinstance(v, (int, float)):
        return str(v)
    return S(v)


def to_json(v):
    """Python 列表/字典 → JSON 字符串（交给 V() 处理转义）"""
    if v is None:
        return None  # V(None) → 'NULL'
    import json
    return json.dumps(v, ensure_ascii=False)


def INSERT_ROW(table, columns, values):
    """生成单行 INSERT"""
    cols = ', '.join(columns)
    vals = ', '.join(V(v) for v in values)
    return f"INSERT INTO {table} ({cols}) VALUES ({vals});"


# ── 加载 YAML ──────────────────────────────────────────

with open(os.path.join(DATA_DIR, 'diseases.yaml'), 'r', encoding='utf-8') as f:
    diseases_list = yaml.safe_load(f)

with open(os.path.join(DATA_DIR, 'symptoms.yaml'), 'r', encoding='utf-8') as f:
    symptoms_list = yaml.safe_load(f)

with open(os.path.join(DATA_DIR, 'relations.yaml'), 'r', encoding='utf-8') as f:
    relations = yaml.safe_load(f)

with open(os.path.join(DATA_DIR, 'drugs.yaml'), 'r', encoding='utf-8') as f:
    drugs_list = yaml.safe_load(f)

with open(os.path.join(DATA_DIR, 'diagnostic_tests.yaml'), 'r', encoding='utf-8') as f:
    tests_list = yaml.safe_load(f)

with open(os.path.join(DATA_DIR, 'treatment_rules.yaml'), 'r', encoding='utf-8') as f:
    treatment_rules = yaml.safe_load(f)

with open(os.path.join(DATA_DIR, 'cases.yaml'), 'r', encoding='utf-8') as f:
    cases_list = yaml.safe_load(f)

# ── 生成 SQL ───────────────────────────────────────────

L = []
L.append('-- ============================================')
L.append('-- 兽医知识库 种子数据 v2.0 (YAML 驱动)')
L.append(f'-- {len(diseases_list)} 疾病 + {len(symptoms_list)} 症状 + {len(drugs_list)} 药物 + {len(tests_list)} 检查 + {len(cases_list)} 病例 + 完整关联关系')
L.append(f'-- 生成时间: {datetime.datetime.now().strftime("%Y-%m-%d %H:%M:%S")}')
L.append('-- ============================================')
L.append('')

# ===== 疾病 =====
L.append(f'-- ===== 疾病 ({len(diseases_list)}种) =====')
disease_cols = ['id', 'name_zh', 'name_en', 'category', 'species', 'overview',
                'etiology', 'pathophysiology', 'prognosis', 'difficulty', 'urgency_level']
for d in diseases_list:
    vals = [
        d['id'], d['name_zh'], d['name_en'],
        to_json(d.get('category')), to_json(d.get('species')),
        d.get('overview'), to_json(d.get('etiology')),
        d.get('pathophysiology'), d.get('prognosis'),
        d.get('difficulty', 'intermediate'), d.get('urgency_level', 3)
    ]
    L.append(INSERT_ROW('diseases', disease_cols, vals))
L.append('')

# ===== 症状 =====
L.append(f'-- ===== 症状 ({len(symptoms_list)}种) =====')
symptom_cols = ['id', 'name_zh', 'name_en', 'definition', 'species_notes']
for s in symptoms_list:
    vals = [
        s['id'], s['name_zh'], s['name_en'],
        s.get('definition'), to_json(s.get('species_notes'))
    ]
    L.append(INSERT_ROW('symptoms', symptom_cols, vals))
L.append('')

# ===== 疾病-症状关联 =====
L.append('-- ===== 疾病-症状关联 =====')
ds_count = 0
for d in diseases_list:
    did = d['id']
    entries = relations.get('disease_symptoms', {}).get(did, [])
    for entry in entries:
        symptom_id = entry['symptom']
        freq = entry['frequency']
        stage = entry.get('stage', '')
        is_patho = entry.get('is_pathognomonic', False)

        if is_patho:
            L.append(
                f"INSERT INTO disease_symptom (disease_id,symptom_id,frequency,stage,is_pathognomonic) "
                f"VALUES ({S(did)}, {S(symptom_id)}, {S(freq)}, {S(stage)}, 1);"
            )
        else:
            L.append(
                f"INSERT INTO disease_symptom (disease_id,symptom_id,frequency,stage) "
                f"VALUES ({S(did)}, {S(symptom_id)}, {S(freq)}, {S(stage)});"
            )
        ds_count += 1
L.append('')

# ===== 鉴别诊断关联 =====
L.append('-- ===== 鉴别诊断关联 =====')
ddx_list = relations.get('ddx', [])
for ddx in ddx_list:
    L.append(
        f"INSERT INTO disease_ddx (disease_id,ddx_id,distinguishing_feature) "
        f"VALUES ({S(ddx['disease'])}, {S(ddx['target'])}, {S(ddx['feature'])});"
    )
L.append('')

# ===== 药物 =====
L.append(f'-- ===== 药物 ({len(drugs_list)}种) =====')
drug_cols = ['id', 'name_zh', 'name_en', 'drug_class', 'indications',
             'contraindications', 'side_effects', 'species_dosing']
for d in drugs_list:
    vals = [
        d['id'], d['name_zh'], d['name_en'], d.get('drug_class'),
        to_json(d.get('indications')), to_json(d.get('contraindications')),
        to_json(d.get('side_effects')), to_json(d.get('species_dosing'))
    ]
    L.append(INSERT_ROW('drugs', drug_cols, vals))
L.append('')

# ===== 诊断检查 =====
L.append(f'-- ===== 诊断检查 ({len(tests_list)}项) =====')
test_cols = ['id', 'name_zh', 'category', 'reference_ranges', 'interpretation',
             'cost_estimate', 'turnaround_time']
for t in tests_list:
    vals = [
        t['id'], t['name_zh'], t.get('category'), t.get('reference_ranges'),
        t.get('interpretation'), t.get('cost_estimate'), t.get('turnaround_time')
    ]
    L.append(INSERT_ROW('diagnostic_tests', test_cols, vals))
L.append('')

# ===== 疾病-治疗关联 =====
L.append('-- ===== 疾病-治疗关联 =====')
dt_list = treatment_rules.get('disease_treatment', [])
for r in dt_list:
    species = r.get('species', '')
    notes = r.get('notes', '')
    if species and notes:
        L.append(
            f"INSERT INTO disease_treatment (disease_id,drug_id,line,species,notes) "
            f"VALUES ({S(r['disease'])}, {S(r['drug'])}, {S(r['line'])}, {S(species)}, {S(notes)});"
        )
    elif species:
        L.append(
            f"INSERT INTO disease_treatment (disease_id,drug_id,line,species) "
            f"VALUES ({S(r['disease'])}, {S(r['drug'])}, {S(r['line'])}, {S(species)});"
        )
    else:
        L.append(
            f"INSERT INTO disease_treatment (disease_id,drug_id,line) "
            f"VALUES ({S(r['disease'])}, {S(r['drug'])}, {S(r['line'])});"
        )
L.append('')

# ===== 疾病-诊断关联 =====
L.append('-- ===== 疾病-诊断关联 =====')
dd_list = treatment_rules.get('disease_diagnostic', [])
for r in dd_list:
    species = r.get('species', '')
    expected = r.get('expected_result', '')
    if species and expected:
        L.append(
            f"INSERT INTO disease_diagnostic (disease_id,test_id,purpose,evidence_level,species,expected_result) "
            f"VALUES ({S(r['disease'])}, {S(r['test'])}, {S(r['purpose'])}, {S(r['evidence_level'])}, {S(species)}, {S(expected)});"
        )
    elif species:
        L.append(
            f"INSERT INTO disease_diagnostic (disease_id,test_id,purpose,evidence_level,species) "
            f"VALUES ({S(r['disease'])}, {S(r['test'])}, {S(r['purpose'])}, {S(r['evidence_level'])}, {S(species)});"
        )
    else:
        L.append(
            f"INSERT INTO disease_diagnostic (disease_id,test_id,purpose,evidence_level) "
            f"VALUES ({S(r['disease'])}, {S(r['test'])}, {S(r['purpose'])}, {S(r['evidence_level'])});"
        )
L.append('')

# ===== 病例 =====
L.append(f'-- ===== 病例 ({len(cases_list)}个) =====')
case_cols = ['id', 'title', 'species', 'breed', 'age', 'weight', 'chief_complaint',
             'history', 'physical_exam', 'lab_results', 'imaging', 'diagnosis',
             'treatment', 'outcome', 'learning_points', 'difficulty']
for c in cases_list:
    vals = [
        c['id'], c.get('title'), c.get('species'), c.get('breed'),
        c.get('age'), c.get('weight'), c.get('chief_complaint'),
        c.get('history'), c.get('physical_exam'), c.get('lab_results'),
        c.get('imaging'), c.get('diagnosis'), c.get('treatment'),
        c.get('outcome'), c.get('learning_points'), c.get('difficulty', 'intermediate')
    ]
    L.append(INSERT_ROW('cases', case_cols, vals))
L.append('')

# ===== 病例-疾病关联 =====
L.append('-- ===== 病例-疾病关联 =====')
# 病例到疾病的映射
case_disease_map = {
    'case_001': ['dis_009'],   # 胰腺炎
    'case_002': ['dis_019'],   # 猫甲亢
    'case_003': ['dis_014'],   # DCM
    'case_004': ['dis_025'],   # 犬细小
    'case_005': ['dis_004'],   # 气管塌陷
    'case_006': ['dis_015'],   # 猫HCM
    'case_007': ['dis_029'],   # GDV
    'case_008': ['dis_018'],   # 库欣
    'case_009': ['dis_028'],   # FIP
    'case_010': ['dis_031'],   # 癫痫
    'case_011': ['dis_008'],   # 猫下尿路疾病/FIC
    'case_012': ['dis_018'],   # 库欣
    'case_013': ['dis_029'],   # GDV
    'case_014': ['dis_028'],   # FIP
    'case_015': ['dis_032'],   # 免疫介导性多关节炎
}
cd_count = 0
for case_id, disease_ids in case_disease_map.items():
    for did in disease_ids:
        L.append(
            f"INSERT INTO case_disease (case_id, disease_id) "
            f"VALUES ({S(case_id)}, {S(did)});"
        )
        cd_count += 1
L.append('')

# ── 写入文件 ───────────────────────────────────────────

os.makedirs(SEED_DIR, exist_ok=True)
with open(OUT, 'w', encoding='utf-8') as f:
    f.write('\n'.join(L))
    f.write('\n')

print(f'OK: wrote {len(L)} lines to {OUT}')
print(f'   Diseases: {len(diseases_list)}')
print(f'   Symptoms: {len(symptoms_list)}')
print(f'   Drugs: {len(drugs_list)}')
print(f'   Diagnostic tests: {len(tests_list)}')
print(f'   Disease-Symptom relations: {ds_count}')
print(f'   DDX relations: {len(ddx_list)}')
print(f'   Disease-Treatment relations: {len(dt_list)}')
print(f'   Disease-Diagnostic relations: {len(dd_list)}')
print(f'   Cases: {len(cases_list)}')
print(f'   Case-Disease relations: {cd_count}')
