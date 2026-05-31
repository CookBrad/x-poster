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