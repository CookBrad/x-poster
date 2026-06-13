import { useCallback, useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { SETTING_KEYS } from '../lib/constants'
import { hasXCredentials } from '../lib/db'
import ApiKeySettings from './ApiKeySettings'
import XCredentialsSettings from './XCredentialsSettings'

interface SetupStatus {
  xaiKeyConfigured: boolean
  xPostingConfigured: boolean
}

export function SettingsTab() {
  const [savedXaiKey, setSavedXaiKey] = useState('')
  const [status, setStatus] = useState<SetupStatus>({
    xaiKeyConfigured: false,
    xPostingConfigured: false,
  })
  const [loadingStatus, setLoadingStatus] = useState(true)

  const refreshStatus = useCallback(async () => {
    setLoadingStatus(true)
    try {
      const envFallback = import.meta.env.VITE_XAI_API_KEY as string | undefined
      const storedKey = await invoke<string | null>('get_setting', {
        key: SETTING_KEYS.xaiApiKey,
      })
      const effectiveKey = storedKey ?? envFallback ?? ''
      setSavedXaiKey(effectiveKey)

      const xPostingConfigured = await hasXCredentials()
      setStatus({
        xaiKeyConfigured: effectiveKey.trim().length > 0,
        xPostingConfigured,
      })
    } catch (loadError) {
      console.error('Failed to load settings status', loadError)
    } finally {
      setLoadingStatus(false)
    }
  }, [])

  useEffect(() => {
    void refreshStatus()
  }, [refreshStatus])

  const handleXaiKeySaved = (key: string) => {
    setSavedXaiKey(key)
    setStatus((previous) => ({ ...previous, xaiKeyConfigured: true }))
  }

  const handleXaiKeyCleared = () => {
    setSavedXaiKey('')
    setStatus((previous) => ({ ...previous, xaiKeyConfigured: false }))
  }

  const handleXCredentialsChanged = () => {
    void refreshStatus()
  }

  return (
    <div className="max-w-3xl space-y-6" data-testid="settings-tab">
      <div>
        <h2 className="text-2xl font-semibold mb-2">Settings</h2>
        <p className="text-sm opacity-80 max-w-2xl">
          Configure the two things x-poster needs: an <strong>xAI key</strong> for research and draft
          generation, and <strong>X credentials</strong> for posting approved drafts. Everything is
          stored locally on your machine.
        </p>
      </div>

      <div className="card bg-base-100 shadow-sm" data-testid="setup-status-card">
        <div className="card-body py-4">
          <h3 className="font-semibold text-sm uppercase tracking-wide opacity-70">
            Setup status
          </h3>
          {loadingStatus ? (
            <span className="loading loading-spinner loading-sm mt-2" />
          ) : (
            <div className="flex flex-col sm:flex-row gap-3 mt-2">
              <SetupStatusBadge
                label="Research & drafts"
                ready={status.xaiKeyConfigured}
                readyText="xAI key saved"
                missingText="Add xAI API key below"
              />
              <SetupStatusBadge
                label="Posting to X"
                ready={status.xPostingConfigured}
                readyText="X credentials saved"
                missingText="Add X credentials below"
              />
            </div>
          )}
        </div>
      </div>

      <div className="card bg-base-100 shadow-sm">
        <div className="card-body">
          <div className="flex items-start gap-3 mb-4">
            <span className="badge badge-primary badge-lg">1</span>
            <div>
              <h3 className="text-lg font-semibold">Research & draft generation</h3>
              <p className="text-sm opacity-70 mt-1">
                Powers Grok research on the Research tab and generating draft posts. Get a key from{' '}
                <a
                  href="https://console.x.ai/"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="link link-primary"
                >
                  console.x.ai
                </a>
                .
              </p>
            </div>
          </div>
          <ApiKeySettings
            initialSavedKey={savedXaiKey}
            onKeySaved={handleXaiKeySaved}
            onKeyCleared={handleXaiKeyCleared}
          />
        </div>
      </div>

      <div className="card bg-base-100 shadow-sm">
        <div className="card-body">
          <div className="flex items-start gap-3 mb-2">
            <span className="badge badge-secondary badge-lg">2</span>
            <div>
              <h3 className="text-lg font-semibold">Posting to X</h3>
              <p className="text-sm opacity-70 mt-1">
                OAuth 1.0a credentials for publishing approved drafts. Create an app at{' '}
                <a
                  href="https://developer.x.com/"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="link link-primary"
                >
                  developer.x.com
                </a>
                .
              </p>
            </div>
          </div>
          <XCredentialsSettings onCredentialsChanged={handleXCredentialsChanged} />
        </div>
      </div>
    </div>
  )
}

function SetupStatusBadge({
  label,
  ready,
  readyText,
  missingText,
}: {
  label: string
  ready: boolean
  readyText: string
  missingText: string
}) {
  return (
    <div
      className={`flex-1 rounded-lg border px-4 py-3 ${
        ready ? 'border-success/40 bg-success/10' : 'border-warning/40 bg-warning/10'
      }`}
    >
      <div className="text-xs font-medium uppercase tracking-wide opacity-70">{label}</div>
      <div className={`text-sm font-medium mt-1 ${ready ? 'text-success' : 'text-warning'}`}>
        {ready ? `✓ ${readyText}` : `○ ${missingText}`}
      </div>
    </div>
  )
}