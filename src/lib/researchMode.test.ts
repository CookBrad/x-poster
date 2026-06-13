import { describe, it, expect } from 'vitest'
import { RESEARCH_MODE } from './constants'
import { researchModeLabel, researchModeRequiresXaiKey } from './researchMode'

describe('researchMode', () => {
  it('requires xAI key for X and Both modes', () => {
    expect(researchModeRequiresXaiKey(RESEARCH_MODE.rss)).toBe(false)
    expect(researchModeRequiresXaiKey(RESEARCH_MODE.x)).toBe(true)
    expect(researchModeRequiresXaiKey(RESEARCH_MODE.both)).toBe(true)
  })

  it('labels modes for status text', () => {
    expect(researchModeLabel(RESEARCH_MODE.rss)).toBe('RSS')
    expect(researchModeLabel(RESEARCH_MODE.both)).toBe('RSS + X')
  })
})