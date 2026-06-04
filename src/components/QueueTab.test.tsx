import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import QueueTab from './QueueTab'
import { getDrafts, postDraftToX, type Draft } from '../lib/db'

vi.mock('../lib/db', () => ({
  getDrafts: vi.fn(),
  createDraft: vi.fn(),
  updateDraft: vi.fn(),
  deleteDraft: vi.fn(),
  postDraftToX: vi.fn(),
}))

const mockGetDrafts = vi.mocked(getDrafts)
const mockPostDraftToX = vi.mocked(postDraftToX)

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
}

describe('QueueTab', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockGetDrafts.mockResolvedValue([pendingDraft])
  })

  it('loads and displays pending drafts', async () => {
    render(<QueueTab />)
    await waitFor(() => {
      expect(screen.getByText(/Fresh take on Robotaxi/i)).toBeInTheDocument()
    })
    expect(screen.getByTestId('draft-card-draft-1')).toBeInTheDocument()
  })

  it('shows approve button for pending drafts', async () => {
    render(<QueueTab />)
    await waitFor(() => {
      expect(screen.getByTestId('approve-draft-1')).toBeInTheDocument()
    })
  })
})