#!/usr/bin/env python3
"""兽医知识库 SQL 种子数据生成器 — 从 Markdown+Frontmatter 数据源生成

读取 data/{diseases,symptoms,drugs,tests,cases,treatments}/*.md 的 frontmatter，
加上 data/relations.yaml 和 data/treatment_rules.yaml，
生成 src-tauri/data/seed/001_initial.sql。

添加新数据只需编辑对应目录下的 MD 文件，然后重新运行此脚本即可。
"""

import os
import re
import glob
import datetime
import json
import yaml
import frontmatter

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


def parse_body_sections(content):
    """从 MD body 中解析 ## 章节，返回 {标题: 内容} 字典"""
    sections = {}
    pattern = re.compile(r'^##\s+(.+?)\n(.*?)(?=\n##\s+|\Z)', re.MULTILINE | re.DOTALL)
    for m in pattern.finditer(content):
        title = m.group(1).strip()
        body = m.group(2).strip()
        sections[title] = body
    return sections


def load_md_entities(pattern, body_fields=None):
    """从 MD 文件目录加载所有实体，返回列表（按文件名排序）。

    body_fields: [(section_key, frontmatter_key), ...]
        指定从 MD body 的哪个章节提取内容，合并到 frontmatter 字段。
    """
    if body_fields is None:
        body_fields = []
    entities = []
    for path in sorted(glob.glob(pattern)):
        post = frontmatter.load(path)
        meta = dict(post.metadata)
        if body_fields:
            sections = parse_body_sections(post.content)
            for section_key, fm_key in body_fields:
                if section_key in sections:
                    meta[fm_key] = sections[section_key]
        entities.append(meta)
    return entities


# ── 加载数据 ──────────────────────────────────────────

# 疾病：body 章节 → frontmatter 字段映射
diseases_list = load_md_entities(os.path.join(DATA_DIR, 'diseases', '*.md'), [
    ('概述', 'overview'),
    ('病因', 'etiology'),
    ('病理生理', 'pathophysiology'),
    ('生理基础', 'physiological_basis'),
    ('预后', 'prognosis'),
])

# 症状：body 章节
symptoms_list = load_md_entities(os.path.join(DATA_DIR, 'symptoms', '*.md'), [
    ('定义', 'definition'),
    ('物种特异性', 'species_notes'),
    ('生理基础', 'physiological_basis'),
])

# 药物：body 章节
drugs_list = load_md_entities(os.path.join(DATA_DIR, 'drugs', '*.md'), [
    ('适应症', 'indications'),
    ('禁忌症', 'contraindications'),
    ('不良反应', 'side_effects'),
    ('不良反应机制', 'adverse_mechanism'),
    ('物种剂量', 'species_dosing'),
    ('作用机制', 'mechanism_of_action'),
    ('药代动力学', 'pk_pd'),
])

# 检查：body 章节
tests_list = load_md_entities(os.path.join(DATA_DIR, 'tests', '*.md'), [
    ('参考范围', 'reference_ranges'),
    ('结果解读', 'interpretation'),
])

# 病例：body 章节
cases_list = load_md_entities(os.path.join(DATA_DIR, 'cases', '*.md'), [
    ('主诉', 'chief_complaint'),
    ('病史', 'history'),
    ('体格检查', 'physical_exam'),
    ('实验室检查', 'lab_results'),
    ('影像学', 'imaging'),
    ('诊断', 'diagnosis'),
    ('治疗', 'treatment'),
    ('转归', 'outcome'),
    ('学习要点', 'learning_points'),
])

# 治疗：body 章节
treatments_list = load_md_entities(os.path.join(DATA_DIR, 'treatments', '*.md'), [
    ('治疗原则', 'principle'),
    ('操作指南', 'procedure_text'),
    ('生理基础', 'physiological_basis'),
    ('预后评估', 'prognosis_assessment'),
])

# 关系数据仍用 YAML
with open(os.path.join(DATA_DIR, 'relations.yaml'), 'r', encoding='utf-8') as f:
    relations = yaml.safe_load(f)

with open(os.path.join(DATA_DIR, 'treatment_rules.yaml'), 'r', encoding='utf-8') as f:
    treatment_rules = yaml.safe_load(f)

# ── 加载标签 ──────────────────────────────────────────

# 1) 从 tags.yaml 加载预置标签
tag_registry = {}
tags_yaml_path = os.path.join(DATA_DIR, 'tags.yaml')
if os.path.exists(tags_yaml_path):
    with open(tags_yaml_path, 'r', encoding='utf-8') as f:
        preset_tags = yaml.safe_load(f) or []
    for tag in preset_tags:
        tag_registry[tag['id']] = tag

