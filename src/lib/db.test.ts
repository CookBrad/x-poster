import { describe, it, expect, vi, beforeEach } from 'vitest'
import {
  createDraft,
  getDrafts,
  updateDraft,
  deleteDraft,
  clearPendingDrafts,
  markDraftPosted,
  resetResearchData,
  getAllHistoricalSources,
  generateDraftsFromLatestResearch,
  postDraftToX,
  hasXCredentials,
  type Draft,
} from './db'
import { invoke } from '@tauri-apps/api/core'

// Mock the Tauri invoke function
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

const mockInvoke = vi.mocked(invoke)

describe('db.ts - Tauri command wrappers', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  const mockDraft: Draft = {
    id: 'draft-123',
    text: 'Test draft about Tesla deliveries',
    sources_json: '[{"type":"x","user":"@Tesla"}]',
    image_url: null,
    status: 'pending',
    created_at: '2025-06-02T10:00:00Z',
    updated_at: '2025-06-02T10:00:00Z',
    posted_at: null,
    x_post_id: null,
  }

  describe('createDraft', () => {
    it('calls the correct Tauri command with input', async () => {
      mockInvoke.mockResolvedValueOnce(mockDraft)

      const input = {
        text: 'New Tesla update',
        sources_json: '[]',
        image_url: null,
      }

      const result = await createDraft(input)

      expect(mockInvoke).toHaveBeenCalledWith('create_draft', { input })
      expect(result).toEqual(mockDraft)
    })

    it('propagates errors from the backend', async () => {
      mockInvoke.mockRejectedValueOnce(new Error('Database is locked'))

      await expect(createDraft({ text: 'fail', sources_json: '[]' })).rejects.toThrow('Database is locked')
    })
  })

  describe('getDrafts', () => {
    it('fetches all drafts when no status filter is provided', async () => {
      mockInvoke.mockResolvedValueOnce([mockDraft])

      const result = await getDrafts()

      expect(mockInvoke).toHaveBeenCalledWith('get_drafts', { status: undefined })
      expect(result).toHaveLength(1)
    })

    it('passes status filter when provided', async () => {
      mockInvoke.mockResolvedValueOnce([])

      await getDrafts('posted')

      expect(mockInvoke).toHaveBeenCalledWith('get_drafts', { status: 'posted' })
    })
  })

  describe('updateDraft', () => {
    it('calls update with partial data', async () => {
      mockInvoke.mockResolvedValueOnce(undefined)

      await updateDraft('draft-123', { text: 'Updated text', status: 'pending' })

      expect(mockInvoke).toHaveBeenCalledWith('update_draft', {
        id: 'draft-123',
        input: { text: 'Updated text', status: 'pending' },
      })
    })
  })

  describe('clearPendingDrafts', () => {
    it('calls clear_pending_drafts command', async () => {
      mockInvoke.mockResolvedValueOnce({ deleted: 3 })
      const result = await clearPendingDrafts()
      expect(mockInvoke).toHaveBeenCalledWith('clear_pending_drafts', {})
      expect(result).toEqual({ deleted: 3 })
    })
  })

  describe('deleteDraft & markDraftPosted', () => {
    it('calls delete command', async () => {
      mockInvoke.mockResolvedValueOnce(undefined)
      await deleteDraft('draft-xyz')
      expect(mockInvoke).toHaveBeenCalledWith('delete_draft', { id: 'draft-xyz' })
    })

    it('allows deleting a posted item (local only)', async () => {
      mockInvoke.mockResolvedValueOnce(undefined)
      await deleteDraft('posted-draft-456')
      expect(mockInvoke).toHaveBeenCalledWith('delete_draft', { id: 'posted-draft-456' })
    })

    it('calls mark as posted with x_post_id', async () => {
      mockInvoke.mockResolvedValueOnce(undefined)
      await markDraftPosted('draft-123', 'x_post_98765')
      expect(mockInvoke).toHaveBeenCalledWith('mark_draft_posted', {
        id: 'draft-123',
        xPostId: 'x_post_98765',
      })
    })
  })

  describe('resetResearchData & getAllHistoricalSources', () => {
    it('calls reset_research_data command', async () => {
      mockInvoke.mockResolvedValueOnce({ deleted_sources: 249, deleted_runs: 20 })

      const result = await resetResearchData()

      expect(mockInvoke).toHaveBeenCalledWith('reset_research_data', {})
      expect(result).toEqual({ deleted_sources: 249, deleted_runs: 20 })
    })

    it('propagates reset errors from the backend', async () => {
      mockInvoke.mockRejectedValueOnce(new Error('Failed to delete research sources'))

      await expect(resetResearchData()).rejects.toThrow('Failed to delete research sources')
    })

    it('fetches historical sources after reset would return empty', async () => {
      mockInvoke.mockResolvedValueOnce([])

      const result = await getAllHistoricalSources()

      expect(mockInvoke).toHaveBeenCalledWith('get_all_historical_sources')
      expect(result).toEqual([])
    })
  })

  describe('generateDraftsFromLatestResearch & postDraftToX', () => {
    it('calls generate_drafts_from_latest_research', async () => {
      mockInvoke.mockResolvedValueOnce([mockDraft])

      const result = await generateDraftsFromLatestResearch(3)

      expect(mockInvoke).toHaveBeenCalledWith('generate_drafts_from_latest_research', { count: 3 })
      expect(result).toHaveLength(1)
    })

    it('calls post_draft_to_x', async () => {
      const posted = { ...mockDraft, status: 'posted' as const, x_post_id: '999' }
      mockInvoke.mockResolvedValueOnce(posted)

      const result = await postDraftToX('draft-123')

      expect(mockInvoke).toHaveBeenCalledWith('post_draft_to_x', { id: 'draft-123' })
      expect(result.status).toBe('posted')
    })

    it('calls has_x_credentials', async () => {
      mockInvoke.mockResolvedValueOnce(true)
      expect(await hasXCredentials()).toBe(true)
      expect(mockInvoke).toHaveBeenCalledWith('has_x_credentials', {})
    })
  })
})