export interface HandoffEvalCase {
  id: string;
  title?: string;
  source_agent: string;
  target_agent: string;
  repo_name?: string;
  outcome: 'passed' | 'failed' | 'unknown' | string;
  score_pct?: number;
  trust_level?: 'exact' | 'estimated' | 'api' | 'unknown' | string;
  duration_ms?: number;
  summary?: string;
  packet_score_pct?: number;
}

export interface HandoffEvalProvider {
  target_agent: string;
  cases?: number;
  passed_cases?: number;
  pass_rate_pct?: number;
  mean_score_pct?: number;
}

export interface HandoffEvalSummary {
  total_cases?: number;
  passed_cases?: number;
  pass_rate_pct?: number;
  mean_score_pct?: number;
  trust_level?: 'exact' | 'estimated' | 'api' | 'unknown' | string;
}

export interface HandoffEvalReport {
  updated_at?: string;
  summary?: HandoffEvalSummary;
  by_provider?: HandoffEvalProvider[];
  cases?: HandoffEvalCase[];
}

export function renderHandoffEvalMarkup(report: HandoffEvalReport | null | undefined): string;
