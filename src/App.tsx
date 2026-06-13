import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import ApiKeySettings from './components/ApiKeySettings'
import PostsTab from './components/PostsTab'
import { ResearchTab } from './components/ResearchTab'
import XCredentialsSettings from './components/XCredentialsSettings'
import { SETTING_KEYS } from './lib/constants'

type Tab = 'posts' | 'research' | 'settings'

function App() {
  const [activeTab, setActiveTab] = useState<Tab>('posts')
  const [savedXaiKey, setSavedXaiKey] = useState('')

  useEffect(() => {
    document.documentElement.setAttribute('data-theme', 'synthwave')
  }, [])

  useEffect(() => {
    async function loadXaiKey() {
      const envFallback = import.meta.env.VITE_XAI_API_KEY as string | undefined

      try {
        const stored = await invoke<string | null>('get_setting', {
          key: SETTING_KEYS.xaiApiKey,
        })
        setSavedXaiKey(stored ?? envFallback ?? '')
      } catch (loadError) {
        console.error('Failed to load saved xAI key', loadError)
        setSavedXaiKey(envFallback ?? '')
      }
    }

    void loadXaiKey()
  }, [])

  return (
    <div className="min-h-screen bg-base-200">
      <div className="navbar bg-base-100 border-b border-primary/30 px-4">
        <div className="flex-1">
          <span className="text-2xl font-semibold tracking-tight text-primary">x-poster</span>
          <span className="ml-2 text-xs opacity-70 align-super text-secondary">
            Tesla • TSLA • Elon (non-political)
          </span>
        </div>
        <div className="flex-none">
          <div className="badge badge-primary badge-sm badge-outline mr-3">local • dev</div>
          <button
            type="button"
            className="btn btn-sm btn-primary"
            onClick={() => window.location.reload()}
          >
            Refresh
          </button>
        </div>
      </div>

      <div className="tabs tabs-bordered tabs-lg bg-base-100 border-b border-primary/20 px-4 pt-2">
        <button
          type="button"
          className={`tab ${activeTab === 'posts' ? 'tab-active' : ''}`}
          onClick={() => setActiveTab('posts')}
        >
          Posts
        </button>
        <button
          type="button"
          className={`tab ${activeTab === 'research' ? 'tab-active' : ''}`}
          onClick={() => setActiveTab('research')}
        >
          Research
        </button>
        <button
          type="button"
          className={`tab ${activeTab === 'settings' ? 'tab-active' : ''}`}
          onClick={() => setActiveTab('settings')}
        >
          Settings
        </button>
      </div>

      <div className="p-6 max-w-6xl mx-auto">
        {activeTab === 'posts' && <PostsTab />}

        {activeTab === 'research' && <ResearchTab />}

        {activeTab === 'settings' && (
          <div className="max-w-2xl">
            <h2 className="text-2xl font-semibold mb-4">Settings</h2>

            <div className="card bg-base-100 mb-6">
              <div className="card-body">
                <h3 className="font-semibold mb-2">API Keys</h3>
                <p className="text-sm mb-3 opacity-80">
                  Enter your xAI API key below and click <strong>Save Key</strong>. The key is
                  stored locally in the app (SQLite).
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
      </div>

      <footer className="text-center text-xs opacity-50 py-6">
        x-poster • local only • human approval required (MVP)
      </footer>
    </div>
  )
}

export default App