import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import { DraftImage } from './DraftImage'
import { invoke } from '@tauri-apps/api/core'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

const mockInvoke = vi.mocked(invoke)

const draft = {
  id: 'draft-1',
  text: 'Test post',
  sources_json: '[]',
  image_url: 'https://example.com/photo.jpg',
  status: 'pending' as const,
  created_at: '',
  updated_at: '',
  posted_at: null,
  x_post_id: null,
}

describe('DraftImage', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders image immediately when draft has image_url', () => {
    render(<DraftImage draft={draft} />)
    expect(screen.getByTestId('draft-image-draft-1')).toHaveAttribute(
      'src',
      'https://example.com/photo.jpg'
    )
  })

  it('resolves image via backend when missing', async () => {
    mockInvoke.mockResolvedValueOnce({
      ...draft,
      image_url: 'https://example.com/resolved.jpg',
    })

    render(
      <DraftImage
        draft={{ ...draft, image_url: null }}
      />
    )

    await waitFor(() => {
      expect(screen.getByTestId('draft-image-draft-1')).toHaveAttribute(
        'src',
        'https://example.com/resolved.jpg'
      )
    })
    expect(mockInvoke).toHaveBeenCalledWith('resolve_draft_image', { id: 'draft-1' })
  })
})