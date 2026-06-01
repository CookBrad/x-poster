import { useState, useEffect, useCallback, useMemo } from 'react'
import { invoke } from '@tauri-apps/api/core'
import ApiKeySettings from './components/ApiKeySettings'
import { 
  getDrafts, 
  createDraft, 
  updateDraft, 
  deleteDraft, 
  markDraftPosted,
  parseSources,
  runResearch,
  getLatestResearchRun,
  getAllHistoricalSources,
  type Draft,
  type ResearchRunWithSources,
  type HistoricalResearchSource
} from './lib/db'

type Tab = 'queue' | 'research' | 'settings' | 'history'

function App() {
  const [activeTab, setActiveTab] = useState<Tab>('queue')

  // Persisted xAI key (loaded from backend or .env fallback)
  const [savedXaiKey, setSavedXaiKey] = useState<string>('')

  // Load saved xAI key from backend on mount (with .env fallback)
  useEffect(() => {
    async function loadKey() {
      try {
        const stored = await invoke<string | null>('get_setting', { key: 'xai_api_key' })
        if (stored) {
          setSavedXaiKey(stored)
        } else {
          // Fallback to .env during development
          const envKey = import.meta.env.VITE_XAI_API_KEY as string | undefined
          if (envKey) {
            setSavedXaiKey(envKey)
          }
        }
      } catch (e) {
        console.error('Failed to load saved xAI key', e)
        // fallback to env
        const envKey = import.meta.env.VITE_XAI_API_KEY as string | undefined
        if (envKey) setSavedXaiKey(envKey)
      }
    }
    loadKey()
  }, [])

  return (
    <div className="min-h-screen bg-base-200">
      {/* Top bar */}
      <div className="navbar bg-base-100 border-b px-4">
        <div className="flex-1">
          <span className="text-2xl font-semibold tracking-tight">x-poster</span>
          <span className="ml-2 text-xs opacity-60 align-super">Tesla • TSLA • Elon (non-political)</span>
        </div>
        <div className="flex-none">
          <div className="badge badge-outline badge-sm mr-3">local • dev</div>
          <button className="btn btn-sm btn-primary" onClick={() => window.location.reload()}>
            Refresh
          </button>
        </div>
      </div>

      {/* Tabs (daisyUI) */}
      <div className="tabs tabs-bordered tabs-lg bg-base-100 px-4 pt-2">
        <a
          className={`tab ${activeTab === 'queue' ? 'tab-active' : ''}`}
          onClick={() => setActiveTab('queue')}
        >
          Queue
        </a>
        <a
          className={`tab ${activeTab === 'research' ? 'tab-active' : ''}`}
          onClick={() => setActiveTab('research')}
        >
          Research
        </a>
        <a
          className={`tab ${activeTab === 'settings' ? 'tab-active' : ''}`}
          onClick={() => setActiveTab('settings')}
        >
          Settings
        </a>
        <a
          className={`tab ${activeTab === 'history' ? 'tab-active' : ''}`}
          onClick={() => setActiveTab('history')}
        >
          History
        </a>
      </div>

      {/* Content area */}
      <div className="p-6 max-w-6xl mx-auto">
        {activeTab === 'queue' && (
          <QueueTab />
        )}

        {activeTab === 'research' && (
          <ResearchTab />
        )}

        {activeTab === 'settings' && (
          <div className="max-w-2xl">
            <h2 className="text-2xl font-semibold mb-4">Settings</h2>

            <div className="card bg-base-100 mb-6">
              <div className="card-body">
                <h3 className="font-semibold mb-2">API Keys</h3>
                <p className="text-sm mb-3 opacity-80">
                  Enter your xAI API key below and click <strong>Save Key</strong>. 
                  The key is stored locally in the app (SQLite).
                </p>

                <ApiKeySettings 
                  initialSavedKey={savedXaiKey} 
                  onKeySaved={setSavedXaiKey}
                  onKeyCleared={() => setSavedXaiKey('')}
                />
              </div>
            </div>


          </div>
        )}

        {activeTab === 'history' && (
          <div>
            <h2 className="text-2xl font-semibold mb-4">Posted History</h2>
            <p className="opacity-70">Your previously approved posts will be listed here with direct links to X.</p>
          </div>
        )}
      </div>

      <footer className="text-center text-xs opacity-50 py-6">
        x-poster • local only • human approval required (MVP)
      </footer>
    </div>
  )
}

