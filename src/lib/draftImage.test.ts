import { describe, it, expect } from 'vitest'
import { getDraftDisplayImage, matchPrimarySource } from './draftImage'
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
  generation_rationale: null,
}

describe('getDraftDisplayImage', () => {
  it('returns draft.image_url when set for single-source draft', () => {
    const draft = {
      ...baseDraft,
      sources_json: '[]',
      image_url: 'https://example.com/a.jpg',
    }
    expect(getDraftDisplayImage(draft)).toBe('https://example.com/a.jpg')
  })

  it('falls back to primary source media_url', () => {
    expect(getDraftDisplayImage(baseDraft)).toBe('https://pbs.twimg.com/media/example.jpg')
  })

  it('prefers resolved image_url over source media_url', () => {
    const draft = {
      ...baseDraft,
      image_url: 'https://example.com/resolved.jpg',
    }
    expect(getDraftDisplayImage(draft)).toBe('https://example.com/resolved.jpg')
  })

  it('picks the matching source when multiple from same author', () => {
    const sources = [
      {
        source_type: 'x_grok',
        source_name: '@SawyerMerritt',
        title: 'Cybercabs in Houston',
        content: 'Tons of Tesla Cybercabs in Houston',
        media_url: 'https://pbs.twimg.com/media/cybercab.jpg',
      },
      {
        source_type: 'x_grok',
        source_name: '@SawyerMerritt',
        title: 'Maui $26.5M home',
        content: 'Maui home sold for $26.5 million with Solar Tile Roof',
        media_url: 'https://pbs.twimg.com/media/maui.jpg',
      },
    ]
    const draft: Draft = {
      ...baseDraft,
      sources_json: JSON.stringify(sources),
      image_url: null,
      text: "Maui's $26.5M sale (@SawyerMerritt) validates solar tiles.",
    }
    expect(matchPrimarySource(draft.text, sources)?.media_url).toBe(
      'https://pbs.twimg.com/media/maui.jpg'
    )
    expect(getDraftDisplayImage(draft)).toBe('https://pbs.twimg.com/media/maui.jpg')
  })
})