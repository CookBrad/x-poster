import { useEffect, useMemo, useState } from 'react'
import { getAllHistoricalSources, type HistoricalResearchSource } from '../lib/db'
import { errorMessage } from '../lib/errors'
import { isResearchSourceUsed } from '../lib/researchSource'
import { formatResearchSourceDate, ResearchSourceCard } from './ResearchSourceCard'

interface HistoricalSourcesListProps {
  reloadToken: number
  hasXaiKey?: boolean
  generatingSourceId?: string | null
  onGenerateFromSource?: (sourceId: string) => void
}

const PAGE_SIZE_OPTIONS = [10, 25, 50, 100] as const

export function HistoricalSourcesList({
  reloadToken,
  hasXaiKey = false,
  generatingSourceId = null,
  onGenerateFromSource,
}: HistoricalSourcesListProps) {
  const [allSources, setAllSources] = useState<HistoricalResearchSource[]>([])
  const [searchTerm, setSearchTerm] = useState('')
  const [pageSize, setPageSize] = useState(25)
  const [currentPage, setCurrentPage] = useState(1)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    setSearchTerm('')
    setCurrentPage(1)
    setAllSources([])
    setLoading(true)
    setError(null)

    let cancelled = false

    void (async () => {
      try {
        const sources = await getAllHistoricalSources()
        if (!cancelled) {
          setAllSources(sources)
        }
      } catch (error: unknown) {
        console.error(error)
        if (!cancelled) {
          setError(errorMessage(error, 'Failed to load historical research sources.'))
        }
      } finally {
        if (!cancelled) {
          setLoading(false)
        }
      }
    })()

    return () => {
      cancelled = true
    }
  }, [reloadToken])

  const filteredSources = useMemo(() => {
    const term = searchTerm.trim().toLowerCase()
    if (!term) {
      return allSources
    }

    return allSources.filter(
      (source) =>
        source.title.toLowerCase().includes(term) ||
        source.content.toLowerCase().includes(term) ||
        source.source_name.toLowerCase().includes(term)
    )
  }, [allSources, searchTerm])

  const totalItems = filteredSources.length
  const totalPages = Math.max(1, Math.ceil(totalItems / pageSize))
  const startIndex = (currentPage - 1) * pageSize
  const endIndex = Math.min(startIndex + pageSize, totalItems)
  const paginatedSources = filteredSources.slice(startIndex, endIndex)

  useEffect(() => {
    setCurrentPage(1)
  }, [searchTerm, pageSize])

  useEffect(() => {
    if (currentPage > totalPages) {
      setCurrentPage(totalPages)
    }
  }, [totalPages, currentPage])

  if (loading) {
    return (
      <div className="flex justify-center py-12">
        <span className="loading loading-spinner loading-lg" />
      </div>
    )
  }

  if (error) {
    return <div className="alert alert-error">{error}</div>
  }

  if (allSources.length === 0) {
    return (
      <div className="alert alert-info">
        No historical research sources yet. Run research a few times to build up history.
      </div>
    )
  }

  return (
    <div>
      <div className="flex flex-col md:flex-row gap-4 mb-4 items-start md:items-center justify-between">
        <div className="flex items-center gap-4 flex-wrap">
          <div className="form-control w-full max-w-xs">
            <input
              type="text"
              placeholder="Search sources..."
              className="input input-bordered input-sm w-full"
              value={searchTerm}
              onChange={(event) => setSearchTerm(event.target.value)}
            />
          </div>

          <div className="flex items-center gap-2">
            <span className="text-sm opacity-70">Per page:</span>
            <select
              className="select select-bordered select-sm"
              value={pageSize}
              onChange={(event) => setPageSize(Number(event.target.value))}
            >
              {PAGE_SIZE_OPTIONS.map((size) => (
                <option key={size} value={size}>
                  {size}
                </option>
              ))}
            </select>
          </div>
        </div>

        <div className="flex items-center gap-4">
          <span className="text-sm opacity-70">
            Showing {totalItems === 0 ? 0 : startIndex + 1}–{endIndex} of {totalItems}
            {searchTerm && ` (filtered from ${allSources.length})`}
          </span>

          <div className="join">
            <button
              type="button"
              className="btn btn-sm join-item"
              onClick={() => setCurrentPage((page) => Math.max(1, page - 1))}
              disabled={currentPage === 1}
            >
              ← Prev
            </button>
            <button type="button" className="btn btn-sm join-item pointer-events-none">
              Page {currentPage} of {totalPages}
            </button>
            <button
              type="button"
              className="btn btn-sm join-item"
              onClick={() => setCurrentPage((page) => Math.min(totalPages, page + 1))}
              disabled={currentPage === totalPages}
            >
              Next →
            </button>
          </div>
        </div>
      </div>

      {paginatedSources.length === 0 ? (
        <div className="alert alert-info">No sources match your search.</div>
      ) : (
        <div className="space-y-3">
          {paginatedSources.map((source) => (
            <ResearchSourceCard
              key={source.id}
              sourceId={source.id}
              title={source.title}
              content={source.content}
              url={source.url}
              sourceName={source.source_name}
              sourceType={source.source_type}
              dateLabel={formatResearchSourceDate(source.published_at, source.run_at)}
              canGenerate={hasXaiKey}
              isUsed={isResearchSourceUsed(source)}
              generating={generatingSourceId === source.id}
              onGeneratePost={onGenerateFromSource}
            />
          ))}
        </div>
      )}
    </div>
  )
}