// ============================================
// QueueTab - Database-backed draft queue
// ============================================
function QueueTab() {
  const [drafts, setDrafts] = useState<Draft[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const loadDrafts = useCallback(async () => {
    try {
      setLoading(true)
      const data = await getDrafts()
      setDrafts(data)
      setError(null)
    } catch (e: any) {
      console.error('Failed to load drafts', e)
      setError('Failed to load drafts from database')
    } finally {
      setLoading(false)
    }
  }, [])

  // Load drafts when component mounts
  useEffect(() => {
    loadDrafts()
  }, [loadDrafts])

  const handleCreateTestDraft = async () => {
    try {
      const testDraft = {
        text: 'Tesla delivered a record number of vehicles this quarter, with strong growth in energy storage and FSD adoption continuing to accelerate.',
        sources_json: JSON.stringify([
          { type: 'x_post', user: '@Tesla', text: 'Q2 delivery numbers are in...' },
          { type: 'rss', source: 'Electrek', title: 'Tesla Q2 deliveries beat expectations' }
        ]),
        image_url: null,
      }
      await createDraft(testDraft)
      await loadDrafts()
    } catch (e: any) {
      alert('Failed to create test draft: ' + e)
    }
  }

  const handleSkip = async (id: string) => {
    try {
      await updateDraft(id, { status: 'skipped' })
      await loadDrafts()
    } catch (e: any) {
      alert('Failed to skip draft: ' + e)
    }
  }

  const handleDelete = async (draft: Draft) => {
    const isPosted = draft.status === 'posted';
    const itemType = isPosted ? 'post' : 'draft';

    // Only confirm for posted items (more destructive action)
    if (isPosted) {
      const confirmed = confirm(
        'Delete this posted item from your local history?\n\n(This will NOT delete the actual tweet from X)'
      );
      if (!confirmed) return;
    }

    // Optimistic update: remove the card from the UI immediately
    setDrafts(prev => prev.filter(d => d.id !== draft.id));

    try {
      await deleteDraft(draft.id);
      // Re-sync with the database to stay consistent
      await loadDrafts();
    } catch (e: any) {
      alert(`Failed to delete ${itemType}: ${e?.message || e}`);
      // Rollback the optimistic removal on failure
      await loadDrafts();
    }
  }

  const handleApprovePost = async (id: string) => {
    // For now we simulate posting by marking it as posted with a fake ID.
    // Later this will call the real X posting logic.
    const fakePostId = 'sim_' + Date.now()
    try {
      await markDraftPosted(id, fakePostId)
      alert('Draft marked as posted (simulated). Real X posting coming soon!')
      await loadDrafts()
    } catch (e: any) {
      alert('Failed to mark as posted: ' + e)
    }
  }

  if (loading) {
    return <div className="flex justify-center py-12"><span className="loading loading-spinner loading-lg"></span></div>
  }

  return (
    <div>
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-2xl font-semibold">Draft Queue</h2>
        <div className="flex gap-2">
          <button className="btn btn-primary btn-sm" onClick={handleCreateTestDraft}>
            + Create Test Draft
          </button>
          <button className="btn btn-outline btn-sm" onClick={loadDrafts}>
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
          <span>No drafts yet. Click "Create Test Draft" to add one and test the database.</span>
        </div>
      ) : (
        <div className="grid gap-4 md:grid-cols-2">
          {drafts.map((draft) => {
            const sources = parseSources(draft)
            return (
              <div key={draft.id} className="card bg-base-100 shadow draft-card">
                <div className="card-body">
                  <div className="flex justify-between text-xs opacity-70 mb-1">
                    <span className="badge badge-sm">{draft.status}</span>
                    <span>{new Date(draft.created_at).toLocaleString()}</span>
                  </div>

                  <p className="font-medium whitespace-pre-wrap">{draft.text}</p>

                  {sources.length > 0 && (
                    <div className="text-xs opacity-60 mt-2">
                      Sources: {sources.map((s: any, i: number) => s.user || s.source || s.title).join(', ')}
                    </div>
                  )}

                  {draft.x_post_id && (
                    <div className="text-xs text-success mt-1">
                      Posted as: {draft.x_post_id}
                    </div>
                  )}

                  <div className="card-actions justify-end mt-4 gap-2">
                    <button 
                      className="btn btn-ghost btn-sm" 
                      onClick={() => alert('Editing coming soon')}
                    >
                      Edit
                    </button>
                    
                    {draft.status === 'pending' && (
                      <>
                        <button 
                          className="btn btn-success btn-sm"
                          onClick={() => handleApprovePost(draft.id)}
                        >
                          Approve &amp; Post
                        </button>
                        <button 
                          className="btn btn-ghost btn-sm"
                          onClick={() => handleSkip(draft.id)}
                        >
                          Skip
                        </button>
                      </>
                    )}
                    
                    <button 
                      className="btn btn-error btn-sm btn-outline"
                      onClick={() => handleDelete(draft)}
                    >
                      {draft.status === 'posted' ? 'Delete Post' : 'Delete Draft'}
                    </button>
                  </div>
                </div>
              </div>
            )
          })}
        </div>
      )}
    </div>
  )
}

