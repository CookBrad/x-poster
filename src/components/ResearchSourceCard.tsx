import { RESEARCH_SOURCE_TYPE } from '../lib/constants'
import { openUrl } from '@tauri-apps/plugin-opener'

export interface ResearchSourceCardProps {
  sourceId: string
  title: string
  content: string
  url: string
  sourceName: string
  sourceType: string
  dateLabel: string
  canGenerate?: boolean
  isUsed?: boolean
  generating?: boolean
  onGeneratePost?: (sourceId: string) => void
}

export function ResearchSourceCard({
  sourceId,
  title,
  content,
  url,
  sourceName,
  sourceType,
  dateLabel,
  canGenerate = false,
  isUsed = false,
  generating = false,
  onGeneratePost,
}: ResearchSourceCardProps) {
  const allowGenerate = canGenerate && !isUsed
  const displaySourceName =
    sourceType === RESEARCH_SOURCE_TYPE.rss
      ? `source: ${sourceName.replace(/^@/, '')}`
      : sourceName.startsWith('@')
        ? sourceName
        : `@${sourceName.replace(/^@/, '')}`

  const handleGenerate = () => {
    onGeneratePost?.(sourceId)
  }

  return (
    <div
      className={`card bg-base-100 shadow-sm ${allowGenerate ? 'cursor-pointer hover:shadow-md transition-shadow' : ''} ${isUsed ? 'opacity-70' : ''}`}
      onClick={allowGenerate && !generating ? handleGenerate : undefined}
      onKeyDown={(event) => {
        if (!allowGenerate || generating || !onGeneratePost) return
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault()
          handleGenerate()
        }
      }}
      role={allowGenerate ? 'button' : undefined}
      tabIndex={allowGenerate ? 0 : undefined}
      data-testid={`research-source-card-${sourceId}`}
    >
      <div className="card-body py-3">
        <div className="flex justify-between items-start gap-3">
          <div className="min-w-0 flex-1">
            {url ? (
              <span
                className="font-medium hover:underline text-sm cursor-pointer"
                onClick={async (event) => {
                  event.stopPropagation()
                  try {
                    await openUrl(url)
                  } catch (err) {
                    console.error('Failed to open URL via opener, falling back', err)
                    window.open(url, '_blank')
                  }
                }}
              >
                {title}
              </span>
            ) : (
              <span className="font-medium text-sm">{title}</span>
            )}
            <div className="text-xs opacity-60 mt-0.5">
              {displaySourceName} • {dateLabel}
            </div>
          </div>
          <div className="flex flex-col items-end gap-1 shrink-0">
            {isUsed && (
              <div className="badge badge-neutral badge-sm" data-testid={`used-badge-${sourceId}`}>
                Used
              </div>
            )}
            <div className="badge badge-outline badge-sm">{sourceType}</div>
          </div>
        </div>

        <p className="text-sm line-clamp-2 opacity-80 mt-1">{content}</p>

        {sourceType === RESEARCH_SOURCE_TYPE.xGrok && (
          <div className="text-[10px] text-emerald-600 font-medium mt-1">
            ★ Grok-curated high-signal post
          </div>
        )}

        {canGenerate && !isUsed && (
          <div className="card-actions justify-end mt-2">
            <button
              type="button"
              className="btn btn-primary btn-xs"
              onClick={(event) => {
                event.stopPropagation()
                handleGenerate()
              }}
              disabled={generating}
              data-testid={`generate-post-${sourceId}`}
            >
              {generating ? 'Generating…' : 'Generate Post'}
            </button>
          </div>
        )}
      </div>
    </div>
  )
}

export function formatResearchSourceDate(
  publishedAt: string | null | undefined,
  fallbackDate: string
): string {
  const date = publishedAt ?? fallbackDate
  return new Date(date).toLocaleDateString()
}