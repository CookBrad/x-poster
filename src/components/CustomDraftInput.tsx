import type { FormEvent } from 'react'
import { DRAFT_STYLE, type DraftStyle } from '../lib/constants'

export interface CustomDraftInputProps {
  value: string
  onChange: (value: string) => void
  onGeneratePost: () => void
  onGenerateReply: () => void
  generatingPost?: boolean
  generatingReply?: boolean
  disabled?: boolean
  hasXaiKey?: boolean
  draftStyle?: DraftStyle
}

export function CustomDraftInput({
  value,
  onChange,
  onGeneratePost,
  onGenerateReply,
  generatingPost = false,
  generatingReply = false,
  disabled = false,
  hasXaiKey = false,
  draftStyle = DRAFT_STYLE.insight,
}: CustomDraftInputProps) {
  const trimmed = value.trim()
  const busy = generatingPost || generatingReply
  const baseDisabled = disabled || busy || !hasXaiKey || trimmed.length === 0

  const emptyHint = !hasXaiKey
    ? 'xAI key required'
    : trimmed.length === 0
      ? 'Enter a link, X post URL, or topic'
      : undefined

  const handleSubmit = (event: FormEvent) => {
    event.preventDefault()
    if (!baseDisabled) {
      onGeneratePost()
    }
  }

  return (
    <div className="card bg-base-200/60 mb-4">
      <div className="card-body py-4 gap-3">
        <div>
          <h3 className="font-semibold text-base">Generate from a link or topic</h3>
          <p className="text-xs opacity-70 mt-1">
            Paste an article URL, X post URL, or type a topic. <strong>Generate Post</strong> writes a
            standalone draft; <strong>Generate Reply</strong> writes a reply (best with an X status URL
            so Approve &amp; Post can thread it). Style:{' '}
            {draftStyle === DRAFT_STYLE.meme ? 'meme' : draftStyle}.
          </p>
        </div>

        <form onSubmit={handleSubmit} className="flex flex-col gap-2">
          <label className="form-control w-full">
            <textarea
              className="textarea textarea-bordered textarea-sm w-full min-h-[4.5rem]"
              placeholder="https://x.com/user/status/… · https://example.com/story · or a topic"
              value={value}
              onChange={(event) => onChange(event.target.value)}
              disabled={disabled || busy}
              data-testid="custom-draft-input"
            />
          </label>

          <div className="flex flex-wrap gap-2 justify-end">
            <button
              type="submit"
              className="btn btn-sm btn-primary"
              disabled={baseDisabled}
              title={emptyHint ?? 'Generate one standalone post from this input'}
              data-testid="custom-draft-generate"
            >
              {generatingPost ? (
                <>
                  <span className="loading loading-spinner loading-xs" />
                  Generating…
                </>
              ) : (
                'Generate Post'
              )}
            </button>

            <button
              type="button"
              className="btn btn-sm btn-secondary"
              disabled={baseDisabled}
              title={emptyHint ?? 'Generate one reply from this input'}
              onClick={() => {
                if (!baseDisabled) onGenerateReply()
              }}
              data-testid="custom-reply-generate"
            >
              {generatingReply ? (
                <>
                  <span className="loading loading-spinner loading-xs" />
                  Generating…
                </>
              ) : (
                'Generate Reply'
              )}
            </button>
          </div>
        </form>
      </div>
    </div>
  )
}
