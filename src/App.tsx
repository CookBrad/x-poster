import { useState, useEffect, useMemo } from 'react'
import { invoke } from '@tauri-apps/api/core'
import ApiKeySettings from './components/ApiKeySettings'
import QueueTab from './components/QueueTab'
import HistoryTab from './components/HistoryTab'
import XCredentialsSettings from './components/XCredentialsSettings'
import {
  runResearch,
  getLatestResearchRun,
  getAllHistoricalSources,
  resetResearchData,
  generateDraftsFromLatestResearch,
  type ResearchRunWithSources,
  type HistoricalResearchSource,
} from './lib/db'

type Tab = 'queue' | 'research' | 'settings' | 'history'

function App() {
  const [activeTab, setActiveTab] = useState<Tab>('queue')

  // Aggressively ensure dark colorful theme (synthwave) is applied
  useEffect(() => {
    document.documentElement.setAttribute('data-theme', 'synthwave')
  }, [])

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
      <div className="navbar bg-base-100 border-b border-primary/30 px-4">
        <div className="flex-1">
          <span className="text-2xl font-semibold tracking-tight text-primary">x-poster</span>
          <span className="ml-2 text-xs opacity-70 align-super text-secondary">Tesla • TSLA • Elon (non-political)</span>
        </div>
        <div className="flex-none">
          <div className="badge badge-primary badge-sm badge-outline mr-3">local • dev</div>
          <button className="btn btn-sm btn-primary" onClick={() => window.location.reload()}>
            Refresh
          </button>
        </div>
      </div>

      {/* Tabs (daisyUI) */}
      <div className="tabs tabs-bordered tabs-lg bg-base-100 border-b border-primary/20 px-4 pt-2">
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
                <XCredentialsSettings />
              </div>
            </div>
          </div>
        )}

        {activeTab === 'history' && <HistoryTab />}
      </div>

      <footer className="text-center text-xs opacity-50 py-6">
        x-poster • local only • human approval required (MVP)
      </footer>
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
  const [hasXaiKey, setHasXaiKey] = useState<boolean>(false);
  const [historicalResetKey, setHistoricalResetKey] = useState(0);
  const [showResetConfirm, setShowResetConfirm] = useState(false);

  const [loading, setLoading] = useState(false);
  const [generating, setGenerating] = useState(false);
  const [isResetting, setIsResetting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [resetSuccess, setResetSuccess] = useState<string | null>(null);
  const [generateSuccess, setGenerateSuccess] = useState<string | null>(null);

  const loadLatest = async () => {
    try {
      const run = await getLatestResearchRun();
      setCurrentRun(run);
    } catch (e: any) {
      console.error(e);
    }
  };

  const handleRunResearch = async (mode: 'rss' | 'x' | 'both' = 'both') => {
    setLoading(true);
    setError(null);
    setGenerateSuccess(null);

    try {
      const newRun = await runResearch(mode);
      setCurrentRun(newRun);
      setActiveSubTab('current');
    } catch (e: unknown) {
      console.error(e);
      setError(e instanceof Error ? e.message : 'Research failed. Check your xAI API key and connection.');
    } finally {
      setLoading(false);
    }
  };

  const handleGenerateDrafts = async () => {
    setGenerating(true);
    setError(null);
    setGenerateSuccess(null);

    try {
      const drafts = await generateDraftsFromLatestResearch(3);
      setGenerateSuccess(
        `Generated ${drafts.length} draft(s) with fresh-take prompts. Open the Queue tab to review.`
      );
    } catch (e: unknown) {
      console.error(e);
      setError(e instanceof Error ? e.message : 'Draft generation failed.');
    } finally {
      setGenerating(false);
    }
  };

  const performResetResearchData = async () => {
    setShowResetConfirm(false);
    setIsResetting(true);
    setError(null);
    setResetSuccess(null);

    try {
      const result = await resetResearchData();

      // Belt-and-suspenders: confirm the DB is actually empty before updating UI.
      const remaining = await getAllHistoricalSources();
      if (remaining.length > 0) {
        throw new Error(
          `Reset reported success but ${remaining.length} historical source(s) still remain in the database.`
        );
      }

      setCurrentRun(null);
      setHistoricalResetKey(prev => prev + 1);
      await loadLatest();
      setActiveSubTab('historical');
      setResetSuccess(
        `Deleted ${result.deleted_sources} source(s) and ${result.deleted_runs} research run(s).`
      );
    } catch (e: unknown) {
      console.error(e);
      const message = e instanceof Error ? e.message : String(e);
      setError('Failed to reset research data: ' + message);
    } finally {
      setIsResetting(false);
    }
  };

  // Load latest on mount + check if xAI key exists
  useEffect(() => {
    loadLatest();

    // Check if xAI key is present (for enabling X research buttons)
    (async () => {
      try {
        const key = await invoke<string | null>('get_setting', { key: 'xai_api_key' });
        setHasXaiKey(!!key);
      } catch {
        setHasXaiKey(false);
      }
    })();
  }, []);

  return (
    <div>
      <h2 className="text-2xl font-semibold mb-2">Research — Musk Companies Only</h2>
      <p className="mb-4 text-sm opacity-70">
        Focused strictly on <strong>Elon Musk's companies</strong> (Tesla, SpaceX, xAI, Neuralink, Boring Company).<br />
        General EV news is excluded.
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

      <div className="flex justify-end mb-2">
        <button 
          className="btn btn-error btn-sm"
          onClick={() => setShowResetConfirm(true)}
          disabled={isResetting}
          title="Permanently delete all research runs and sources"
        >
          {isResetting ? 'Resetting…' : 'Reset All Research Data'}
        </button>
      </div>

      {showResetConfirm && (
        <dialog className="modal modal-open">
          <div className="modal-box">
            <h3 className="font-bold text-lg text-error">Reset all research data?</h3>
            <p className="py-4 text-sm">
              This permanently deletes every research run and all associated sources (RSS + X/Grok)
              from your local database. This cannot be undone.
            </p>
            <div className="modal-action">
              <button
                type="button"
                className="btn"
                onClick={() => setShowResetConfirm(false)}
                disabled={isResetting}
              >
                Cancel
              </button>
              <button
                type="button"
                className="btn btn-error"
                onClick={() => void performResetResearchData()}
                disabled={isResetting}
              >
                {isResetting ? (
                  <>
                    <span className="loading loading-spinner loading-xs" />
                    Deleting…
                  </>
                ) : (
                  'Yes, delete everything'
                )}
              </button>
            </div>
          </div>
          <form method="dialog" className="modal-backdrop">
            <button type="button" onClick={() => setShowResetConfirm(false)}>close</button>
          </form>
        </dialog>
      )}

      {error && <div className="alert alert-error mb-4">{error}</div>}
      {resetSuccess && <div className="alert alert-success mb-4">{resetSuccess}</div>}
      {generateSuccess && <div className="alert alert-success mb-4">{generateSuccess}</div>}

      {activeSubTab === 'current' && (
        <div>
          <div className="flex flex-wrap gap-3 mb-6">
            <button 
              className="btn btn-sm btn-primary" 
              onClick={() => handleRunResearch('rss')}
              disabled={loading}
            >
              Run RSS Only
            </button>

            <button 
              className="btn btn-sm btn-secondary" 
              onClick={() => handleRunResearch('x')}
              disabled={loading || !hasXaiKey}
              title={!hasXaiKey ? "xAI key required for Grok X research" : ""}
            >
              Run X Only (Grok)
            </button>

            <button 
              className="btn btn-sm btn-accent" 
              onClick={() => handleRunResearch('both')}
              disabled={loading || !hasXaiKey}
              title={!hasXaiKey ? "xAI key required for Grok X research" : ""}
            >
              Run Both
            </button>

            <button
              type="button"
              className="btn btn-sm btn-warning"
              onClick={() => void handleGenerateDrafts()}
              disabled={generating || loading || !hasXaiKey || !currentRun}
              title={
                !hasXaiKey
                  ? 'xAI key required'
                  : !currentRun
                    ? 'Run research first'
                    : 'Generate draft posts from latest research (fresh takes)'
              }
            >
              {generating ? 'Generating drafts…' : 'Generate Drafts → Queue'}
            </button>
          </div>

          {loading && (
            <div className="flex flex-col items-center justify-center py-6 mb-4 bg-base-200 rounded-box">
              <span className="loading loading-spinner loading-lg text-primary"></span>
              <p className="mt-3 font-medium">Researching sources…</p>
              <p className="text-xs opacity-60 mt-1">Querying RSS + Grok for high-signal Musk company updates</p>
            </div>
          )}

          {!hasXaiKey && (
            <div className="alert alert-warning alert-sm mb-4">
              X research via Grok is disabled — no xAI API key is saved in Settings.
            </div>
          )}

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

                  {currentRun.sources.some(s => s.source_type === 'x_grok') && (
                    <div className="alert alert-info alert-sm mb-4 text-xs">
                      X items are Grok-suggested based on its knowledge. Always verify links and quotes — the model can make mistakes.
                    </div>
                  )}

                  <div className="space-y-3">
                    {currentRun.sources.map((source, index) => (
                      <div key={index} className="card bg-base-100 shadow-sm">
                        <div className="card-body py-3">
                          <div className="flex justify-between items-start">
                            <div>
                              {source.url ? (
                                <a 
                                  href={source.url} 
                                  target="_blank" 
                                  rel="noopener noreferrer"
                                  className="font-medium hover:underline"
                                >
                                  {source.title}
                                </a>
                              ) : (
                                <span className="font-medium">{source.title}</span>
                              )}
                              <div className="text-xs opacity-60 mt-0.5">
                                {source.source_name} • {source.published_at 
                                  ? new Date(source.published_at).toLocaleDateString() 
                                  : currentRun 
                                    ? new Date(currentRun.run.run_at).toLocaleDateString() 
                                    : 'Unknown date'}
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
        <HistoricalSourcesList
          key={historicalResetKey}
          reloadToken={historicalResetKey}
        />
      )}
    </div>
  );
}

// ============================================
// HistoricalSourcesList - Flat aggregated list of all research sources (paginated + searchable)
// ============================================
function HistoricalSourcesList({ reloadToken }: { reloadToken: number }) {
  const [allSources, setAllSources] = useState<HistoricalResearchSource[]>([]);
  const [searchTerm, setSearchTerm] = useState('');
  const [pageSize, setPageSize] = useState(25);
  const [currentPage, setCurrentPage] = useState(1);

  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Refetch whenever parent bumps reloadToken (e.g. after Reset All Research Data).
  // Clear local list immediately so stale historical rows never linger during the request.
  useEffect(() => {
    setSearchTerm('');
    setCurrentPage(1);
    setAllSources([]);
    setLoading(true);
    setError(null);

    let cancelled = false;

    (async () => {
      try {
        const all = await getAllHistoricalSources();
        if (!cancelled) {
          setAllSources(all);
        }
      } catch (e: unknown) {
        console.error(e);
        if (!cancelled) {
          setError('Failed to load historical research sources.');
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [reloadToken]);

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
                    {source.url ? (
                      <a 
                        href={source.url} 
                        target="_blank" 
                        rel="noopener noreferrer"
                        className="font-medium hover:underline text-sm"
                      >
                        {source.title}
                      </a>
                    ) : (
                      <span className="font-medium text-sm">{source.title}</span>
                    )}
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
