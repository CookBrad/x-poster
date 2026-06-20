import {
  clampDraftCount,
  DRAFT_COUNT_STORAGE_KEY,
  DRAFT_STYLE_STORAGE_KEY,
  DEFAULT_DRAFT_GENERATION_COUNT,
  DEFAULT_DRAFT_STYLE,
  MAX_DRAFT_GENERATION_COUNT,
  MIN_DRAFT_GENERATION_COUNT,
  isDraftStyle,
  type DraftStyle,
} from './constants'

export function draftCountOptions(): number[] {
  return Array.from(
    { length: MAX_DRAFT_GENERATION_COUNT - MIN_DRAFT_GENERATION_COUNT + 1 },
    (_, index) => MIN_DRAFT_GENERATION_COUNT + index
  )
}

export function loadDraftGenerationCount(): number {
  try {
    const stored = localStorage.getItem(DRAFT_COUNT_STORAGE_KEY)
    if (!stored) {
      return DEFAULT_DRAFT_GENERATION_COUNT
    }
    const parsed = Number.parseInt(stored, 10)
    if (Number.isNaN(parsed)) {
      return DEFAULT_DRAFT_GENERATION_COUNT
    }
    return clampDraftCount(parsed)
  } catch {
    return DEFAULT_DRAFT_GENERATION_COUNT
  }
}

export function saveDraftGenerationCount(count: number): number {
  const clamped = clampDraftCount(count)
  try {
    localStorage.setItem(DRAFT_COUNT_STORAGE_KEY, String(clamped))
  } catch {
    /* ignore storage failures */
  }
  return clamped
}

export function loadDraftGenerationStyle(): DraftStyle {
  try {
    const stored = localStorage.getItem(DRAFT_STYLE_STORAGE_KEY)
    if (stored && isDraftStyle(stored)) {
      return stored
    }
    return DEFAULT_DRAFT_STYLE
  } catch {
    return DEFAULT_DRAFT_STYLE
  }
}

export function saveDraftGenerationStyle(style: DraftStyle): DraftStyle {
  try {
    localStorage.setItem(DRAFT_STYLE_STORAGE_KEY, style)
  } catch {
    /* ignore storage failures */
  }
  return style
}