export interface Tag {
  id: string
  name_zh: string
  name_en: string | null
  tag_group: string
  emergency_level: string | null
  clinical_action: string | null
  textbook_logic: string | null
  typical_scenario: string | null
  color: string | null
}

export interface Disease {
  id: string
  name_zh: string
  name_en: string | null
  name_latin: string | null
  category: string[]
  species: string[]
  overview: string | null
  etiology: string | null
  pathophysiology: string | null
  prognosis: string | null
  difficulty: string | null
  urgency_level: number | null
  pathogenic_type: string | null
  epidemiology: string | null
  body_system: string | null
  physiological_basis: string | null
  tags: string[]
}

/**
 * 症状的物种差异备注条目。
 * 对应 Rust `pub struct SpeciesNote`（models.rs）。DB 列原本是 JSON 对象
 * `{"犬": "...", "猫": "..."}`，后端 `parse_species_notes` 已解析为结构化数组下发。
 */
export interface SpeciesNote {
  species: string
  note: string
}

/**
 * 症状实体（后端 src-tauri/src/db/models.rs 的 `pub struct Symptom` + row_from_symptom()）。
 * 字段名与 serde 默认输出一致（snake_case，无 rename_all）。
 */
export interface Symptom {
  id: string
  name_zh: string
  name_en: string | null
  definition: string | null
  /** 物种差异备注，后端已解析为结构化列表（空数组表示无），前端逐条渲染 */
  species_notes: SpeciesNote[]
  physiological_basis: string | null
  /** 标签 id 列表（不是标签名），由 tags_subquery() 提供，需回查 get_tags() 取显示名/颜色 */
  tags: string[]
}

export interface Drug {
  id: string
  name_zh: string
  name_en: string | null
  drug_class: string | null
  indications: string | null
  contraindications: string | null
  side_effects: string | null
  species_dosing: string | null
  mechanism_of_action: string | null
  pk_pd: string | null
  adverse_mechanism: string | null
  tags: string[]
  pubchem_cid: number | null
  molecular_formula: string | null
  molecular_weight: number | null
  smiles: string | null
  inchikey: string | null
  iupac_name: string | null
}

export interface Treatment {
  id: string
  name_zh: string
  name_en: string | null
  therapy_type: string | null
  principle: string | null
  procedure_text: string | null
  physiological_basis: string | null
  prognosis_assessment: string | null
  tags: string[]
}

export interface DiagnosticTest {
  id: string
  name_zh: string
  category: string | null
  reference_ranges: string | null
  interpretation: string | null
  cost_estimate: number | null
  turnaround_time: number | null
}

// ===== 实体关联视图（JOIN 结果的统一类型）=====
// 后端有两类返回形状，必须区分清楚，否则界面会静默渲染空白：
//   1) Rust 命名结构体（如 models.rs 的 DiseaseWithSymptom）→ JSON 对象，前端用 r.disease / r.frequency
//   2) Rust 元组 Vec<(A, B, C)>（commands/*.rs 里大量 JOIN 查询）→ serde 序列化为 JSON **数组**，
//      前端只能按下标取值（A = t[0]、B = t[1]…），不能写成对象属性
// 下面的元组类型（tuple type）就是第 2 类的显式契约，顺序与 Rust 端一一对应。

/** get_disease_symptoms / DiseaseCompareView.symptoms：[症状, frequency, stage, is_pathognomonic] */
export type DiseaseSymptomRow = [Symptom, string, string, number]

/** get_disease_treatments / DiseaseCompareView.treatments：[药物, line, species, notes] */
export type DiseaseDrugRow = [Drug, string, string, string]

/**
 * get_disease_diagnostics / DiseaseCompareView.diagnostics：
 * [检查项目, purpose, evidence_level, species, expected_result]
 */
export type DiseaseDiagnosticRow = [DiagnosticTest, string, string, string, string]

/** get_disease_ddx / DiseaseCompareView.ddx：[需鉴别的疾病, distinguishing_feature] */
export type DiseaseDdxRow = [Disease, string]

/** get_disease_treatment_map：[疗法, line, species, notes] */
export type DiseaseTreatmentRow = [Treatment, string, string, string]

/**
 * get_diseases_by_symptom 的单条结果。
 * 对应 Rust `pub struct DiseaseWithSymptom`（models.rs），命名结构体 → JSON 对象。
 */
export interface DiseaseWithSymptom {
  disease: Disease
  frequency: string
  stage: string
  is_pathognomonic: number
}

/**
 * get_disease_compare 的单条结果。
 * 对应 Rust `pub struct DiseaseCompareView`（commands/knowledge.rs）。
 * 注意：除 disease 外的四个字段在 Rust 侧是元组 Vec，序列化后是"数组的数组"。
 */
