import { describe, it, expect } from 'vitest'
import { countUnusedResearchSources, isResearchSourceUsed } from './researchSource'
import type { ResearchSource } from './db'

const source = (overrides: Partial<ResearchSource> = {}): ResearchSource => ({
  id: '1',
  title: 'Story',
  content: 'Details',
  url: 'https://example.com',
  published_at: null,
  source_name: 'Teslarati',
  source_type: 'rss',
  ...overrides,
})

describe('researchSource', () => {
  it('detects used sources', () => {
    expect(isResearchSourceUsed(source())).toBe(false)
    expect(isResearchSourceUsed(source({ used_at: '2026-06-13T00:00:00Z' }))).toBe(true)
  })

  it('counts unused sources', () => {
    const count = countUnusedResearchSources([
      source(),
      source({ id: '2', used_at: '2026-06-13T00:00:00Z' }),
    ])
    expect(count).toBe(1)
  })
})