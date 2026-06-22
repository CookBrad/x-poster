import { useCallback, useEffect, useState } from 'react'
import PostsTab from './components/PostsTab'
import { ResearchTab } from './components/ResearchTab'
import { SettingsTab } from './components/SettingsTab'
import { LAST_ACTIVE_TAB_KEY } from './lib/constants'

type Tab = 'posts' | 'research' | 'settings'

type Toast = {
  id: number
  message: string
  kind: 'success' | 'error' | 'info'
}

function App() {
  const [activeTab, setActiveTab] = useState<Tab>(() => {
    try {
      const saved = localStorage.getItem(LAST_ACTIVE_TAB_KEY) as Tab | null
      if (saved === 'posts' || saved === 'research' || saved === 'settings') return saved
    } catch {}
    return 'posts'
  })

  const [refreshToken, setRefreshToken] = useState(0)

  // Shared bump for "Reload data" effect (used by navbar and by generation success
  // so that Posts list auto-refreshes even if user is on Posts tab or switches during gen)
  const bumpRefresh = useCallback(() => {
    setRefreshToken((t) => t + 1)
  }, [])

  // Track in-progress long operations (research, generate, post) across tabs
  const [busyCount, setBusyCount] = useState(0)
  const onBusyChange = useCallback((delta: number) => {
    setBusyCount((c) => Math.max(0, c + delta))
  }, [])

  // Tiny global toast system (visible across tabs, auto-dismiss)
  const [toasts, setToasts] = useState<Toast[]>([])

  const showToast = useCallback((message: string, kind: 'success' | 'error' | 'info' = 'success') => {
    const id = Date.now() + Math.random()
    setToasts((prev) => [...prev, { id, message, kind }])
    setTimeout(() => {
      setToasts((prev) => prev.filter((t) => t.id !== id))
    }, 4500)
  }, [])

  const dismissToast = useCallback((id: number) => {
    setToasts((prev) => prev.filter((t) => t.id !== id))
  }, [])

  // Persist main tab
  useEffect(() => {
    try {
      localStorage.setItem(LAST_ACTIVE_TAB_KEY, activeTab)
    } catch {}
  }, [activeTab])

  useEffect(() => {
    document.documentElement.setAttribute('data-theme', 'synthwave')
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
            onClick={bumpRefresh}
            title="Reload data for the current tab (no full page reload)"
          >
            Reload data
          </button>
          {busyCount > 0 && (
            <span
              className="ml-2 badge badge-secondary badge-sm flex items-center gap-1"
              title="Long-running research, generation or posting operation in progress (you can switch tabs safely)"
            >
              <span className="loading loading-spinner loading-xs" />
              Working…
            </span>
          )}
        </div>
      </div>

      <div
        className="flex gap-4 bg-base-100 border-b border-primary/20 px-4 py-3"
        data-testid="main-tabs"
      >
        <button
          type="button"
          className={`btn btn-sm min-w-[7rem] ${activeTab === 'posts' ? 'btn-primary' : 'btn-outline'}`}
          onClick={() => setActiveTab('posts')}
          data-testid="main-tab-posts"
        >
          Posts
        </button>
        <button
          type="button"
          className={`btn btn-sm min-w-[7rem] ${activeTab === 'research' ? 'btn-primary' : 'btn-outline'}`}
          onClick={() => setActiveTab('research')}
          data-testid="main-tab-research"
        >
          Research
        </button>
        <button
          type="button"
          className={`btn btn-sm min-w-[7rem] ${activeTab === 'settings' ? 'btn-primary' : 'btn-outline'}`}
          onClick={() => setActiveTab('settings')}
          data-testid="main-tab-settings"
        >
          Settings
        </button>
      </div>

      {/* All tab contents are always mounted so long-running research/generate/post
          operations (and their loading/generating states) continue even when the user
          switches tabs. Visibility is controlled via Tailwind hidden/block. */}
      <div className="p-6 max-w-6xl mx-auto">
        <div className={activeTab === 'posts' ? 'block' : 'hidden'}>
          <PostsTab
            refreshToken={refreshToken}
            onShowToast={showToast}
            onBusyChange={onBusyChange}
            onBumpRefresh={bumpRefresh}
          />
        </div>
        <div className={activeTab === 'research' ? 'block' : 'hidden'}>
          <ResearchTab
            refreshToken={refreshToken}
            onShowToast={showToast}
            onBusyChange={onBusyChange}
            onBumpRefresh={bumpRefresh}
          />
        </div>
        <div className={activeTab === 'settings' ? 'block' : 'hidden'}>
          <SettingsTab />
        </div>
      </div>

      {/* Global toast host (fixed, survives tab switches) */}
      <div className="fixed bottom-4 right-4 z-[999] flex flex-col gap-2">
        {toasts.map((toast) => (
          <div
            key={toast.id}
            className={`alert shadow-lg w-80 ${
              toast.kind === 'success'
                ? 'alert-success'
                : toast.kind === 'error'
                ? 'alert-error'
                : 'alert-info'
            }`}
            role="alert"
          >
            <span className="text-sm">{toast.message}</span>
            <button
              type="button"
              className="btn btn-sm btn-ghost -mr-2"
              onClick={() => dismissToast(toast.id)}
              aria-label="Dismiss"
            >
              ✕
            </button>
          </div>
        ))}
      </div>

      <footer className="text-center text-xs opacity-50 py-6">
        x-poster • local only • human approval required (MVP)
      </footer>
    </div>
  )
}

export default App