export interface DiseaseCompareView {
  disease: Disease
  symptoms: DiseaseSymptomRow[]
  treatments: DiseaseDrugRow[]
  diagnostics: DiseaseDiagnosticRow[]
  ddx: DiseaseDdxRow[]
}

export interface Case {
  id: string
  title: string
  species: string | null
  breed: string | null
  age: number | null
  weight: number | null
  chief_complaint: string | null
  history: string | null
  physical_exam: string | null
  lab_results: string | null
  imaging: string | null
  diagnosis: string | null
  treatment: string | null
  outcome: string | null
  learning_points: string | null
  difficulty: string | null
}

// ===== 闪卡（SM-2 复习）=====

/**
 * 闪卡记录，对应 Rust `pub struct Flashcard`（commands/flashcards.rs）。
 * difficulty / ease_factor 在 Rust 侧是 f64，review_count 是 COALESCE 后的 i64（非空）。
 */
export interface Flashcard {
  id: string
  front: string
  back: string
  card_type: string
  entity_id: string | null
  difficulty: number
  created_at: string
  next_review: string | null
  review_count: number
  ease_factor: number
}

/** 复习统计，对应 Rust `pub struct ReviewStats`（commands/flashcards.rs 的 get_review_stats） */
export interface ReviewStats {
  total_cards: number
  due_today: number
  reviewed_today: number
  mastered: number
}

export interface SearchResult {
  entity_type: string
  entity_id: string
  title: string
  snippet: string
  relevance: number
}

export interface DiagnosisCandidate {
  disease_id: string
  disease_name: string
  match_score: number
  input_coverage: number
  matched_symptoms: string[]
  missing_key_symptoms: string[]
  suggested_tests: TestSuggestion[]
  distinguishing_features: string[]
}

export interface DrugInfo {
  drug_name: string
  name: string
  half_life_h: number
  description: string
}

export interface TestSuggestion {
  test_id: string
  test_name: string
  purpose: string
}

export interface DrugInteraction {
  drug_id: string
  drug_name: string
  severity: 'major' | 'moderate' | 'mild'
  description: string
}

// ===== 诊断游戏接口 =====

export interface GameCaseSummary {
  id: string
  title: string
  difficulty: number
  difficulty_label: string
  species: string
  breed: string
  age: string
  weight_kg: number
  chief_complaint: string
}

/** 检查报告里的单项结果（virtual-vet 原始 JSON，字段随检查项目而变） */
export interface GameReportResult {
  param?: string
  value?: unknown
  unit?: string
  normal_range?: string
  flag?: string
}

/**
 * GameSnapshot.new_reports 的元素类型。
 * Rust 侧是 `Vec<serde_json::Value>`（commands/game.rs），即 sidecar 原始 JSON 直接透传、没有固定 schema，
 * 因此这里只声明界面真正读取的字段，其余键留在索引签名里 —— 免得整条链路退化成 any（正是 lint 要拦的）。
 */
export interface GameReport {
  test_type?: string
  summary?: string
  results?: GameReportResult[]
  [key: string]: unknown
}

export interface GameSnapshot {
  phase: string
  medical_phase: string
  time_elapsed_min: number
  time_budget_min: number
  time_remaining_min: number
  death_timer: number | null
  vitals: GameVitals
  active_signs: GameActiveSign[]
  new_reports: GameReport[]
  pending_reports: number
  case?: GameCaseInfo
  error?: string
  action_started_at_s?: number
  state_time_s?: number
  time_cost_min?: number
  success?: boolean
  message?: string
}

export interface GameVitals {
  hr_bpm: number
  map_mmhg: number
  spo2_pct: number
  rr_bpm: number
  temp_c: number
  gfr_ml_min: number
  ph: number
  co_ml_min: number
  blood_volume_ml: number
  lactate_mmol_l: number
  bun_mg_dl: number
  game_time: string
  is_night: boolean
}

export interface GameActiveSign {
  sign_id: string
  display_name: string
  severity: string
  organ_system: string
  clue_id?: string
  localizing_value?: string
}

export interface GameCaseInfo {
  id: string
  title: string
  difficulty: number
  difficulty_label: string
  animal: GameAnimal
  chief_complaint: string
  history: string
  time_budget_min: number
  starting_hints: string[]
}

export interface GameAnimal {
  species: string
  breed: string
  name: string
  age: string
  weight_kg: number
  sex: string
}

export interface NewSessionResponse {
  session_id: string
  initial_snapshot: GameSnapshot
}

// ===== 异步推进接口（阶段 3）=====

export interface AdvanceJobInfo {
  job_id: string
  estimated_ms: number
}

export interface AdvancePollResult {
  done: boolean
  progress_pct: number
  snapshot: GameSnapshot | null
  error: string | null
}
