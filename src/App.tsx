import { useEffect, useState } from 'react'
import PostsTab from './components/PostsTab'
import { ResearchTab } from './components/ResearchTab'
import { SettingsTab } from './components/SettingsTab'

type Tab = 'posts' | 'research' | 'settings'

function App() {
  const [activeTab, setActiveTab] = useState<Tab>('posts')

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
            onClick={() => window.location.reload()}
          >
            Refresh
          </button>
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

      <div className="p-6 max-w-6xl mx-auto">
        {activeTab === 'posts' && <PostsTab />}
        {activeTab === 'research' && <ResearchTab />}
        {activeTab === 'settings' && <SettingsTab />}
      </div>

      <footer className="text-center text-xs opacity-50 py-6">
        x-poster • local only • human approval required (MVP)
      </footer>
    </div>
  )
}

export default App