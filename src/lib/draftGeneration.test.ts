import { describe, it, expect, beforeEach } from 'vitest'
import { DRAFT_COUNT_STORAGE_KEY } from './constants'
import { loadDraftGenerationCount, saveDraftGenerationCount } from './draftGeneration'

describe('draftGeneration', () => {
  beforeEach(() => {
    localStorage.clear()
  })

  it('returns default when nothing is stored', () => {
    expect(loadDraftGenerationCount()).toBe(3)
  })

  it('persists and reloads a clamped count', () => {
    expect(saveDraftGenerationCount(7)).toBe(7)
    expect(loadDraftGenerationCount()).toBe(7)
    expect(localStorage.getItem(DRAFT_COUNT_STORAGE_KEY)).toBe('7')
  })

  it('clamps out-of-range values', () => {
    expect(saveDraftGenerationCount(99)).toBe(10)
    expect(saveDraftGenerationCount(0)).toBe(1)
  })
})