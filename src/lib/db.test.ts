import { describe, it, expect, vi, beforeEach } from 'vitest'
import { createDraft, getDrafts, updateDraft, deleteDraft, markDraftPosted, type Draft } from './db'
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
})