import { useEffect, useState } from 'react'
import { updateDraft, parseSources, type Draft } from '../lib/db'
import { getDraftDisplayImage } from '../lib/draftImage'
import { DraftImage } from './DraftImage'
import { openUrl } from '@tauri-apps/plugin-opener'
import { getCharCountClass, formatCharCount, buildXPostUrl } from '../lib/draftUtils'

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
      setImageUrl(getDraftDisplayImage(draft) ?? draft.image_url ?? '')
      setError(null)
    }
  }, [draft, open])

  if (!open || !draft) return null

  const sources = parseSources(draft)
  const previewText = text.trim() || '(empty post)'
  const replyToUrl = buildXPostUrl(draft.in_reply_to_tweet_id)

  const handleSave = async () => {
    const trimmed = text.trim()
    if (!trimmed) {
      setError('Post text cannot be empty.')
      return
    }
    if (trimmed.length > 280) {
      const proceed = window.confirm(
        'This post is over 280 characters — X may truncate it or reject the post.\n\nPost anyway?'
      )
      if (!proceed) return
    }

    setSaving(true)
    setError(null)
    try {
      await updateDraft(draft.id, {
        text: trimmed,
        image_url: imageUrl.trim() || null,
      })
      onSaved({
        ...draft,
        text: trimmed,
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
        <h3 className="font-bold text-lg mb-2">
          {draft.in_reply_to_tweet_id ? 'Edit reply draft' : 'Edit draft'}
        </h3>

        {replyToUrl && (
          <div className="mb-3 space-y-1" data-testid="draft-edit-reply-target">
            <p className="text-xs opacity-70">
              Will post as a reply to{' '}
              <span
                className="link link-primary cursor-pointer"
                title={replyToUrl}
                onClick={async (e) => {
                  e.stopPropagation()
                  try {
                    await openUrl(replyToUrl)
                  } catch {
                    window.open(replyToUrl, '_blank', 'noopener,noreferrer')
                  }
                }}
              >
                parent post
              </span>
              .
            </p>
            <p className="text-[10px] opacity-60 leading-snug">
              Note: X&apos;s API often blocks replies to accounts that have not mentioned/engaged you
              first (anti-bot rule) — even when the same reply works in the X app. If Approve fails,
              open the parent and paste this text manually. Your draft stays pending.
            </p>
          </div>
        )}

        <label className="form-control w-full mb-3">
          <span className="label-text text-xs opacity-70">
            {draft.in_reply_to_tweet_id ? 'Reply text' : 'Post text'}
          </span>
          <textarea
            className="textarea textarea-bordered h-32 font-medium"
            value={text}
            onChange={(e) => setText(e.target.value)}
            data-testid="draft-edit-text"
          />
          <div className={`text-xs mt-1 ${getCharCountClass(text.length)}`} data-testid="draft-edit-char-count">
            {formatCharCount(text.length)}
          </div>
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
              {sources.map((s: { title?: string; user?: string; source?: string; url?: string }, i: number) => {
                const label = s.title || s.user || s.source || 'Source'
                if (s.url) {
                  return (
                    <li key={i}>
                      <span
                        className="link link-primary cursor-pointer"
                        title={s.url}
                        onClick={async (e) => {
                          e.stopPropagation()
                          try {
                            await openUrl(s.url!)
                          } catch (err) {
                            console.error('Failed to open URL via opener, falling back', err)
                            window.open(s.url!, '_blank')
                          }
                        }}
                      >
                        {label}
                      </span>
                    </li>
                  )
                }
                return <li key={i}>{label}</li>
              })}
            </ul>
            <p className="text-[10px] opacity-50 mt-1">
              Drafts aim for useful insight (not headline regurgitation), constructive bullish framing on
              Musk companies, and $TSLA / $SPCX when stock-relevant.
            </p>
            {draft.generation_rationale && (
              <div className="mt-2 p-2 bg-base-300/50 rounded text-[10px] opacity-70">
                <span className="font-medium">Grok&#39;s intended insight / added value:</span>{' '}
                {draft.generation_rationale}
              </div>
            )}
          </div>
        )}

        <div className="card bg-base-200 mb-3">
          <div className="card-body py-3">
            <span className="text-xs font-medium opacity-70 mb-1">Preview</span>
            <p className="whitespace-pre-wrap text-sm" data-testid="draft-edit-preview">
              {previewText}
            </p>
            {imageUrl.trim() ? (
              <img
                src={imageUrl.trim()}
                alt="Preview"
                className="mt-2 rounded-lg max-h-40 object-cover w-full"
                onError={(e) => {
                  ;(e.target as HTMLImageElement).style.display = 'none'
                }}
              />
            ) : (
              <DraftImage
                draft={draft}
                className="max-h-40"
                onResolved={(updated) => {
                  if (updated.image_url) setImageUrl(updated.image_url)
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