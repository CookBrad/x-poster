import { useEffect, useState } from 'react'
import { updateDraft, parseSources, type Draft } from '../lib/db'

export interface DraftEditModalProps {
  draft: Draft | null
  open: boolean
  onClose: () => void
  onSaved: (updated: Draft) => void
}

export function DraftEditModal({ draft, open, onClose, onSaved }: DraftEditModalProps) {
  const [text, setText] = useState('')
  const [imageUrl, setImageUrl] = useState('')
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (draft && open) {
      setText(draft.text)
      setImageUrl(draft.image_url ?? '')
      setError(null)
    }
  }, [draft, open])

  if (!open || !draft) return null

  const sources = parseSources(draft)
  const previewText = text.trim() || '(empty post)'

  const handleSave = async () => {
    if (!text.trim()) {
      setError('Post text cannot be empty.')
      return
    }

    setSaving(true)
    setError(null)
    try {
      await updateDraft(draft.id, {
        text: text.trim(),
        image_url: imageUrl.trim() || null,
      })
      onSaved({
        ...draft,
        text: text.trim(),
        image_url: imageUrl.trim() || null,
        updated_at: new Date().toISOString(),
      })
      onClose()
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : 'Failed to save draft')
    } finally {
      setSaving(false)
    }
  }

  return (
    <dialog className="modal modal-open">
      <div className="modal-box max-w-2xl">
        <h3 className="font-bold text-lg mb-2">Edit draft</h3>

        <label className="form-control w-full mb-3">
          <span className="label-text text-xs opacity-70">Post text</span>
          <textarea
            className="textarea textarea-bordered h-32 font-medium"
            value={text}
            onChange={(e) => setText(e.target.value)}
            data-testid="draft-edit-text"
          />
        </label>

        <label className="form-control w-full mb-3">
          <span className="label-text text-xs opacity-70">Image URL (optional)</span>
          <input
            type="url"
            className="input input-bordered input-sm"
            placeholder="https://..."
            value={imageUrl}
            onChange={(e) => setImageUrl(e.target.value)}
            data-testid="draft-edit-image-url"
          />
        </label>

        {sources.length > 0 && (
          <div className="mb-3">
            <span className="text-xs font-medium opacity-70">Research sources used</span>
            <ul className="text-xs opacity-60 mt-1 list-disc list-inside">
              {sources.map((s: { title?: string; user?: string; source?: string }, i: number) => (
                <li key={i}>{s.title || s.user || s.source || 'Source'}</li>
              ))}
            </ul>
            <p className="text-[10px] opacity-50 mt-1">
              Generation avoided repeating themes from your recent posted drafts when possible.
            </p>
          </div>
        )}

        <div className="card bg-base-200 mb-3">
          <div className="card-body py-3">
            <span className="text-xs font-medium opacity-70 mb-1">Preview</span>
            <p className="whitespace-pre-wrap text-sm" data-testid="draft-edit-preview">
              {previewText}
            </p>
            {imageUrl.trim() && (
              <img
                src={imageUrl.trim()}
                alt="Preview"
                className="mt-2 rounded-lg max-h-40 object-cover"
                onError={(e) => {
                  (e.target as HTMLImageElement).style.display = 'none'
                }}
              />
            )}
          </div>
        </div>

        {error && (
          <div className="alert alert-error alert-sm mb-2" data-testid="draft-edit-error">
            {error}
          </div>
        )}

        <div className="modal-action">
          <button type="button" className="btn" onClick={onClose} disabled={saving}>
            Cancel
          </button>
          <button
            type="button"
            className="btn btn-primary"
            onClick={() => void handleSave()}
            disabled={saving}
            data-testid="draft-edit-save"
          >
            {saving ? 'Saving…' : 'Save'}
          </button>
        </div>
      </div>
      <form method="dialog" className="modal-backdrop">
        <button type="button" onClick={onClose}>close</button>
      </form>
    </dialog>
  )
}