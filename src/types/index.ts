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
  category: string | null
  species: string | null
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

export interface Symptom {
  id: string
  name_zh: string
  name_en: string | null
  definition: string | null
  species_notes: string | null
  physiological_basis: string | null
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

export interface TestSuggestion {
  test_id: string
  test_name: string
  purpose: string
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

export interface GameSnapshot {
  phase: string
  medical_phase: string
  time_elapsed_min: number
  time_budget_min: number
  time_remaining_min: number
  death_timer: number | null
  vitals: GameVitals
  active_signs: GameActiveSign[]
  new_reports: any[]
  pending_reports: number
  case?: GameCaseInfo
  error?: string
  action_started_at_s?: number
  state_time_s?: number
  time_cost_min?: number
  success?: boolean
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
  severity: number
  organ_system: string
  clue_id?: string
  localizing_value?: number
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
