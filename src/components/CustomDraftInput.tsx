import type { FormEvent } from 'react'
import { DRAFT_STYLE, type DraftStyle } from '../lib/constants'

export interface CustomDraftInputProps {
  value: string
  onChange: (value: string) => void
  onGenerate: () => void
  generating?: boolean
  disabled?: boolean
  hasXaiKey?: boolean
  draftStyle?: DraftStyle
}

export function CustomDraftInput({
  value,
  onChange,
  onGenerate,
  generating = false,
  disabled = false,
  hasXaiKey = false,
  draftStyle = DRAFT_STYLE.insight,
}: CustomDraftInputProps) {
  const trimmed = value.trim()
  const generateDisabled = disabled || generating || !hasXaiKey || trimmed.length === 0

  const handleSubmit = (event: FormEvent) => {
    event.preventDefault()
    if (!generateDisabled) {
      onGenerate()
    }
  }

  return (
    <div className="card bg-base-200/60 mb-4">
      <div className="card-body py-4 gap-3">
        <div>
          <h3 className="font-semibold text-base">Generate from a link or topic</h3>
          <p className="text-xs opacity-70 mt-1">
            Paste an article or X post URL, or type a topic — Grok will write one{' '}
            {draftStyle === DRAFT_STYLE.meme ? 'meme-style ' : ''}
            draft post using the selected post style.
          </p>
        </div>

        <form onSubmit={handleSubmit} className="flex flex-col sm:flex-row gap-2">
          <label className="form-control flex-1">
            <textarea
              className="textarea textarea-bordered textarea-sm w-full min-h-[4.5rem]"
              placeholder="https://example.com/story or Starship booster catch milestone"
              value={value}
              onChange={(event) => onChange(event.target.value)}
              disabled={disabled || generating}
              data-testid="custom-draft-input"
            />
          </label>

          <button
            type="submit"
            className="btn btn-sm btn-primary sm:self-end"
            disabled={generateDisabled}
            title={
              !hasXaiKey
                ? 'xAI key required'
                : trimmed.length === 0
                  ? 'Enter a link or topic'
                  : 'Generate one post from this input'
            }
            data-testid="custom-draft-generate"
          >
            {generating ? (
              <>
                <span className="loading loading-spinner loading-xs" />
                Generating…
              </>
            ) : (
              'Generate Post'
            )}
          </button>
        </form>
      </div>
    </div>
  )
}