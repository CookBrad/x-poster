import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import PostsTab from './PostsTab'
import { getDrafts, postDraftToX, clearPendingDrafts, parseSources, type Draft } from '../lib/db'
import { invoke } from '@tauri-apps/api/core'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

vi.mock('../lib/db', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../lib/db')>()
  return {
    ...actual,
    getDrafts: vi.fn(),
    createDraft: vi.fn(),
    updateDraft: vi.fn(),
    deleteDraft: vi.fn(),
    clearPendingDrafts: vi.fn(),
    postDraftToX: vi.fn(),
  }
})

const mockGetDrafts = vi.mocked(getDrafts)
const mockPostDraftToX = vi.mocked(postDraftToX)
const mockClearPendingDrafts = vi.mocked(clearPendingDrafts)
const mockInvoke = vi.mocked(invoke)

const pendingDraft: Draft = {
  id: 'draft-1',
  text: 'Fresh take on Robotaxi expansion',
  sources_json: '[]',
  image_url: null,
  status: 'pending',
  created_at: '2026-01-01T00:00:00Z',
  updated_at: '2026-01-01T00:00:00Z',
  posted_at: null,
  x_post_id: null,
  generation_rationale: null,
}

const postedDraft: Draft = {
  ...pendingDraft,
  id: 'draft-2',
  text: 'Already posted tweet',
  status: 'posted',
  posted_at: '2026-01-02T00:00:00Z',
  x_post_id: '999',
  generation_rationale: null,
}

describe('PostsTab', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockGetDrafts.mockResolvedValue([pendingDraft, postedDraft])
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'resolve_draft_image') return pendingDraft
      return null
    })
    expect(parseSources).toBeDefined()
  })

  it('shows pending drafts in pending sub-tab', async () => {
    render(<PostsTab />)
    await waitFor(() => {
      expect(screen.getByText(/Fresh take on Robotaxi/i)).toBeInTheDocument()
    })
    expect(screen.queryByText(/Already posted tweet/i)).not.toBeInTheDocument()
    expect(screen.getByTestId('approve-draft-1')).toBeInTheDocument()
  })

  it('shows posted drafts in posted sub-tab', async () => {
    render(<PostsTab />)
    await waitFor(() => {
      expect(screen.getByTestId('posts-subtab-posted')).toBeInTheDocument()
    })
    fireEvent.click(screen.getByTestId('posts-subtab-posted'))
    await waitFor(() => {
      expect(screen.getByText(/Already posted tweet/i)).toBeInTheDocument()
    })
    expect(screen.queryByText(/Fresh take on Robotaxi/i)).not.toBeInTheDocument()
  })

  it('clears all pending posts after confirmation', async () => {
    mockClearPendingDrafts.mockResolvedValueOnce({ deleted: 1 })
    mockGetDrafts
      .mockResolvedValueOnce([pendingDraft, postedDraft])
      .mockResolvedValueOnce([postedDraft])

    render(<PostsTab />)
    await waitFor(() => {
      expect(screen.getByTestId('clear-pending-posts')).toBeInTheDocument()
    })

    fireEvent.click(screen.getByTestId('clear-pending-posts'))
    fireEvent.click(screen.getByTestId('confirm-clear-pending'))

    await waitFor(() => {
      expect(mockClearPendingDrafts).toHaveBeenCalled()
      expect(screen.queryByText(/Fresh take on Robotaxi/i)).not.toBeInTheDocument()
    })
  })

  it('moves to posted sub-tab after approve', async () => {
    mockPostDraftToX.mockResolvedValueOnce({ ...pendingDraft, status: 'posted', x_post_id: '123' })
    mockGetDrafts
      .mockResolvedValueOnce([pendingDraft, postedDraft])
      .mockResolvedValueOnce([
        { ...pendingDraft, status: 'posted', x_post_id: '123', posted_at: '2026-01-03T00:00:00Z' },
        postedDraft,
      ])

    render(<PostsTab />)
    await waitFor(() => {
      expect(screen.getByTestId('approve-draft-1')).toBeInTheDocument()
    })
    fireEvent.click(screen.getByTestId('approve-draft-1'))

    await waitFor(() => {
      expect(mockPostDraftToX).toHaveBeenCalledWith('draft-1')
      expect(screen.getByTestId('posts-subtab-posted')).toHaveClass('btn-primary')
    })
  })
})