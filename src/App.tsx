import { useState, useEffect, useCallback } from 'react'
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
  getResearchRuns,
  getResearchRun,
  type Draft,
  type ResearchRunWithSources,
  type ResearchRun
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
  const [historicalRuns, setHistoricalRuns] = useState<ResearchRun[]>([]);
  const [selectedHistorical, setSelectedHistorical] = useState<ResearchRunWithSources | null>(null);

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

  const loadHistoricalList = async () => {
    try {
      const runs = await getResearchRuns();
      setHistoricalRuns(runs);
    } catch (e: any) {
      console.error(e);
    }
  };

  const loadHistoricalRun = async (runId: string) => {
    setLoading(true);
    try {
      const run = await getResearchRun(runId);
      setSelectedHistorical(run);
    } catch (e: any) {
      setError('Failed to load historical research run.');
    } finally {
      setLoading(false);
    }
  };

  const runResearch = async () => {
    setLoading(true);
    setError(null);

    try {
      const newRun = await runResearch();
      setCurrentRun(newRun);
      setActiveSubTab('current');
      await loadHistoricalList(); // refresh history list
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
    loadHistoricalList();
  }, []);

  return (
    <div>
      <h2 className="text-2xl font-semibold mb-2">Research</h2>
      <p className="mb-4 text-sm opacity-70">
        Uses Grok to discover high-signal Tesla/Elon posts on X + pulls from key RSS feeds.
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
            onClick={runResearch}
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
              <p className="text-xs opacity-60 mb-4">{currentRun.sources.length} sources</p>

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
              No research run yet. Click "Run Research Now" to start.
            </div>
          )}
        </div>
      )}

      {activeSubTab === 'historical' && (
        <div>
          {historicalRuns.length === 0 ? (
            <div className="alert alert-info">No historical research runs yet.</div>
          ) : (
            <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
              {/* List of runs */}
              <div>
                <h3 className="font-semibold mb-2">Past Runs</h3>
                <div className="space-y-2">
                  {historicalRuns.map((run) => (
                    <button
                      key={run.id}
                      onClick={() => loadHistoricalRun(run.id)}
                      className={`w-full text-left p-3 rounded border hover:bg-base-200 ${selectedHistorical?.run.id === run.id ? 'border-primary bg-base-200' : ''}`}
                    >
                      <div className="font-medium">{new Date(run.run_at).toLocaleString()}</div>
                      <div className="text-xs opacity-60">{run.source}</div>
                    </button>
                  ))}
                </div>
              </div>

              {/* Selected historical run */}
              <div>
                {selectedHistorical ? (
                  <div>
                    <h3 className="font-semibold mb-2">
                      Run from {new Date(selectedHistorical.run.run_at).toLocaleString()}
                    </h3>
                    <div className="space-y-3">
                      {selectedHistorical.sources.map((source, index) => (
                        <div key={index} className="card bg-base-100 shadow-sm">
                          <div className="card-body py-3">
                            <a 
                              href={source.url} 
                              target="_blank" 
                              rel="noopener noreferrer"
                              className="font-medium hover:underline text-sm"
                            >
                              {source.title}
                            </a>
                            <p className="text-xs opacity-70 mt-1 line-clamp-2">{source.content}</p>
                          </div>
                        </div>
                      ))}
                    </div>
                  </div>
                ) : (
                  <div className="text-sm opacity-70">Select a past run on the left to view its sources.</div>
                )}
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

export default App
