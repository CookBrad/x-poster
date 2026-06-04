import { invoke } from '@tauri-apps/api/core'

export interface Draft {
  id: string
  text: string
  sources_json: string
  image_url: string | null
  status: 'pending' | 'posted' | 'skipped'
  created_at: string
  updated_at: string
  posted_at: string | null
  x_post_id: string | null
}

export interface CreateDraftInput {
  text: string
  sources_json: string
  image_url?: string | null
}

export interface UpdateDraftInput {
  text?: string
  image_url?: string | null
  status?: 'pending' | 'posted' | 'skipped'
}

/**
 * Create a new draft
 */
export async function createDraft(input: CreateDraftInput): Promise<Draft> {
  return invoke<Draft>('create_draft', { input })
}

/**
 * Get all drafts (optionally filtered by status)
 */
export async function getDrafts(status?: Draft['status']): Promise<Draft[]> {
  return invoke<Draft[]>('get_drafts', { status })
}

/**
 * Get a single draft by ID
 */
export async function getDraft(id: string): Promise<Draft | null> {
  return invoke<Draft | null>('get_draft', { id })
}

/**
 * Update a draft
 */
export async function updateDraft(id: string, input: UpdateDraftInput): Promise<void> {
  return invoke('update_draft', { id, input })
}

/**
 * Delete a draft or posted item.
 * For posted items this removes the local record only (does not delete from X).
 */
export async function deleteDraft(id: string): Promise<void> {
  return invoke('delete_draft', { id })
}

/**
 * Mark a draft as successfully posted to X
 */
export async function markDraftPosted(id: string, xPostId: string): Promise<void> {
  return invoke('mark_draft_posted', { id, xPostId })
}

// ============================================
// Research (T-003 / T-004)
// ============================================

export interface ResearchSource {
  id: string;
  title: string;
  content: string;
  url: string;
  published_at: string | null;
  source_name: string;
  source_type: string;
  // Optional engagement data (present on X-sourced items)
  retweet_count?: number;
  like_count?: number;
  reply_count?: number;
  quote_count?: number;
  // The original identifier from the source (useful when the row id is a surrogate)
  original_id?: string;
}

export async function fetchResearchSources(): Promise<ResearchSource[]> {
  return invoke('fetch_research_sources');
}

/**
 * Helper to parse sources from a draft
 */
export function parseSources(draft: Draft): any[] {
  try {
    return JSON.parse(draft.sources_json || '[]')
  } catch {
    return []
  }
}

// ============================================
// Research Runs (Current + Historical)
// ============================================

export interface ResearchRun {
  id: string;
  run_at: string;
  source: string;
}

export interface ResearchRunWithSources {
  run: ResearchRun;
  sources: ResearchSource[];
}

export async function runResearch(mode: 'rss' | 'x' | 'both' = 'both'): Promise<ResearchRunWithSources> {
  return invoke<ResearchRunWithSources>('run_research', { mode });
}

export async function getLatestResearchRun(): Promise<ResearchRunWithSources | null> {
  return invoke<ResearchRunWithSources | null>('get_latest_research_run');
}

export async function getResearchRuns(): Promise<ResearchRun[]> {
  return invoke<ResearchRun[]>('get_research_runs');
}

export async function getResearchRun(runId: string): Promise<ResearchRunWithSources | null> {
  return invoke<ResearchRunWithSources | null>('get_research_run', { run_id: runId });
}

export interface HistoricalResearchSource extends ResearchSource {
  run_at: string;
}

export async function getAllHistoricalSources(): Promise<HistoricalResearchSource[]> {
  return invoke<HistoricalResearchSource[]>('get_all_historical_sources');
}

export interface ResetResearchResult {
  deleted_sources: number;
  deleted_runs: number;
}

/**
 * Permanently deletes all research runs and sources from the local database.
 * This cannot be undone. Use with caution (UI should show a warning prompt).
 */
export async function resetResearchData(): Promise<ResetResearchResult> {
  return invoke<ResetResearchResult>('reset_research_data', {});
}