import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import { DraftEditModal } from './DraftEditModal'
import { updateDraft, type Draft } from '../lib/db'

vi.mock('../lib/db', () => ({
  updateDraft: vi.fn(),
  parseSources: vi.fn(() => [{ title: 'Teslarati article', url: 'https://example.com/tesla' }]),
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
  generation_rationale: 'The data velocity angle is the real story.',
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
        // generation_rationale omitted / undefined is fine for update
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

  it('renders live char count with appropriate class and updates on edit', () => {
    render(<DraftEditModal draft={draft} open onClose={vi.fn()} onSaved={vi.fn()} />)

    const countEl = screen.getByTestId('draft-edit-char-count')
    expect(countEl).toHaveTextContent('18 / 280') // "Original post text".length
    expect(countEl.className).toContain('char-ok')

    fireEvent.change(screen.getByTestId('draft-edit-text'), {
      target: { value: 'x'.repeat(265) },
    })
    expect(screen.getByTestId('draft-edit-char-count')).toHaveTextContent('265 / 280')
    expect(screen.getByTestId('draft-edit-char-count').className).toContain('char-warn')
  })

  it('prompts confirm and can cancel when saving over 280 chars', async () => {
    const original = window.confirm
    const confirmSpy = vi.fn(() => false)
    ;(window as any).confirm = confirmSpy
    render(<DraftEditModal draft={draft} open onClose={vi.fn()} onSaved={vi.fn()} />)

    fireEvent.change(screen.getByTestId('draft-edit-text'), { target: { value: 'x'.repeat(290) } })
    fireEvent.click(screen.getByTestId('draft-edit-save'))

    expect(confirmSpy).toHaveBeenCalled()
    expect(mockUpdateDraft).not.toHaveBeenCalled()
    ;(window as any).confirm = original
  })
})