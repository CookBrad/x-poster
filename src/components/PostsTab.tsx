import { useState, useEffect, useCallback } from 'react'
import {
  getDrafts,
  createDraft,
  updateDraft,
  deleteDraft,
  clearPendingDrafts,
  postDraftToX,
  parseSources,
  type Draft,
} from '../lib/db'
import { DRAFT_STATUS } from '../lib/constants'
import { errorMessage } from '../lib/errors'
import {
  buildXPostUrl,
  countDraftsByStatus,
  formatDraftTimestamp,
  formatSourceLabel,
  isPendingDraft,
  isPostedDraft,
} from '../lib/draftUtils'
import { DraftEditModal } from './DraftEditModal'
import { DraftImage } from './DraftImage'
import { openUrl } from '@tauri-apps/plugin-opener'

type PostsSubview = 'pending' | 'posted'

export default function PostsTab() {
  const [subTab, setSubTab] = useState<PostsSubview>('pending')
  const [drafts, setDrafts] = useState<Draft[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [editingDraft, setEditingDraft] = useState<Draft | null>(null)
  const [postingId, setPostingId] = useState<string | null>(null)
  const [showClearConfirm, setShowClearConfirm] = useState(false)
  const [clearing, setClearing] = useState(false)

  const reportError = useCallback((message: string) => {
    setError(message)
  }, [])

  const loadDrafts = useCallback(async () => {
    try {
      setLoading(true)
      const data = await getDrafts()
      setDrafts(data)
      setError(null)
    } catch (loadError: unknown) {
      console.error(loadError)
      reportError('Failed to load posts from database')
    } finally {
      setLoading(false)
    }
  }, [reportError])

  useEffect(() => {
    void loadDrafts()
  }, [loadDrafts])

  const { pending: pendingCount, posted: postedCount } = countDraftsByStatus(drafts)

  const visibleDrafts = drafts.filter((draft) =>
    subTab === 'pending' ? isPendingDraft(draft) : isPostedDraft(draft)
  )

  const updateDraftInList = (updated: Draft) => {
    setDrafts((previous) =>
      previous.map((draft) => (draft.id === updated.id ? updated : draft))
    )
  }

  const handleCreateTestDraft = async () => {
    try {
      await createDraft({
        text: 'Tesla delivered a record number of vehicles this quarter, with strong growth in energy storage and FSD adoption continuing to accelerate.',
        sources_json: JSON.stringify([
          { type: 'x_post', user: '@Tesla', text: 'Q2 delivery numbers are in...' },
          { type: 'rss', source: 'Teslarati', title: 'Tesla Q2 deliveries beat expectations' },
        ]),
        image_url: null,
        generation_rationale: null,
      })
      setSubTab('pending')
      await loadDrafts()
    } catch (createError: unknown) {
      reportError(`Failed to create test draft: ${errorMessage(createError)}`)
    }
  }

  const handleSkip = async (id: string) => {
    try {
      await updateDraft(id, { status: DRAFT_STATUS.skipped })
      await loadDrafts()
    } catch (skipError: unknown) {
      reportError(`Failed to skip draft: ${errorMessage(skipError)}`)
    }
  }

  const handleDelete = async (draft: Draft) => {
    if (
      isPostedDraft(draft) &&
      !window.confirm(
        'Delete this posted item from your local history?\n\n(This will NOT delete the tweet on X)'
      )
    ) {
      return
    }

    setDrafts((previous) => previous.filter((item) => item.id !== draft.id))
    try {
      await deleteDraft(draft.id)
      await loadDrafts()
    } catch (deleteError: unknown) {
      reportError(`Failed to delete: ${errorMessage(deleteError)}`)
      await loadDrafts()
    }
  }

  const handleClearPending = async () => {
    setShowClearConfirm(false)
    setClearing(true)
    setError(null)
    try {
      const result = await clearPendingDrafts()
      await loadDrafts()
      if (result.deleted === 0) {
        reportError('No pending posts to clear.')
      }
    } catch (clearError: unknown) {
      reportError(errorMessage(clearError, 'Failed to clear pending posts'))
      await loadDrafts()
    } finally {
      setClearing(false)
    }
  }

  const handleApprovePost = async (draft: Draft) => {
    setPostingId(draft.id)
    setError(null)
    try {
      await postDraftToX(draft.id)
      await loadDrafts()
      setSubTab('posted')
    } catch (postError: unknown) {
      reportError(`Failed to post to X: ${errorMessage(postError)}`)
    } finally {
      setPostingId(null)
    }
  }

  if (loading) {
    return (
      <div className="flex justify-center py-12">
        <span className="loading loading-spinner loading-lg" />
      </div>
    )
  }

  return (
    <div>
      <div className="flex items-center justify-between mb-4 flex-wrap gap-2">
        <h2 className="text-2xl font-semibold">Posts</h2>
        <div className="flex gap-2">
          <button
            type="button"
            className="btn btn-primary btn-sm"
            onClick={() => void handleCreateTestDraft()}
          >
            + Create Test Draft
          </button>
          <button type="button" className="btn btn-outline btn-sm" onClick={() => void loadDrafts()}>
            Refresh
          </button>
        </div>
      </div>

      <div className="flex items-center justify-between mb-4 flex-wrap gap-3">
        <div className="flex gap-4" data-testid="posts-subtabs">
          <button
            type="button"
            className={`btn btn-sm min-w-[7rem] ${subTab === 'pending' ? 'btn-primary' : 'btn-outline'}`}
            onClick={() => setSubTab('pending')}
            data-testid="posts-subtab-pending"
          >
            Pending {pendingCount > 0 ? `(${pendingCount})` : ''}
          </button>
          <button
            type="button"
            className={`btn btn-sm min-w-[7rem] ${subTab === 'posted' ? 'btn-primary' : 'btn-outline'}`}
            onClick={() => setSubTab('posted')}
            data-testid="posts-subtab-posted"
          >
            Posted {postedCount > 0 ? `(${postedCount})` : ''}
          </button>
        </div>

        {subTab === 'pending' && (
          <button
            type="button"
            className="btn btn-error btn-outline btn-sm"
            onClick={() => setShowClearConfirm(true)}
            disabled={pendingCount === 0 || clearing}
            data-testid="clear-pending-posts"
          >
            {clearing ? 'Clearing…' : 'Clear Pending'}
          </button>
        )}
      </div>

      {showClearConfirm && (
        <dialog className="modal modal-open">
          <div className="modal-box">
            <h3 className="font-bold text-lg">Clear all pending posts?</h3>
            <p className="py-3 text-sm opacity-80">
              This permanently deletes {pendingCount} pending post
              {pendingCount === 1 ? '' : 's'} from your local queue. Posted items are not affected.
              This cannot be undone.
            </p>
            <div className="modal-action">
              <button
                type="button"
                className="btn"
                onClick={() => setShowClearConfirm(false)}
                disabled={clearing}
              >
                Cancel
              </button>
              <button
                type="button"
                className="btn btn-error"
                onClick={() => void handleClearPending()}
                disabled={clearing}
                data-testid="confirm-clear-pending"
              >
                {clearing ? 'Clearing…' : 'Clear Pending'}
              </button>
            </div>
          </div>
          <form method="dialog" className="modal-backdrop">
            <button type="button" onClick={() => setShowClearConfirm(false)}>
              close
            </button>
          </form>
        </dialog>
      )}

      {error && (
        <div className="alert alert-error mb-4">
          <span>{error}</span>
        </div>
      )}

      {visibleDrafts.length === 0 ? (
        <div className="alert alert-info" data-testid="posts-empty">
          <span>
            {subTab === 'pending'
              ? 'No pending posts. Run research and generate drafts, or create a test draft.'
              : 'No posted items yet. Approve a pending post to publish it to X.'}
          </span>
        </div>
      ) : (
        <div className="grid gap-4 md:grid-cols-2">
          {visibleDrafts.map((draft) => (
            <DraftCard
              key={draft.id}
              draft={draft}
              posting={postingId === draft.id}
              onEdit={() => setEditingDraft(draft)}
              onApprove={() => void handleApprovePost(draft)}
              onSkip={() => void handleSkip(draft.id)}
              onDelete={() => void handleDelete(draft)}
              onImageResolved={updateDraftInList}
            />
          ))}
        </div>
      )}

      <DraftEditModal
        draft={editingDraft}
        open={!!editingDraft}
        onClose={() => setEditingDraft(null)}
        onSaved={(updated) => {
          updateDraftInList(updated)
          setEditingDraft(null)
        }}
      />
    </div>
  )
}

interface DraftCardProps {
  draft: Draft
  posting: boolean
  onEdit: () => void
  onApprove: () => void
  onSkip: () => void
  onDelete: () => void
  onImageResolved?: (updated: Draft) => void
}

function DraftCard({
  draft,
  posting,
  onEdit,
  onApprove,
  onSkip,
  onDelete,
  onImageResolved,
}: DraftCardProps) {
  const xUrl = buildXPostUrl(draft.x_post_id)

  return (
    <div className="card bg-base-100 shadow draft-card" data-testid={`draft-card-${draft.id}`}>
      <div className="card-body">
        <div className="flex justify-between text-xs opacity-70 mb-1">
          <span className="badge badge-sm">{draft.status}</span>
          <span>{formatDraftTimestamp(draft)}</span>
        </div>

        <p className="font-medium whitespace-pre-wrap">{draft.text}</p>

        <DraftImage draft={draft} onResolved={onImageResolved} />

        {(() => {
          const srcs = parseSources(draft)
          if (srcs.length === 0) return null
          return (
            <div className="text-xs opacity-60 mt-2">
              Sources:{' '}
              {srcs.map((s: any, i: number) => {
                const label = formatSourceLabel(s) || 'Source'
                const hasUrl = !!s.url
                if (hasUrl) {
                  return (
                    <span
                      key={i}
                      className="link link-primary cursor-pointer"
                      title={s.url}
                      onClick={async (e) => {
                        e.stopPropagation()
                        try {
                          await openUrl(s.url)
                        } catch (err) {
                          console.error('Failed to open URL via opener, falling back', err)
                          window.open(s.url, '_blank')
                        }
                      }}
                    >
                      {label}
                      {i < srcs.length - 1 ? ', ' : ''}
                    </span>
                  )
                }
                return (
                  <span key={i}>
                    {label}
                    {i < srcs.length - 1 ? ', ' : ''}
                  </span>
                )
              })}
            </div>
          )
        })()}

        {draft.x_post_id && (
          <div className="text-xs text-success mt-1">
            {xUrl ? (
              <span
                className="link link-primary cursor-pointer"
                onClick={async (e) => {
                  e.stopPropagation()
                  try {
                    await openUrl(xUrl)
                  } catch (err) {
                    console.error('Failed to open URL via opener, falling back', err)
                    window.open(xUrl, '_blank')
                  }
                }}
              >
                View on X →
              </span>
            ) : (
              <>Posted as: {draft.x_post_id}</>
            )}
          </div>
        )}

        <div className="card-actions justify-end mt-4 gap-2">
          {isPendingDraft(draft) && (
            <button type="button" className="btn btn-ghost btn-sm" onClick={onEdit}>
              Edit
            </button>
          )}

          {isPendingDraft(draft) && (
            <>
              <button
                type="button"
                className="btn btn-success btn-sm"
                onClick={onApprove}
                disabled={posting}
                data-testid={`approve-${draft.id}`}
              >
                {posting ? 'Posting…' : 'Approve & Post'}
              </button>
              <button type="button" className="btn btn-ghost btn-sm" onClick={onSkip}>
                Skip
              </button>
            </>
          )}

          <button type="button" className="btn btn-error btn-sm btn-outline" onClick={onDelete}>
            {isPostedDraft(draft) ? 'Delete Post' : 'Delete Draft'}
          </button>
        </div>
      </div>
    </div>
  )
}