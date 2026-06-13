import { invoke } from '@tauri-apps/api/core'
import { parseSources, type Draft } from './db'

type SourceLike = {
  source_name?: string
  source_type?: string
  media_url?: string | null
  url?: string
  title?: string
}

/** Immediate client-side preview from stored source metadata (no network). */
export function getDraftDisplayImage(draft: Draft): string | null {
  if (draft.image_url?.trim()) {
    return draft.image_url.trim()
  }

  const sources = parseSources(draft) as SourceLike[]
  const primary = primaryXSource(draft.text, sources)
  const media = primary?.media_url?.trim()
  return media || null
}

function primaryXSource(text: string, sources: SourceLike[]): SourceLike | null {
  const xSources = sources.filter(
    (s) => s.source_type === 'x_grok' || s.source_type === 'x'
  )
  if (xSources.length === 0) return null

  const textLower = text.toLowerCase()
  for (const source of xSources) {
    const author = source.source_name?.trim().replace(/^@/, '') ?? ''
    if (!author) continue
    const authorLower = author.toLowerCase()
    if (
      textLower.includes(`@${authorLower}`) ||
      textLower.includes(`(${authorLower})`) ||
      textLower.includes(authorLower)
    ) {
      return source
    }
  }

  return xSources[0] ?? null
}

/** Fetch and persist image URL from X/source when missing (uses X credentials if set). */
export async function resolveDraftImage(draft: Draft): Promise<Draft> {
  if (getDraftDisplayImage(draft)) {
    return draft
  }
  return invoke<Draft>('resolve_draft_image', { id: draft.id })
}