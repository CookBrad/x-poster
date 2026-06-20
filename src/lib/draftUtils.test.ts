import { describe, it, expect } from 'vitest'
import {
  buildXPostUrl,
  countDraftsByStatus,
  formatSourceLabels,
  isPendingDraft,
} from './draftUtils'
import { DRAFT_STATUS } from './constants'
import type { Draft } from './db'

const baseDraft: Draft = {
  id: 'draft-1',
  text: 'Hello',
  sources_json: JSON.stringify([
    { source_type: 'x_grok', source_name: '@Tesla', title: 'Ignored when user present' },
    { source_type: 'rss', source_name: 'Not A Tesla App', title: 'Fallback title' },
  ]),
  image_url: null,
  status: DRAFT_STATUS.pending,
  created_at: '2026-01-01T00:00:00Z',
  updated_at: '2026-01-01T00:00:00Z',
  posted_at: null,
  x_post_id: null,
  generation_rationale: null,
}

describe('draftUtils', () => {
  it('counts pending and posted drafts', () => {
    const counts = countDraftsByStatus([
      baseDraft,
      { ...baseDraft, id: 'draft-2', status: DRAFT_STATUS.posted },
      { ...baseDraft, id: 'draft-3', status: DRAFT_STATUS.skipped },
    ])

    expect(counts).toEqual({ pending: 1, posted: 1 })
  })

  it('formats source labels from draft sources', () => {
    expect(formatSourceLabels(baseDraft)).toBe('@Tesla, source: Not A Tesla App')
  })

  it('builds X URLs only for real tweet ids', () => {
    expect(buildXPostUrl('12345')).toBe('https://x.com/i/web/status/12345')
    expect(buildXPostUrl('sim_12345')).toBeNull()
    expect(buildXPostUrl(null)).toBeNull()
  })

  it('identifies pending drafts', () => {
    expect(isPendingDraft(baseDraft)).toBe(true)
    expect(isPendingDraft({ ...baseDraft, status: DRAFT_STATUS.posted })).toBe(false)
  })
})