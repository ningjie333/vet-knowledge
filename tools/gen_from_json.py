#!/usr/bin/env python3
"""兽医知识库 SQL 种子数据生成器 — 从统一 JSON 数据源生成

读取 data/knowledge.json，生成 src-tauri/data/seed/001_initial.sql。

添加新数据只需编辑 data/knowledge.json，然后重新运行此脚本即可。
"""

import os
import re
import json

BASE_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DATA_DIR = os.path.join(BASE_DIR, 'data')
SEED_DIR = os.path.join(BASE_DIR, 'src-tauri', 'data', 'seed')
OUT = os.path.join(SEED_DIR, '001_initial.sql')


def S(s):
    """转义字符串为 SQL 文本字面量"""
    if s is None:
        return 'NULL'
    # 单引号转义 + 移除 NUL 字节（SQLite 字符串终止符）
    cleaned = str(s).replace("'", "''").replace("\x00", "")
    return "'" + cleaned + "'"


def V(v):
    """任意值 → SQL 字面量"""
    if v is None:
        return 'NULL'
    if isinstance(v, (int, float)):
        return str(v)
    return S(v)


def parse_list_text(text):
    """将 '- item1\n- item2' 格式的文本解析为列表"""
    if not isinstance(text, str):
        return text
    lines = text.strip().split('\n')
    items = []
    for line in lines:
        stripped = line.strip()
        if stripped.startswith('- '):
            items.append(stripped[2:].strip())
        elif stripped.startswith('* '):
            items.append(stripped[2:].strip())
    return items if items else text


def parse_species_dict(text):
    """将 '**犬**: xxx\n**猫**: xxx' 或 '犬：xxx；猫：xxx' 格式解析为字典"""
    if not isinstance(text, str):
        return text
    result = {}
    bold_pattern = re.compile(r'\*\*(犬|猫|其他)\*\*[：:]\s*(.+?)(?=\n\*\*|$)', re.DOTALL)
    matches = bold_pattern.findall(text)
    if matches:
        for species, content in matches:
            result[species] = content.strip()
        return result if result else text
    colon_pattern = re.compile(r'(犬|猫)[：:]\s*(.+?)(?=；(?:犬|猫)[：:]|$)', re.DOTALL)
    matches = colon_pattern.findall(text)
    if matches:
        for species, content in matches:
            result[species] = content.strip()
        return result if result else text
    return text


def to_json(v):
    """Python 列表/字典 → JSON 字符串"""
    if v is None:
        return None
    if isinstance(v, str):
        if '\n- ' in v or v.startswith('- '):
            parsed = parse_list_text(v)
            if isinstance(parsed, list):
                return json.dumps(parsed, ensure_ascii=False)
        if '**犬**' in v or '**猫**' in v or \
           (('犬：' in v or '犬:' in v) and ('猫：' in v or '猫:' in v)):
            parsed = parse_species_dict(v)
            if isinstance(parsed, dict):
                return json.dumps(parsed, ensure_ascii=False)
    if isinstance(v, (list, dict)):
        return json.dumps(v, ensure_ascii=False)
    return v


def INSERT_ROW(table, columns, values):
    """生成单行 INSERT"""
    cols = ', '.join(columns)
    vals = ', '.join(V(v) for v in values)
    return f"INSERT INTO {table} ({cols}) VALUES ({vals});"


