import { invoke } from '@tauri-apps/api/core'
import { DEFAULT_DRAFT_GENERATION_COUNT, type DraftStatus } from './constants'

export interface Draft {
  id: string
  text: string
  sources_json: string
  image_url: string | null
  status: DraftStatus
  created_at: string
  updated_at: string
  posted_at: string | null
  x_post_id: string | null
}

export interface DraftSource {
  type?: string
  source_type?: string
  user?: string
  source?: string
  source_name?: string
  title?: string
  text?: string
}

export interface CreateDraftInput {
  text: string
  sources_json: string
  image_url?: string | null
}

export interface UpdateDraftInput {
  text?: string
  image_url?: string | null
  status?: DraftStatus
}

export async function createDraft(input: CreateDraftInput): Promise<Draft> {
  return invoke<Draft>('create_draft', { input })
}

export async function getDrafts(status?: DraftStatus): Promise<Draft[]> {
  return invoke<Draft[]>('get_drafts', { status })
}

export async function getDraft(id: string): Promise<Draft | null> {
  return invoke<Draft | null>('get_draft', { id })
}

export async function updateDraft(id: string, input: UpdateDraftInput): Promise<void> {
  return invoke('update_draft', { id, input })
}

export async function deleteDraft(id: string): Promise<void> {
  return invoke('delete_draft', { id })
}

export interface ClearPendingDraftsResult {
  deleted: number
}

export async function clearPendingDrafts(): Promise<ClearPendingDraftsResult> {
  return invoke<ClearPendingDraftsResult>('clear_pending_drafts', {})
}

export async function markDraftPosted(id: string, xPostId: string): Promise<void> {
  return invoke('mark_draft_posted', { id, xPostId })
}

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

export function parseSources(draft: Draft): DraftSource[] {
  try {
    const parsed: unknown = JSON.parse(draft.sources_json || '[]')
    return Array.isArray(parsed) ? (parsed as DraftSource[]) : []
  } catch {
    return []
  }
}

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

export async function resetResearchData(): Promise<ResetResearchResult> {
  return invoke<ResetResearchResult>('reset_research_data', {});
}

export async function generateDraftsFromLatestResearch(
  count = DEFAULT_DRAFT_GENERATION_COUNT
): Promise<Draft[]> {
  return invoke<Draft[]>('generate_drafts_from_latest_research', { count });
}

export async function generateDraftFromSource(
  sourceId: string,
  count = 1
): Promise<Draft[]> {
  return invoke<Draft[]>('generate_draft_from_source', { sourceId, count });
}

export async function postDraftToX(id: string): Promise<Draft> {
  return invoke<Draft>('post_draft_to_x', { id });
}

export async function hasXCredentials(): Promise<boolean> {
  return invoke<boolean>('has_x_credentials', {});
}

export async function testXCredentials(): Promise<string> {
  return invoke<string>('test_x_credentials', {});
}