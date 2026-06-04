import { useEffect, useState, useCallback } from 'react'
import { getDrafts, parseSources, type Draft } from '../lib/db'

export default function HistoryTab() {
  const [posted, setPosted] = useState<Draft[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(async () => {
    try {
      setLoading(true)
      const data = await getDrafts('posted')
      setPosted(data)
      setError(null)
    } catch (e: unknown) {
      setError('Failed to load posted history')
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    void load()
  }, [load])

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
        <h2 className="text-2xl font-semibold">Posted History</h2>
        <button type="button" className="btn btn-outline btn-sm" onClick={() => void load()}>
          Refresh
        </button>
      </div>

      {error && <div className="alert alert-error mb-4">{error}</div>}

      {posted.length === 0 ? (
        <div className="alert alert-info">
          No posted drafts yet. Approve a draft from the Queue to post it to X.
        </div>
      ) : (
        <div className="space-y-3">
          {posted.map((draft) => {
            const sources = parseSources(draft)
            const xUrl =
              draft.x_post_id && !draft.x_post_id.startsWith('sim_')
                ? `https://x.com/i/web/status/${draft.x_post_id}`
                : null

            return (
              <div key={draft.id} className="card bg-base-100 shadow-sm" data-testid="history-item">
                <div className="card-body py-4">
                  <div className="flex justify-between text-xs opacity-70 mb-1">
                    <span>{draft.posted_at ? new Date(draft.posted_at).toLocaleString() : '—'}</span>
                    {xUrl && (
                      <a
                        href={xUrl}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="link link-primary"
                      >
                        View on X →
                      </a>
                    )}
                  </div>
                  <p className="text-sm whitespace-pre-wrap">{draft.text}</p>
                  {sources.length > 0 && (
                    <p className="text-xs opacity-60 mt-2">
                      Sources: {sources.map((s: { title?: string }) => s.title).filter(Boolean).join(', ')}
                    </p>
                  )}
                </div>
              </div>
            )
          })}
        </div>
      )}
    </div>
  )
}