# 2) 从实体 frontmatter.tags 收集标签（去重，不覆盖预置）
def collect_tags(entity_list, default_group='custom'):
    """从实体列表的 frontmatter['tags'] 收集标签"""
    for entity in entity_list:
        tags = entity.get('tags', [])
        if isinstance(tags, str):
            tags = [t.strip() for t in tags.split(',')]
        for tag_ref in tags:
            if not tag_ref:
                continue
            # 支持两种格式：tag_id（直接引用预置标签）或 纯文本（自动创建）
            tag_id = tag_ref.lower().replace(' ', '_').replace('#', '')
            if tag_id not in tag_registry:
                tag_registry[tag_id] = {
                    'id': tag_id,
                    'name_zh': tag_ref,
                    'tag_group': default_group,
                }

collect_tags(diseases_list, 'custom')
collect_tags(symptoms_list, 'custom')
collect_tags(drugs_list, 'custom')
collect_tags(treatments_list, 'custom')

# ── 生成 SQL ───────────────────────────────────────────

L = []
L.append('-- ============================================')
L.append('-- 兽医知识库 种子数据 v3.0 (Markdown+Frontmatter 驱动)')
L.append(f'-- {len(diseases_list)} 疾病 + {len(symptoms_list)} 症状 + {len(drugs_list)} 药物 + {len(tests_list)} 检查 + {len(cases_list)} 病例 + {len(treatments_list)} 治疗 + 标签系统 + 完整关联关系')
L.append(f'-- 生成时间: {datetime.datetime.now().strftime("%Y-%m-%d %H:%M:%S")}')
L.append('-- ============================================')
L.append('')

# ===== 标签 =====
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
            cols.append('name_en')
            vals.append(tag.get('name_en'))
        if has_color:
            cols.append('color')
            vals.append(tag.get('color'))
        L.append(INSERT_ROW('tags', cols, vals))
    else:
        L.append(
            f"INSERT INTO tags (id, name_zh, tag_group) "
            f"VALUES ({S(tag['id'])}, {S(tag['name_zh'])}, {S(tag['tag_group'])});"
        )
L.append('')

# ===== 疾病 =====
L.append(f'-- ===== 疾病 ({len(diseases_list)}种) =====')
disease_cols = ['id', 'name_zh', 'name_en', 'name_latin', 'category', 'species',
                'body_system', 'pathogenic_type', 'epidemiology', 'overview',
                'etiology', 'pathophysiology', 'physiological_basis', 'prognosis',
                'difficulty', 'urgency_level']
for d in diseases_list:
    vals = [
        d['id'], d['name_zh'], d['name_en'], d.get('name_latin'),
        to_json(d.get('category')), to_json(d.get('species')),
        d.get('body_system'), d.get('pathogenic_type'), d.get('epidemiology'),
        d.get('overview'), to_json(d.get('etiology')),
        d.get('pathophysiology'), d.get('physiological_basis'),
        d.get('prognosis'), d.get('difficulty', 'intermediate'), d.get('urgency_level', 3)
    ]
    L.append(INSERT_ROW('diseases', disease_cols, vals))
L.append('')

# ===== 疾病-标签关联 =====
L.append('-- ===== 疾病-标签关联 =====')
et_count = 0
for d in diseases_list:
    tags = d.get('tags', [])
    if isinstance(tags, str):
        tags = [t.strip() for t in tags.split(',')]
    for tag_name in tags:
        if not tag_name:
            continue
        tag_id = tag_name.lower().replace(' ', '_').replace('#', '')
        L.append(f"INSERT INTO entity_tags (entity_type, entity_id, tag_id) VALUES ('disease', {S(d['id'])}, {S(tag_id)});")
        et_count += 1
L.append('')

# ===== 症状 =====
L.append(f'-- ===== 症状 ({len(symptoms_list)}种) =====')
symptom_cols = ['id', 'name_zh', 'name_en', 'definition', 'species_notes', 'physiological_basis']
for s in symptoms_list:
    vals = [
        s['id'], s['name_zh'], s['name_en'],
        s.get('definition'), to_json(s.get('species_notes')), s.get('physiological_basis')
    ]
    L.append(INSERT_ROW('symptoms', symptom_cols, vals))
L.append('')

