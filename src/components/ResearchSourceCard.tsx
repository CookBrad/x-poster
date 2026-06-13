import { RESEARCH_SOURCE_TYPE } from '../lib/constants'

interface ResearchSourceCardProps {
  title: string
  content: string
  url: string
  sourceName: string
  sourceType: string
  dateLabel: string
}

export function ResearchSourceCard({
  title,
  content,
  url,
  sourceName,
  sourceType,
  dateLabel,
}: ResearchSourceCardProps) {
  return (
    <div className="card bg-base-100 shadow-sm">
      <div className="card-body py-3">
        <div className="flex justify-between items-start">
          <div>
            {url ? (
              <a
                href={url}
                target="_blank"
                rel="noopener noreferrer"
                className="font-medium hover:underline text-sm"
              >
                {title}
              </a>
            ) : (
              <span className="font-medium text-sm">{title}</span>
            )}
            <div className="text-xs opacity-60 mt-0.5">
              {sourceType === RESEARCH_SOURCE_TYPE.rss
                ? `source: ${sourceName.replace(/^@/, '')}`
                : sourceName.startsWith('@')
                  ? sourceName
                  : `@${sourceName.replace(/^@/, '')}`}{' '}
              • {dateLabel}
            </div>
          </div>
          <div className="badge badge-outline badge-sm">{sourceType}</div>
        </div>
        <p className="text-sm line-clamp-2 opacity-80 mt-1">{content}</p>
        {sourceType === RESEARCH_SOURCE_TYPE.xGrok && (
          <div className="text-[10px] text-emerald-600 font-medium mt-1">
            ★ Grok-curated high-signal post
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