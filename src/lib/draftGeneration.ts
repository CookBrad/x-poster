import {
  clampDraftCount,
  DRAFT_COUNT_STORAGE_KEY,
  DRAFT_STYLE_STORAGE_KEY,
  SETTING_KEYS,
  DEFAULT_DRAFT_GENERATION_COUNT,
  DEFAULT_DRAFT_STYLE,
  MAX_DRAFT_GENERATION_COUNT,
  MIN_DRAFT_GENERATION_COUNT,
  isDraftStyle,
  type DraftStyle,
} from './constants'
import { getSetting, setSetting } from './db'

export function draftCountOptions(): number[] {
  return Array.from(
    { length: MAX_DRAFT_GENERATION_COUNT - MIN_DRAFT_GENERATION_COUNT + 1 },
    (_, index) => MIN_DRAFT_GENERATION_COUNT + index
  )
}

// Sync fallbacks (used for initial useState to avoid flicker; prefer DB).
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
    /* ignore */
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
    /* ignore */
  }
  return style
}

// DB-backed (authoritative). Settings store strings. Call these for persist + migration.
export async function loadPersistedDraftCount(): Promise<number> {
  try {
    const fromDb = await getSetting(SETTING_KEYS.draftGenerationCount)
    if (fromDb != null) {
      const n = Number.parseInt(fromDb, 10)
      if (!Number.isNaN(n)) return clampDraftCount(n)
    }
  } catch {
    /* fall through to LS */
  }
  // Fallback + one-time migrate from LS
  const ls = loadDraftGenerationCount()
  try {
    await setSetting(SETTING_KEYS.draftGenerationCount, String(ls))
    // Best-effort clear LS (non-fatal)
    localStorage.removeItem(DRAFT_COUNT_STORAGE_KEY)
  } catch {
    /* ignore */
  }
  return ls
}

export async function savePersistedDraftCount(count: number): Promise<number> {
  const clamped = clampDraftCount(count)
  // Always keep LS in sync for legacy direct reads + existing tests during transition
  try {
    localStorage.setItem(DRAFT_COUNT_STORAGE_KEY, String(clamped))
  } catch {
    /* ignore */
  }
  try {
    await setSetting(SETTING_KEYS.draftGenerationCount, String(clamped))
  } catch {
    /* DB optional during transition */
  }
  return clamped
}

export async function loadPersistedDraftStyle(): Promise<DraftStyle> {
  try {
    const fromDb = await getSetting(SETTING_KEYS.draftGenerationStyle)
    if (fromDb && isDraftStyle(fromDb)) return fromDb
  } catch {
    /* fall */
  }
  const ls = loadDraftGenerationStyle()
  try {
    await setSetting(SETTING_KEYS.draftGenerationStyle, ls)
    localStorage.removeItem(DRAFT_STYLE_STORAGE_KEY)
  } catch {
    /* ignore */
  }
  return ls
}

export async function savePersistedDraftStyle(style: DraftStyle): Promise<DraftStyle> {
  // Always keep LS in sync for legacy direct reads + existing tests during transition
  try {
    localStorage.setItem(DRAFT_STYLE_STORAGE_KEY, style)
  } catch {
    /* ignore */
  }
  try {
    await setSetting(SETTING_KEYS.draftGenerationStyle, style)
  } catch {
    /* DB optional during transition */
  }
  return style
}