# ===== 症状-标签关联 =====
L.append('-- ===== 症状-标签关联 =====')
for s in symptoms_list:
    tags = s.get('tags', [])
    if isinstance(tags, str):
        tags = [t.strip() for t in tags.split(',')]
    for tag_name in tags:
        if not tag_name:
            continue
        tag_id = tag_name.lower().replace(' ', '_').replace('#', '')
        L.append(f"INSERT INTO entity_tags (entity_type, entity_id, tag_id) VALUES ('symptom', {S(s['id'])}, {S(tag_id)});")
        et_count += 1
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
drug_cols = ['id', 'name_zh', 'name_en', 'drug_class', 'mechanism_of_action', 'pk_pd',
             'indications', 'contraindications', 'side_effects', 'adverse_mechanism', 'species_dosing']
for d in drugs_list:
    vals = [
        d['id'], d['name_zh'], d['name_en'], d.get('drug_class'),
        d.get('mechanism_of_action'), d.get('pk_pd'),
        to_json(d.get('indications')), to_json(d.get('contraindications')),
        to_json(d.get('side_effects')), d.get('adverse_mechanism'),
        to_json(d.get('species_dosing'))
    ]
    L.append(INSERT_ROW('drugs', drug_cols, vals))
L.append('')

# ===== 药物-标签关联 =====
L.append('-- ===== 药物-标签关联 =====')
for d in drugs_list:
    tags = d.get('tags', [])
    if isinstance(tags, str):
        tags = [t.strip() for t in tags.split(',')]
    for tag_name in tags:
        if not tag_name:
            continue
        tag_id = tag_name.lower().replace(' ', '_').replace('#', '')
        L.append(f"INSERT INTO entity_tags (entity_type, entity_id, tag_id) VALUES ('drug', {S(d['id'])}, {S(tag_id)});")
        et_count += 1
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

# ===== 疾病-治疗方案关联 =====
L.append('-- ===== 疾病-治疗方案关联 =====')
dtm_list = treatment_rules.get('disease_treatment_map', [])
for r in dtm_list:
    species = r.get('species', '')
    notes = r.get('notes', '')
    if species and notes:
        L.append(
            f"INSERT INTO disease_treatment_map (disease_id,treatment_id,line,species,notes) "
            f"VALUES ({S(r['disease'])}, {S(r['treatment'])}, {S(r['line'])}, {S(species)}, {S(notes)});"
        )
    elif species:
        L.append(
            f"INSERT INTO disease_treatment_map (disease_id,treatment_id,line,species) "
            f"VALUES ({S(r['disease'])}, {S(r['treatment'])}, {S(r['line'])}, {S(species)});"
        )
    else:
        L.append(
            f"INSERT INTO disease_treatment_map (disease_id,treatment_id,line) "
            f"VALUES ({S(r['disease'])}, {S(r['treatment'])}, {S(r['line'])});"
        )
L.append('')

# ===== 治疗 =====
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

# ===== 治疗-标签关联 =====
L.append('-- ===== 治疗-标签关联 =====')
for t in treatments_list:
    tags = t.get('tags', [])
    if isinstance(tags, str):
        tags = [t.strip() for t in tags.split(',')]
    for tag_name in tags:
        if not tag_name:
            continue
        tag_id = tag_name.lower().replace(' ', '_').replace('#', '')
        L.append(f"INSERT INTO entity_tags (entity_type, entity_id, tag_id) VALUES ('treatment', {S(t['id'])}, {S(tag_id)});")
        et_count += 1
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
case_disease_map = {
    'case_001': ['dis_009'],
    'case_002': ['dis_019'],
    'case_003': ['dis_014'],
    'case_004': ['dis_025'],
    'case_005': ['dis_004'],
    'case_006': ['dis_015'],
    'case_007': ['dis_029'],
    'case_008': ['dis_018'],
    'case_009': ['dis_028'],
    'case_010': ['dis_031'],
    'case_011': ['dis_008'],
    'case_012': ['dis_018'],
    'case_013': ['dis_029'],
    'case_014': ['dis_028'],
    'case_015': ['dis_032'],
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
print(f'   Tags: {len(tag_registry)}')
print(f'   Entity-Tag relations: {et_count}')
print(f'   Diseases: {len(diseases_list)}')
print(f'   Symptoms: {len(symptoms_list)}')
print(f'   Drugs: {len(drugs_list)}')
print(f'   Diagnostic tests: {len(tests_list)}')
print(f'   Disease-Symptom relations: {ds_count}')
print(f'   DDX relations: {len(ddx_list)}')
print(f'   Disease-Treatment relations: {len(dt_list)}')
print(f'   Disease-Diagnostic relations: {len(dd_list)}')
print(f'   Disease-TreatmentMap relations: {len(dtm_list)}')
print(f'   Treatments: {len(treatments_list)}')
print(f'   Cases: {len(cases_list)}')
print(f'   Case-Disease relations: {cd_count}')
