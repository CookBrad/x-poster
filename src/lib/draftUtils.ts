import { DRAFT_STATUS, RESEARCH_SOURCE_TYPE, SIMULATED_POST_ID_PREFIX, X_STATUS_URL_BASE } from './constants'
import { parseSources, type Draft, type DraftSource } from './db'

export function isRssSource(source: DraftSource): boolean {
  const sourceType = (source.source_type ?? source.type ?? '').toLowerCase()
  return sourceType === RESEARCH_SOURCE_TYPE.rss
}

export function isXSource(source: DraftSource): boolean {
  const sourceType = (source.source_type ?? source.type ?? '').toLowerCase()
  return sourceType === RESEARCH_SOURCE_TYPE.xGrok || sourceType === 'x' || sourceType === 'x_post'
}

export interface DraftStatusCounts {
  pending: number
  posted: number
}

export function countDraftsByStatus(drafts: Draft[]): DraftStatusCounts {
  return drafts.reduce(
    (counts, draft) => {
      if (draft.status === DRAFT_STATUS.pending) {
        counts.pending += 1
      } else if (draft.status === DRAFT_STATUS.posted) {
        counts.posted += 1
      }
      return counts
    },
    { pending: 0, posted: 0 }
  )
}

export function formatSourceLabel(source: DraftSource): string | undefined {
  const rawName = source.source_name || source.user || source.source || source.title
  if (!rawName) return undefined

  const name = rawName.trim().replace(/^@/, '')
  if (isRssSource(source)) {
    return `source: ${name}`
  }

  if (isXSource(source) || source.user) {
    return `@${name}`
  }

  return name
}

export function formatSourceLabels(draft: Draft): string {
  return parseSources(draft).map(formatSourceLabel).filter(Boolean).join(', ')
}

export function buildXPostUrl(xPostId: string | null | undefined): string | null {
  if (!xPostId || xPostId.startsWith(SIMULATED_POST_ID_PREFIX)) {
    return null
  }
  return `${X_STATUS_URL_BASE}${xPostId}`
}

/** Whether this draft is intended to post as a reply to another tweet. */
export function isReplyDraft(draft: Draft): boolean {
  return Boolean(draft.in_reply_to_tweet_id?.trim())
}

export function formatDraftTimestamp(draft: Draft): string {
  const timestamp =
    draft.status === DRAFT_STATUS.posted && draft.posted_at
      ? draft.posted_at
      : draft.created_at
  return new Date(timestamp).toLocaleString()
}

export function isPendingDraft(draft: Draft): boolean {
  return draft.status === DRAFT_STATUS.pending
}

export function isPostedDraft(draft: Draft): boolean {
  return draft.status === DRAFT_STATUS.posted
}

/** Returns the appropriate .char-* class for a post length vs X's 280 limit. */
export function getCharCountClass(length: number): string {
  if (length > 280) return 'char-danger'
  if (length > 260) return 'char-warn'
  return 'char-ok'
}

export function formatCharCount(length: number, limit = 280): string {
  return `${length} / ${limit}`
}