import { useState, useEffect, useCallback } from 'react'
import {
  getDrafts,
  createDraft,
  updateDraft,
  deleteDraft,
  postDraftToX,
  type Draft,
} from '../lib/db'
import { DraftEditModal } from './DraftEditModal'

export default function QueueTab() {
  const [drafts, setDrafts] = useState<Draft[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [editingDraft, setEditingDraft] = useState<Draft | null>(null)
  const [postingId, setPostingId] = useState<string | null>(null)

  const loadDrafts = useCallback(async () => {
    try {
      setLoading(true)
      const data = await getDrafts()
      setDrafts(data)
      setError(null)
    } catch (e: unknown) {
      console.error(e)
      setError('Failed to load drafts from database')
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    void loadDrafts()
  }, [loadDrafts])

  const handleCreateTestDraft = async () => {
    try {
      await createDraft({
        text: 'Tesla delivered a record number of vehicles this quarter, with strong growth in energy storage and FSD adoption continuing to accelerate.',
        sources_json: JSON.stringify([
          { type: 'x_post', user: '@Tesla', text: 'Q2 delivery numbers are in...' },
          { type: 'rss', source: 'Teslarati', title: 'Tesla Q2 deliveries beat expectations' },
        ]),
        image_url: null,
      })
      await loadDrafts()
    } catch (e: unknown) {
      alert('Failed to create test draft: ' + (e instanceof Error ? e.message : e))
    }
  }

  const handleSkip = async (id: string) => {
    try {
      await updateDraft(id, { status: 'skipped' })
      await loadDrafts()
    } catch (e: unknown) {
      alert('Failed to skip draft: ' + (e instanceof Error ? e.message : e))
    }
  }

  const handleDelete = async (draft: Draft) => {
    const isPosted = draft.status === 'posted'
    if (isPosted && !window.confirm(
      'Delete this posted item from your local history?\n\n(This will NOT delete the tweet on X)'
    )) {
      return
    }

    setDrafts((prev) => prev.filter((d) => d.id !== draft.id))
    try {
      await deleteDraft(draft.id)
      await loadDrafts()
    } catch (e: unknown) {
      alert(`Failed to delete: ${e instanceof Error ? e.message : e}`)
      await loadDrafts()
    }
  }

  const handleApprovePost = async (draft: Draft) => {
    setPostingId(draft.id)
    setError(null)
    try {
      await postDraftToX(draft.id)
      await loadDrafts()
    } catch (e: unknown) {
      setError('Failed to post to X: ' + (e instanceof Error ? e.message : e))
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
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-2xl font-semibold">Draft Queue</h2>
        <div className="flex gap-2">
          <button type="button" className="btn btn-primary btn-sm" onClick={() => void handleCreateTestDraft()}>
            + Create Test Draft
          </button>
          <button type="button" className="btn btn-outline btn-sm" onClick={() => void loadDrafts()}>
            Refresh
          </button>
        </div>
      </div>

      {error && (
        <div className="alert alert-error mb-4">
          <span>{error}</span>
        </div>
      )}

      {drafts.length === 0 ? (
        <div className="alert alert-info">
          <span>No drafts yet. Run research and generate drafts, or create a test draft.</span>
        </div>
      ) : (
        <div className="grid gap-4 md:grid-cols-2">
          {drafts.map((draft) => (
            <DraftCard
              key={draft.id}
              draft={draft}
              posting={postingId === draft.id}
              onEdit={() => setEditingDraft(draft)}
              onApprove={() => void handleApprovePost(draft)}
              onSkip={() => void handleSkip(draft.id)}
              onDelete={() => void handleDelete(draft)}
            />
          ))}
        </div>
      )}

      <DraftEditModal
        draft={editingDraft}
        open={!!editingDraft}
        onClose={() => setEditingDraft(null)}
        onSaved={(updated) => {
          setDrafts((prev) => prev.map((d) => (d.id === updated.id ? updated : d)))
          setEditingDraft(null)
        }}
      />
    </div>
  )
}

function DraftCard({
  draft,
  posting,
  onEdit,
  onApprove,
  onSkip,
  onDelete,
}: {
  draft: Draft
  posting: boolean
  onEdit: () => void
  onApprove: () => void
  onSkip: () => void
  onDelete: () => void
}) {
  let sources: { user?: string; source?: string; title?: string }[] = []
  try {
    sources = JSON.parse(draft.sources_json || '[]')
  } catch {
    sources = []
  }

  const xUrl = draft.x_post_id && !draft.x_post_id.startsWith('sim_')
    ? `https://x.com/i/web/status/${draft.x_post_id}`
    : null

  return (
    <div className="card bg-base-100 shadow draft-card" data-testid={`draft-card-${draft.id}`}>
      <div className="card-body">
        <div className="flex justify-between text-xs opacity-70 mb-1">
          <span className="badge badge-sm">{draft.status}</span>
          <span>{new Date(draft.created_at).toLocaleString()}</span>
        </div>

        <p className="font-medium whitespace-pre-wrap">{draft.text}</p>

        {draft.image_url && (
          <img
            src={draft.image_url}
            alt="Post attachment from source"
            className="mt-2 rounded-lg max-h-48 w-full object-cover"
            data-testid={`draft-image-${draft.id}`}
            onError={(e) => {
              (e.target as HTMLImageElement).style.display = 'none'
            }}
          />
        )}

        {sources.length > 0 && (
          <div className="text-xs opacity-60 mt-2">
            Sources:{' '}
            {sources
              .map((s: { user?: string; source?: string; source_name?: string; title?: string }) =>
                s.source_name || s.user || s.source || s.title
              )
              .join(', ')}
          </div>
        )}

        {draft.x_post_id && (
          <div className="text-xs text-success mt-1">
            {xUrl ? (
              <a href={xUrl} target="_blank" rel="noopener noreferrer" className="link">
                Posted: {draft.x_post_id}
              </a>
            ) : (
              <>Posted as: {draft.x_post_id}</>
            )}
          </div>
        )}

        <div className="card-actions justify-end mt-4 gap-2">
          <button type="button" className="btn btn-ghost btn-sm" onClick={onEdit}>
            Edit
          </button>

          {draft.status === 'pending' && (
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
            {draft.status === 'posted' ? 'Delete Post' : 'Delete Draft'}
          </button>
        </div>
      </div>
    </div>
  )
}