import { describe, it, expect } from 'vitest'
import { getDraftDisplayImage } from './draftImage'
import type { Draft } from './db'

const baseDraft: Draft = {
  id: '1',
  text: 'Maui sale (SawyerMerritt) signals solar adoption.',
  sources_json: JSON.stringify([
    {
      source_type: 'x_grok',
      source_name: '@SawyerMerritt',
      media_url: 'https://pbs.twimg.com/media/example.jpg',
      title: 'Maui home sale',
    },
  ]),
  image_url: null,
  status: 'pending',
  created_at: '',
  updated_at: '',
  posted_at: null,
  x_post_id: null,
}

describe('getDraftDisplayImage', () => {
  it('returns draft.image_url when set', () => {
    const draft = { ...baseDraft, image_url: 'https://example.com/a.jpg' }
    expect(getDraftDisplayImage(draft)).toBe('https://example.com/a.jpg')
  })

  it('falls back to primary source media_url', () => {
    expect(getDraftDisplayImage(baseDraft)).toBe('https://pbs.twimg.com/media/example.jpg')
  })
})