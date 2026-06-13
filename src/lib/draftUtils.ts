import { DRAFT_STATUS, SIMULATED_POST_ID_PREFIX, X_STATUS_URL_BASE } from './constants'
import { parseSources, type Draft, type DraftSource } from './db'

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
  return source.source_name || source.user || source.source || source.title
}

export function formatSourceLabels(draft: Draft): string {
  return parseSources(draft).map(formatSourceLabel).filter(Boolean).join(', ')
}

export function buildXPostUrl(xPostId: string | null): string | null {
  if (!xPostId || xPostId.startsWith(SIMULATED_POST_ID_PREFIX)) {
    return null
  }
  return `${X_STATUS_URL_BASE}${xPostId}`
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