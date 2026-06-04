import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import HistoryTab from './HistoryTab'
import { getDrafts, type Draft } from '../lib/db'

vi.mock('../lib/db', () => ({
  getDrafts: vi.fn(),
  parseSources: vi.fn(() => []),
}))

const mockGetDrafts = vi.mocked(getDrafts)

describe('HistoryTab', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('shows empty state when no posted drafts', async () => {
    mockGetDrafts.mockResolvedValue([])
    render(<HistoryTab />)
    await waitFor(() => {
      expect(screen.getByText(/No posted drafts yet/i)).toBeInTheDocument()
    })
  })

  it('lists posted drafts with X link', async () => {
    const posted: Draft = {
      id: 'p1',
      text: 'Posted tweet text',
      sources_json: '[]',
      image_url: null,
      status: 'posted',
      created_at: '2026-01-01T00:00:00Z',
      updated_at: '2026-01-01T00:00:00Z',
      posted_at: '2026-01-02T00:00:00Z',
      x_post_id: '1234567890',
    }
    mockGetDrafts.mockResolvedValue([posted])
    render(<HistoryTab />)
    await waitFor(() => {
      expect(screen.getByText(/Posted tweet text/i)).toBeInTheDocument()
      expect(screen.getByText(/View on X/i)).toBeInTheDocument()
    })
  })
})