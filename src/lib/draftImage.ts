import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import { parseSources, type Draft } from './db'

function isRemoteImageUrl(url: string): boolean {
  return url.startsWith('http://') || url.startsWith('https://')
}

/** Map stored draft image paths (local or remote) to a URL the UI can render. */
export function getDisplayableImageUrl(url: string | null | undefined): string | null {
  const trimmed = url?.trim()
  if (!trimmed) {
    return null
  }
  if (isRemoteImageUrl(trimmed)) {
    return trimmed
  }
  return convertFileSrc(trimmed)
}

type SourceLike = {
  source_name?: string
  source_type?: string
  media_url?: string | null
  url?: string
  title?: string
  content?: string
}

function significantTokens(text: string): string[] {
  return text
    .toLowerCase()
    .split(/\s+/)
    .map((w) => w.replace(/^[^a-z0-9$]+|[^a-z0-9$.]+$/gi, ''))
    .filter((w) => w.length >= 4)
}

function distinctiveTokens(text: string): string[] {
  return text
    .toLowerCase()
    .split(/\s+/)
    .map((w) => w.replace(/[^a-z0-9$.]/g, ''))
    .filter(
      (w) =>
        w.length >= 3 &&
        (w.includes('$') ||
          /\d/.test(w) ||
          (w.length >= 6 && !['tesla', 'spacex', 'musk', 'grok'].includes(w)))
    )
}

function sourceMatchScore(text: string, source: SourceLike): number {
  const textLower = text.toLowerCase()
  const haystack = `${source.title ?? ''} ${(source.content ?? '').slice(0, 400)}`.toLowerCase()
  let score = 0

  const author = source.source_name?.trim().replace(/^@/, '') ?? ''
  if (author) {
    if (textLower.includes(`@${author.toLowerCase()}`)) score += 3
    if (textLower.includes(`(${author.toLowerCase()})`)) score += 3
  }

  for (const token of significantTokens(textLower)) {
    if (haystack.includes(token)) score += 2
  }
  for (const token of distinctiveTokens(textLower)) {
    if (haystack.includes(token)) score += 5
  }

  return score
}

/** Pick the research source that best matches this draft (mirrors backend logic). */
export function matchPrimarySource(text: string, sources: SourceLike[]): SourceLike | null {
  if (sources.length === 0) return null
  if (sources.length === 1) return sources[0] ?? null

  let best: { source: SourceLike; score: number } | null = null
  for (const source of sources) {
    const score = sourceMatchScore(text, source)
    if (score === 0) continue
    if (!best || score > best.score) {
      best = { source, score }
    }
  }
  return best?.source ?? null
}

/** Immediate client-side preview from the draft's matched source only. */
export function getDraftDisplayImage(draft: Draft): string | null {
  const sources = parseSources(draft) as SourceLike[]
  const primary = matchPrimarySource(draft.text, sources)
  const media = primary?.media_url?.trim()
  if (media) return media

  if (draft.image_url?.trim()) {
    return getDisplayableImageUrl(draft.image_url)
  }

  return null
}

/** Fetch and persist the image for this draft's matched source (not a shared default). */
export async function resolveDraftImage(draft: Draft): Promise<Draft> {
  const sources = parseSources(draft) as SourceLike[]
  const hasCached = Boolean(draft.image_url?.trim())
  const legacyMultiSource = sources.length > 1

  if (hasCached && !legacyMultiSource && getDraftDisplayImage(draft)) {
    return draft
  }

  return invoke<Draft>('resolve_draft_image', { id: draft.id })
}