def generate_from_json(json_path):
    """从 knowledge.json 生成 SQL，返回 (lines_dict, stats)。
    lines_dict 按表名分组，方便调试。
    """
    with open(json_path, 'r', encoding='utf-8') as f:
        data = json.load(f)

    lines = []  # 最终有序输出
    stats = {}

    # ── 标签（从 knowledge.json 的疾病 tags 收集）──
    tag_registry = {}
    tag_name_to_id = {}

    def resolve_tag_id(tag_ref):
        if not tag_ref:
            return ''
        if tag_ref in tag_registry:
            return tag_ref
        if tag_ref in tag_name_to_id:
            return tag_name_to_id[tag_ref]
        return tag_ref.lower().replace(' ', '_').replace('#', '')

    # 从疾病 tags 收集标签（预置标签从 tags.yaml 加载，若存在）
    tags_yaml_path = os.path.join(DATA_DIR, 'tags.yaml')
    if os.path.exists(tags_yaml_path):
        import yaml
        with open(tags_yaml_path, 'r', encoding='utf-8') as f:
            preset_tags = yaml.safe_load(f) or []
        for tag in preset_tags:
            tag_registry[tag['id']] = tag
            tag_name_to_id[tag['name_zh']] = tag['id']

    # 收集疾病中引用的标签
    for dis in data.get('diseases', []):
        for tag_ref in dis.get('tags', []):
            tag_id = resolve_tag_id(tag_ref)
            if tag_id not in tag_registry:
                tag_registry[tag_id] = {'id': tag_id, 'name_zh': tag_ref, 'tag_group': 'custom'}

    # ── 标签 ──
    L = []
    L.append(f'-- ===== 标签 ({len(tag_registry)}个) =====')
    for tag in sorted(tag_registry.values(), key=lambda t: t['id']):
        has_emergency = tag.get('emergency_level') is not None
        has_color = tag.get('color') is not None
        has_en = tag.get('name_en') is not None
        if has_emergency:
            L.append(
                f"INSERT INTO tags (id, name_zh, name_en, tag_group, emergency_level, "
                f"clinical_action, textbook_logic, typical_scenario, color) "
                f"VALUES ({S(tag['id'])}, {S(tag['name_zh'])}, {S(tag.get('name_en'))}, "
                f"{S(tag['tag_group'])}, {S(tag.get('emergency_level'))}, "
                f"{S(tag.get('clinical_action'))}, {S(tag.get('textbook_logic'))}, "
                f"{S(tag.get('typical_scenario'))}, {S(tag.get('color'))});"
            )
        elif has_color or has_en:
            cols = ['id', 'name_zh', 'tag_group']
            vals = [tag['id'], tag['name_zh'], tag['tag_group']]
            if has_en:
                cols.append('name_en'); vals.append(tag.get('name_en'))
            if has_color:
                cols.append('color'); vals.append(tag.get('color'))
            L.append(INSERT_ROW('tags', cols, vals))
        else:
            L.append(
                f"INSERT INTO tags (id, name_zh, tag_group) "
                f"VALUES ({S(tag['id'])}, {S(tag['name_zh'])}, {S(tag['tag_group'])});"
            )
    L.append('')
    lines.extend(L)
    stats['tags'] = len(tag_registry)

    # ── 症状 ──
    L = []
    symptoms_list = data.get('symptoms', [])
    L.append(f'-- ===== 症状 ({len(symptoms_list)}种) =====')
    symptom_cols = ['id', 'name_zh', 'name_en', 'definition', 'species_notes', 'physiological_basis']
    for s in symptoms_list:
        vals = [
            s['id'], s['name_zh'], s.get('name_en'),
            s.get('definition'), to_json(s.get('species_notes')), s.get('physiological_basis')
        ]
        L.append(INSERT_ROW('symptoms', symptom_cols, vals))
    L.append('')
    lines.extend(L)
    stats['symptoms'] = len(symptoms_list)

    # ── 药物 ──
    L = []
    drugs_list = data.get('drugs', [])
    L.append(f'-- ===== 药物 ({len(drugs_list)}种) =====')
    drug_cols = ['id', 'name_zh', 'name_en', 'drug_class', 'mechanism_of_action', 'pk_pd',
                 'indications', 'contraindications', 'side_effects', 'adverse_mechanism', 'species_dosing']
    for d in drugs_list:
        vals = [
            d['id'], d['name_zh'], d.get('name_en'), d.get('drug_class'),
            d.get('mechanism_of_action'), d.get('pk_pd'),
            to_json(d.get('indications')), to_json(d.get('contraindications')),
            to_json(d.get('side_effects')), d.get('adverse_mechanism'),
            to_json(d.get('species_dosing'))
        ]
        L.append(INSERT_ROW('drugs', drug_cols, vals))
    L.append('')
    lines.extend(L)
    stats['drugs'] = len(drugs_list)

    # ── 检查 ──
    L = []
    tests_list = data.get('tests', [])
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
    lines.extend(L)
    stats['tests'] = len(tests_list)

    # ── 治疗方案 ──
    L = []
    treatments_list = data.get('treatments', [])
    L.append(f'-- ===== 治疗 ({len(treatments_list)}个) =====')
    treatment_cols = ['id', 'name_zh', 'name_en', 'therapy_type', 'principle',
                      'procedure_text', 'physiological_basis', 'prognosis_assessment']
    for t in treatments_list:
        vals = [
            t['id'], t['name_zh'], t.get('name_en'), t.get('therapy_type'),
            t.get('principle'), t.get('procedure_text'),
            t.get('physiological_basis'), t.get('prognosis_assessment')
        ]
        L.append(INSERT_ROW('treatments', treatment_cols, vals))
    L.append('')
    lines.extend(L)
    stats['treatments'] = len(treatments_list)

    # ── 疾病 ──
    L = []
    diseases_list = data.get('diseases', [])
    L.append(f'-- ===== 疾病 ({len(diseases_list)}种) =====')
    disease_cols = ['id', 'name_zh', 'name_en', 'name_latin', 'category', 'species',
                    'body_system', 'pathogenic_type', 'epidemiology', 'overview',
                    'etiology', 'pathophysiology', 'physiological_basis', 'prognosis',
                    'difficulty', 'urgency_level']
    for d in diseases_list:
        vals = [
            d['id'], d['name_zh'], d.get('name_en'), d.get('name_latin'),
            to_json(d.get('category')), to_json(d.get('species')),
            d.get('body_system'), d.get('pathogenic_type'), d.get('epidemiology'),
            d.get('overview'), to_json(d.get('etiology')),
            d.get('pathophysiology'), d.get('physiological_basis'),
            d.get('prognosis'), d.get('difficulty', 'intermediate'), d.get('urgency_level', 3)
        ]
        L.append(INSERT_ROW('diseases', disease_cols, vals))
    L.append('')
    lines.extend(L)
    stats['diseases'] = len(diseases_list)

    # ── 疾病-标签关联 ──
    L = []
    L.append('-- ===== 疾病-标签关联 =====')
    et_count = 0
    for d in diseases_list:
        for tag_name in d.get('tags', []):
            tag_id = resolve_tag_id(tag_name)
            L.append(f"INSERT INTO entity_tags (entity_type, entity_id, tag_id) VALUES ('disease', {S(d['id'])}, {S(tag_id)});")
            et_count += 1
    L.append('')
    lines.extend(L)
    stats['entity_tags'] = et_count

    # ── 疾病-症状关联 ──
    L = []
    L.append('-- ===== 疾病-症状关联 =====')
    ds_count = 0
    for d in diseases_list:
        for e in d.get('symptoms', []):
            symptom_id = e['symptom']
            freq = e.get('frequency', 'common')
            stage = e.get('stage', '')
            is_patho = e.get('is_pathognomonic', False)
            if is_patho:
                L.append(
                    f"INSERT INTO disease_symptom (disease_id,symptom_id,frequency,stage,is_pathognomonic) "
                    f"VALUES ({S(d['id'])}, {S(symptom_id)}, {S(freq)}, {S(stage)}, 1);"
                )
            else:
                L.append(
                    f"INSERT INTO disease_symptom (disease_id,symptom_id,frequency,stage) "
                    f"VALUES ({S(d['id'])}, {S(symptom_id)}, {S(freq)}, {S(stage)});"
                )
            ds_count += 1
    L.append('')
    lines.extend(L)
    stats['disease_symptoms'] = ds_count

    # ── 疾病-药物关联 ──
    L = []
    L.append('-- ===== 疾病-治疗关联 =====')
    dt_count = 0
    for d in diseases_list:
        for e in d.get('drugs', []):
            species = e.get('species', '')
            notes = e.get('notes', '')
            if species and notes:
                L.append(
                    f"INSERT INTO disease_treatment (disease_id,drug_id,line,species,notes) "
                    f"VALUES ({S(d['id'])}, {S(e['drug'])}, {S(e.get('line', 'first'))}, {S(species)}, {S(notes)});"
                )
            elif species:
                L.append(
                    f"INSERT INTO disease_treatment (disease_id,drug_id,line,species) "
                    f"VALUES ({S(d['id'])}, {S(e['drug'])}, {S(e.get('line', 'first'))}, {S(species)});"
                )
            else:
                L.append(
                    f"INSERT INTO disease_treatment (disease_id,drug_id,line) "
                    f"VALUES ({S(d['id'])}, {S(e['drug'])}, {S(e.get('line', 'first'))});"
                )
            dt_count += 1
    L.append('')
    lines.extend(L)
    stats['disease_drugs'] = dt_count

    # ── 疾病-诊断关联 ──
    L = []
    L.append('-- ===== 疾病-诊断关联 =====')
    dd_count = 0
    for d in diseases_list:
        for e in d.get('diagnostics', []):
            species = e.get('species', '')
            expected = e.get('expected_result', '')
            if species and expected:
                L.append(
                    f"INSERT INTO disease_diagnostic (disease_id,test_id,purpose,evidence_level,species,expected_result) "
                    f"VALUES ({S(d['id'])}, {S(e['test'])}, {S(e.get('purpose', 'screening'))}, {S(e.get('evidence_level', 'supportive'))}, {S(species)}, {S(expected)});"
                )
            elif species:
                L.append(
                    f"INSERT INTO disease_diagnostic (disease_id,test_id,purpose,evidence_level,species) "
                    f"VALUES ({S(d['id'])}, {S(e['test'])}, {S(e.get('purpose', 'screening'))}, {S(e.get('evidence_level', 'supportive'))}, {S(species)});"
                )
            else:
                L.append(
                    f"INSERT INTO disease_diagnostic (disease_id,test_id,purpose,evidence_level) "
                    f"VALUES ({S(d['id'])}, {S(e['test'])}, {S(e.get('purpose', 'screening'))}, {S(e.get('evidence_level', 'supportive'))});"
                )
            dd_count += 1
    L.append('')
    lines.extend(L)
    stats['disease_diagnostics'] = dd_count

    # ── 疾病-治疗方案关联 ──
    L = []
    L.append('-- ===== 疾病-治疗方案关联 =====')
    dtm_count = 0
    for d in diseases_list:
        for e in d.get('treatments', []):
            species = e.get('species', '')
            notes = e.get('notes', '')
            if species and notes:
                L.append(
                    f"INSERT INTO disease_treatment_map (disease_id,treatment_id,line,species,notes) "
                    f"VALUES ({S(d['id'])}, {S(e['treatment'])}, {S(e.get('line', 'first'))}, {S(species)}, {S(notes)});"
                )
            elif species:
                L.append(
                    f"INSERT INTO disease_treatment_map (disease_id,treatment_id,line,species) "
                    f"VALUES ({S(d['id'])}, {S(e['treatment'])}, {S(e.get('line', 'first'))}, {S(species)});"
                )
            else:
                L.append(
                    f"INSERT INTO disease_treatment_map (disease_id,treatment_id,line) "
                    f"VALUES ({S(d['id'])}, {S(e['treatment'])}, {S(e.get('line', 'first'))});"
                )
            dtm_count += 1
    L.append('')
    lines.extend(L)
    stats['disease_treatment_maps'] = dtm_count

    # ── 鉴别诊断 ──
    L = []
    L.append('-- ===== 鉴别诊断关联 =====')
    ddx_count = 0
    for d in diseases_list:
        for e in d.get('ddx', []):
            L.append(
                f"INSERT INTO disease_ddx (disease_id,ddx_id,distinguishing_feature) "
                f"VALUES ({S(d['id'])}, {S(e['ddx'])}, {S(e.get('feature', ''))});"
            )
            ddx_count += 1
    L.append('')
    lines.extend(L)
    stats['ddx'] = ddx_count

    # ── 病例 ──
    L = []
    cases_list = data.get('cases', [])
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
    lines.extend(L)
    stats['cases'] = len(cases_list)

    # ── 病例-疾病关联 ──
    L = []
    L.append('-- ===== 病例-疾病关联 =====')
    cd_count = 0
    for c in cases_list:
        for did in c.get('diseases', []):
            L.append(
                f"INSERT INTO case_disease (case_id, disease_id) "
                f"VALUES ({S(c['id'])}, {S(did)});"
            )
            cd_count += 1
    L.append('')
    lines.extend(L)
    stats['case_diseases'] = cd_count

    # ── 药物相互作用 ──
    L = []
    interactions_list = data.get('drug_interactions', [])
    L.append(f'-- ===== 药物相互作用 ({len(interactions_list)}条) =====')
    for ix in interactions_list:
        L.append(
            f"INSERT INTO drug_interactions (drug_a_id,drug_b_id,severity,description) "
            f"VALUES ({S(ix['drug_a'])}, {S(ix['drug_b'])}, {S(ix.get('severity', 'moderate'))}, {S(ix.get('description', ''))});"
        )
    L.append('')
    lines.extend(L)
    stats['drug_interactions'] = len(interactions_list)

    return lines, stats


# ── 主流程 ──────────────────────────────────────────────

json_path = os.path.join(DATA_DIR, 'knowledge.json')
if not os.path.exists(json_path):
    print(f'[ERROR] {json_path} not found. Please create it first.')
    exit(1)

print('[MODE] JSON import (data/knowledge.json)')
L, stats = generate_from_json(json_path)

# ── 写入文件 ───────────────────────────────────────────

os.makedirs(SEED_DIR, exist_ok=True)
with open(OUT, 'w', encoding='utf-8') as f:
    f.write('\n'.join(L))
    f.write('\n')

print(f'OK: wrote {len(L)} lines to {OUT}')
for k, v in stats.items():
    print(f'   {k}: {v}')
