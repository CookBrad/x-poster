import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'

type Tab = 'queue' | 'research' | 'settings' | 'history'

function App() {
  const [activeTab, setActiveTab] = useState<Tab>('queue')
  const [testResult, setTestResult] = useState<string>('')

  // Persisted xAI key (loaded from backend or .env fallback)
  const [savedXaiKey, setSavedXaiKey] = useState<string>('')
  // What the user is currently typing in the input
  const [xaiKeyInput, setXaiKeyInput] = useState<string>('')

  // UI state for the API key field
  const [showXaiKey, setShowXaiKey] = useState(false)
  const [keySaved, setKeySaved] = useState(false)

  // Effective key used for testing (prefer what user typed, otherwise saved)
  const effectiveXaiKey = xaiKeyInput.trim() || savedXaiKey

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

  async function saveXaiKey() {
    const keyToSave = xaiKeyInput.trim()
    if (!keyToSave) {
      setTestResult('Please enter an API key before saving.')
      return
    }
    try {
      await invoke('set_setting', { key: 'xai_api_key', value: keyToSave })
      setSavedXaiKey(keyToSave)
      setXaiKeyInput('') // clear input after save for security
      setKeySaved(true)
      setTestResult('')

      // Auto-hide the success indicator after 3 seconds
      setTimeout(() => setKeySaved(false), 3000)
    } catch (e: any) {
      setTestResult(`❌ Failed to save key: ${e}`)
    }
  }

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

                {/* xAI API Key Input with Show/Hide + better feedback */}
                <label className="form-control w-full max-w-md mb-2">
                  <div className="label">
                    <span className="label-text">xAI API Key</span>
                  </div>

                  <div className="join w-full">
                    <input
                      type={showXaiKey ? 'text' : 'password'}
                      className="input input-bordered join-item flex-1 font-mono text-sm"
                      placeholder="sk-..."
                      value={xaiKeyInput}
                      onChange={(e) => setXaiKeyInput(e.target.value)}
                    />
                    <button
                      type="button"
                      className="btn btn-ghost join-item border border-l-0"
                      onClick={() => setShowXaiKey(!showXaiKey)}
                      title={showXaiKey ? 'Hide key' : 'Show key'}
                    >
                      {showXaiKey ? (
                        // Eye slash icon
                        <svg xmlns="http://www.w3.org/2000/svg" className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13.875 18.825A10.05 10.05 0 0112 19c-4.478 0-8.268-2.943-9.543-7a9.97 9.97 0 011.563-3.029m5.858.908a3 3 0 114.243 4.243M9.878 9.878l4.242 4.242M9.88 9.88l-3.29-3.29m7.532 7.532l3.29 3.29M3 3l3.59 3.59m0 0A9.953 9.953 0 0112 5c4.478 0 8.268 2.943 9.543 7a10.025 10.025 0 01-4.132 5.411m0 0L21 21" />
                        </svg>
                      ) : (
                        // Eye icon
                        <svg xmlns="http://www.w3.org/2000/svg" className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" />
                        </svg>
                      )}
                    </button>
                  </div>

                  <div className="label">
                    <span className="label-text-alt opacity-60">
                      {savedXaiKey
                        ? 'A key is currently saved. Enter a new one above to replace it.'
                        : 'No key saved yet.'}
                    </span>
                  </div>
                </label>

                {/* Action buttons + save feedback */}
                <div className="flex gap-2 items-center">
                  <button
                    className="btn btn-primary flex-1"
                    onClick={saveXaiKey}
                    disabled={!xaiKeyInput.trim()}
                  >
                    Save Key
                  </button>
                  <button
                    className="btn btn-accent flex-1"
                    onClick={testXaiConnection}
                    disabled={!effectiveXaiKey}
                  >
                    Test xAI Connection
                  </button>

                  {/* Improved save feedback */}
                  {keySaved && (
                    <div className="badge badge-success gap-2 whitespace-nowrap">
                      Saved!
                    </div>
                  )}
                </div>

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