// ============================================
// ResearchTab - Current + Historical research runs
// ============================================
type ResearchSubTab = 'current' | 'historical';

function ResearchTab() {
  const [activeSubTab, setActiveSubTab] = useState<ResearchSubTab>('current');

  const [currentRun, setCurrentRun] = useState<ResearchRunWithSources | null>(null);
  const [historicalSources, setHistoricalSources] = useState<HistoricalResearchSource[]>([]);

  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadLatest = async () => {
    try {
      const run = await getLatestResearchRun();
      setCurrentRun(run);
    } catch (e: any) {
      console.error(e);
    }
  };

  const loadAllHistorical = async () => {
    try {
      const allSources = await getAllHistoricalSources();
      setHistoricalSources(allSources);
    } catch (e: any) {
      console.error(e);
    }
  };

  const handleRunResearch = async () => {
    setLoading(true);
    setError(null);

    try {
      const newRun = await runResearch();   // imported from ./lib/db
      setCurrentRun(newRun);
      setActiveSubTab('current');
      await loadAllHistorical(); // refresh history list
    } catch (e: any) {
      console.error(e);
      setError('Research failed. Check your xAI API key and connection.');
    } finally {
      setLoading(false);
    }
  };

  // Load latest on mount
  useEffect(() => {
    loadLatest();
    loadAllHistorical();
  }, []);

  return (
    <div>
      <h2 className="text-2xl font-semibold mb-2">Research — Musk Companies Only</h2>
      <p className="mb-4 text-sm opacity-70">
        Focused strictly on <strong>Elon Musk's companies</strong> (Tesla, SpaceX, xAI, Neuralink, Boring Company).<br />
        General EV news is excluded.<br />
        <span className="text-warning text-xs">X posts are discovered via Grok — your xAI API key must be set in Settings.</span>
      </p>

      <div className="flex gap-2 mb-4">
        <button 
          className={`btn btn-sm ${activeSubTab === 'current' ? 'btn-primary' : 'btn-outline'}`}
          onClick={() => setActiveSubTab('current')}
        >
          Current
        </button>
        <button 
          className={`btn btn-sm ${activeSubTab === 'historical' ? 'btn-primary' : 'btn-outline'}`}
          onClick={() => setActiveSubTab('historical')}
        >
          Historical
        </button>
      </div>

      {activeSubTab === 'current' && (
        <div>
          <button 
            className="btn btn-primary mb-6" 
            onClick={handleRunResearch}
            disabled={loading}
          >
            {loading ? 'Researching...' : 'Run Research Now'}
          </button>

          {error && <div className="alert alert-error mb-4">{error}</div>}

          {currentRun ? (
            <div>
              <h3 className="text-lg font-semibold mb-1">
                Latest Research Run — {new Date(currentRun.run.run_at).toLocaleString()}
              </h3>

              {/* Source breakdown */}
              {(() => {
                const rssCount = currentRun.sources.filter(s => s.source_type === 'rss').length;
                const grokXCount = currentRun.sources.filter(s => s.source_type === 'x_grok').length;
                return (
                  <p className="text-xs mb-4 opacity-75">
                    {currentRun.sources.length} total sources → {rssCount} from RSS, {grokXCount} from X (via Grok)
                  </p>
                );
              })()}

              {currentRun.sources.every(s => s.source_type !== 'x_grok') && (
                <div className="alert alert-warning mb-4 text-sm">
                  No X posts were returned by Grok this time. 
                  Make sure your <strong>xAI API key</strong> is set in Settings. 
                  Grok is currently the only source for X content.
                </div>
              )}

              <div className="space-y-3">
                {currentRun.sources.map((source, index) => (
                  <div key={index} className="card bg-base-100 shadow-sm">
                    <div className="card-body py-3">
                      <div className="flex justify-between items-start">
                        <div>
                          <a 
                            href={source.url} 
                            target="_blank" 
                            rel="noopener noreferrer"
                            className="font-medium hover:underline"
                          >
                            {source.title}
                          </a>
                          <div className="text-xs opacity-60 mt-0.5">
                            {source.source_name} • {source.published_at ? new Date(source.published_at).toLocaleDateString() : 'Unknown date'}
                          </div>
                        </div>
                        <div className="badge badge-outline badge-sm">{source.source_type}</div>
                      </div>
                      <p className="text-sm line-clamp-2 opacity-80 mt-1">{source.content}</p>
                      {source.source_type === 'x_grok' && (
                        <div className="text-[10px] text-emerald-600 font-medium mt-1">★ Grok-curated high-signal post</div>
                      )}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          ) : (
            <div className="alert alert-info">
              No research run yet. Click "Run Research Now" to start.<br />
              <span className="text-xs">Note: X posts come via Grok — make sure your xAI API key is configured in Settings.</span>
            </div>
          )}
        </div>
      )}

      {activeSubTab === 'historical' && (
        <HistoricalSourcesList />
      )}
    </div>
  );
}

// ============================================
// HistoricalSourcesList - Flat aggregated list of all research sources (paginated + searchable)
// ============================================
function HistoricalSourcesList() {
  const [allSources, setAllSources] = useState<HistoricalResearchSource[]>([]);
  const [searchTerm, setSearchTerm] = useState('');
  const [pageSize, setPageSize] = useState(25);
  const [currentPage, setCurrentPage] = useState(1);

  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadAll = async () => {
    setLoading(true);
    setError(null);
    try {
      const all = await getAllHistoricalSources();
      setAllSources(all);
    } catch (e: any) {
      console.error(e);
      setError('Failed to load historical research sources.');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadAll();
  }, []);

  // Filter sources based on search term
  const filteredSources = useMemo(() => {
    if (!searchTerm.trim()) return allSources;

    const term = searchTerm.toLowerCase().trim();
    return allSources.filter(source =>
      source.title.toLowerCase().includes(term) ||
      source.content.toLowerCase().includes(term) ||
      source.source_name.toLowerCase().includes(term)
    );
  }, [allSources, searchTerm]);

  // Calculate pagination
  const totalItems = filteredSources.length;
  const totalPages = Math.max(1, Math.ceil(totalItems / pageSize));
  const startIndex = (currentPage - 1) * pageSize;
  const endIndex = Math.min(startIndex + pageSize, totalItems);
  const paginatedSources = filteredSources.slice(startIndex, endIndex);

  // Reset to page 1 when search or page size changes
  useEffect(() => {
    setCurrentPage(1);
  }, [searchTerm, pageSize]);

  // Ensure current page is valid if total pages change
  useEffect(() => {
    if (currentPage > totalPages) {
      setCurrentPage(totalPages);
    }
  }, [totalPages, currentPage]);

  if (loading) {
    return <div className="flex justify-center py-12"><span className="loading loading-spinner loading-lg"></span></div>;
  }

  if (error) {
    return <div className="alert alert-error">{error}</div>;
  }

  if (allSources.length === 0) {
    return <div className="alert alert-info">No historical research sources yet. Run research a few times to build up history.</div>;
  }

  return (
    <div>
      {/* Controls */}
      <div className="flex flex-col md:flex-row gap-4 mb-4 items-start md:items-center justify-between">
        <div className="flex items-center gap-4 flex-wrap">
          {/* Search */}
          <div className="form-control w-full max-w-xs">
            <input
              type="text"
              placeholder="Search sources..."
              className="input input-bordered input-sm w-full"
              value={searchTerm}
              onChange={(e) => setSearchTerm(e.target.value)}
            />
          </div>

          {/* Page Size */}
          <div className="flex items-center gap-2">
            <span className="text-sm opacity-70">Per page:</span>
            <select 
              className="select select-bordered select-sm"
              value={pageSize}
              onChange={(e) => setPageSize(Number(e.target.value))}
            >
              <option value={10}>10</option>
              <option value={25}>25</option>
              <option value={50}>50</option>
              <option value={100}>100</option>
            </select>
          </div>
        </div>

        {/* Pagination Info + Controls */}
        <div className="flex items-center gap-4">
          <span className="text-sm opacity-70">
            Showing {totalItems === 0 ? 0 : startIndex + 1}–{endIndex} of {totalItems}
            {searchTerm && ` (filtered from ${allSources.length})`}
          </span>

          <div className="join">
            <button 
              className="btn btn-sm join-item"
              onClick={() => setCurrentPage(p => Math.max(1, p - 1))}
              disabled={currentPage === 1}
            >
              ← Prev
            </button>
            <button className="btn btn-sm join-item pointer-events-none">
              Page {currentPage} of {totalPages}
            </button>
            <button 
              className="btn btn-sm join-item"
              onClick={() => setCurrentPage(p => Math.min(totalPages, p + 1))}
              disabled={currentPage === totalPages}
            >
              Next →
            </button>
          </div>
        </div>
      </div>

      {/* Results */}
      {paginatedSources.length === 0 ? (
        <div className="alert alert-info">No sources match your search.</div>
      ) : (
        <div className="space-y-3">
          {paginatedSources.map((source, index) => (
            <div key={`${source.id}-${index}`} className="card bg-base-100 shadow-sm">
              <div className="card-body py-3">
                <div className="flex justify-between items-start">
                  <div>
                    <a 
                      href={source.url} 
                      target="_blank" 
                      rel="noopener noreferrer"
                      className="font-medium hover:underline text-sm"
                    >
                      {source.title}
                    </a>
                    <div className="text-xs opacity-60 mt-0.5">
                      {source.source_name} • {source.published_at 
                        ? new Date(source.published_at).toLocaleDateString() 
                        : new Date(source.run_at).toLocaleDateString()}
                    </div>
                  </div>
                  <div className="badge badge-outline badge-sm">{source.source_type}</div>
                </div>
                <p className="text-sm line-clamp-2 opacity-80 mt-1">{source.content}</p>
                {source.source_type === 'x_grok' && (
                  <div className="text-[10px] text-emerald-600 font-medium mt-1">★ Grok-curated high-signal post</div>
                )}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

export default App
