import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import { DraftEditModal } from './DraftEditModal'
import { updateDraft, type Draft } from '../lib/db'

vi.mock('../lib/db', () => ({
  updateDraft: vi.fn(),
  parseSources: vi.fn(() => [{ title: 'Teslarati article' }]),
}))

const mockUpdateDraft = vi.mocked(updateDraft)

const draft: Draft = {
  id: 'd-1',
  text: 'Original post text',
  sources_json: '[{"title":"Teslarati article"}]',
  image_url: null,
  status: 'pending',
  created_at: '2026-01-01T00:00:00Z',
  updated_at: '2026-01-01T00:00:00Z',
  posted_at: null,
  x_post_id: null,
}

describe('DraftEditModal', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockUpdateDraft.mockResolvedValue(undefined)
  })

  it('renders nothing when closed', () => {
    const { container } = render(
      <DraftEditModal draft={draft} open={false} onClose={vi.fn()} onSaved={vi.fn()} />
    )
    expect(container.firstChild).toBeNull()
  })

  it('shows preview and saves updated text', async () => {
    const onSaved = vi.fn()
    const onClose = vi.fn()

    render(
      <DraftEditModal draft={draft} open onClose={onClose} onSaved={onSaved} />
    )

    const textarea = screen.getByTestId('draft-edit-text')
    fireEvent.change(textarea, { target: { value: 'Updated fresh take on FSD.' } })

    expect(screen.getByTestId('draft-edit-preview')).toHaveTextContent(
      'Updated fresh take on FSD.'
    )

    fireEvent.click(screen.getByTestId('draft-edit-save'))

    await waitFor(() => {
      expect(mockUpdateDraft).toHaveBeenCalledWith('d-1', {
        text: 'Updated fresh take on FSD.',
        image_url: null,
      })
      expect(onSaved).toHaveBeenCalled()
      expect(onClose).toHaveBeenCalled()
    })
  })

  it('shows error when saving empty text', async () => {
    render(<DraftEditModal draft={draft} open onClose={vi.fn()} onSaved={vi.fn()} />)

    fireEvent.change(screen.getByTestId('draft-edit-text'), { target: { value: '   ' } })
    fireEvent.click(screen.getByTestId('draft-edit-save'))

    expect(await screen.findByTestId('draft-edit-error')).toHaveTextContent(/cannot be empty/i)
    expect(mockUpdateDraft).not.toHaveBeenCalled()
  })
})