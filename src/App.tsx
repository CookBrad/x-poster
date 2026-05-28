import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import ApiKeySettings from './components/ApiKeySettings'

type Tab = 'queue' | 'research' | 'settings' | 'history'

function App() {
  const [activeTab, setActiveTab] = useState<Tab>('queue')
  const [testResult, setTestResult] = useState<string>('')

  // Persisted xAI key (loaded from backend or .env fallback)
  const [savedXaiKey, setSavedXaiKey] = useState<string>('')

  // Effective key used for the test connection button
  const effectiveXaiKey = savedXaiKey

  // Load saved key from backend on mount (with .env fallback)
  useEffect(() => {
    async function loadKey() {
      try {
        const stored = await invoke<string | null>('get_setting', { key: 'xai_api_key' })
        if (stored) {
          setSavedXaiKey(stored)
          setXaiKeyInput('') // don't prefill the sensitive value for security
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

  async function testXaiConnection() {
    if (!effectiveXaiKey) {
      setTestResult('Please enter an xAI API key above.')
      return
    }
    setTestResult('Testing xAI connection...')

    try {
      const res = await fetch('https://api.x.ai/v1/chat/completions', {
        method: 'POST',
        headers: {
          'Authorization': `Bearer ${effectiveXaiKey}`,
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          model: 'grok-3', // current stable default (grok-3-mini can be unreliable)
          messages: [
            { role: 'system', content: 'You are a helpful assistant.' },
            { role: 'user', content: 'Say hello from x-poster in one short sentence.' }
          ],
          max_tokens: 50,
        }),
      })

      if (!res.ok) {
        const text = await res.text()
        throw new Error(`HTTP ${res.status}: ${text}`)
      }

      const data = await res.json()
      const reply = data.choices?.[0]?.message?.content || 'No content'
      setTestResult(`✅ Success! xAI replied: "${reply.trim()}"`)
    } catch (err: any) {
      setTestResult(`❌ Error: ${err.message || err}`)
    }
  }

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
          <div>
            <div className="flex items-center justify-between mb-4">
              <h2 className="text-2xl font-semibold">Draft Queue</h2>
              <button className="btn btn-primary btn-sm">Research Now</button>
            </div>

            <div className="alert alert-info mb-6">
              <span>
                This is the early UI shell. Real draft cards with editable text, source citations,
                stock image previews, and approve/post actions are coming in the next steps.
              </span>
            </div>

            {/* Placeholder draft cards */}
            <div className="grid gap-4 md:grid-cols-2">
              {[1, 2].map(i => (
                <div key={i} className="card bg-base-100 shadow draft-card">
                  <div className="card-body">
                    <div className="flex justify-between text-xs opacity-70">
                      <span>Topic: Tesla / TSLA</span>
                      <span>just now</span>
                    </div>
                    <p className="font-medium">
                      Tesla delivered 495k vehicles in Q2, beating expectations. Strong energy storage growth continues...
                    </p>
                    <div className="text-xs opacity-60">Sources: @Tesla, Electrek RSS, X semantic search</div>
                    <div className="card-actions justify-end mt-3 gap-2">
                      <button className="btn btn-ghost btn-sm">Edit</button>
                      <button className="btn btn-success btn-sm">Approve &amp; Post</button>
                      <button className="btn btn-ghost btn-sm">Skip</button>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}

        {activeTab === 'research' && (
          <div>
            <h2 className="text-2xl font-semibold mb-4">Manual Research</h2>
            <p className="mb-4">Trigger on-demand research cycles and see live source fetching (X + RSS).</p>
            <button className="btn btn-primary">Run Research Cycle (mock)</button>
          </div>
        )}

        {activeTab === 'settings' && (
          <div className="max-w-2xl">
            <h2 className="text-2xl font-semibold mb-4">Settings</h2>

            <div className="card bg-base-100 mb-6">
              <div className="card-body">
                <h3 className="font-semibold mb-2">API Keys</h3>
                <p className="text-sm mb-3 opacity-80">
                  Enter your xAI API key below and click <strong>Save Key</strong>. 
                  The key is stored locally in the app. You can still use <code>VITE_XAI_API_KEY</code> in <code>.env</code> as a fallback during development.
                </p>

                <ApiKeySettings 
                  initialSavedKey={savedXaiKey} 
                  onKeySaved={setSavedXaiKey} 
                />

                {testResult && (
                  <div className={`alert alert-sm mt-3 whitespace-pre-wrap text-sm ${testResult.startsWith('✅') ? 'alert-success' : 'alert-error'}`}>
                    {testResult}
                  </div>
                )}
              </div>
            </div>

            <div className="text-xs opacity-70">
              X API credentials will appear here once we implement the client.
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

export default App
