import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'

const CLIENT_KEYS = [
  { key: 'x_oauth_client_id', label: 'OAuth 2.0 Client ID', placeholder: 'From X Developer Portal' },
  { key: 'x_oauth_client_secret', label: 'OAuth 2.0 Client Secret', placeholder: 'Keep private' },
] as const

const REDIRECT_URI = 'http://127.0.0.1:14555/callback'

export default function XCredentialsSettings() {
  const [values, setValues] = useState<Record<string, string>>({})
  const [showSecrets, setShowSecrets] = useState(false)
  const [connected, setConnected] = useState(false)
  const [saving, setSaving] = useState(false)
  const [connecting, setConnecting] = useState(false)
  const [testing, setTesting] = useState(false)
  const [message, setMessage] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)

  const refreshConnectionStatus = async () => {
    try {
      const ok = await invoke<boolean>('has_x_credentials', {})
      setConnected(ok)
    } catch {
      setConnected(false)
    }
  }

  useEffect(() => {
    void (async () => {
      const loaded: Record<string, string> = {}
      for (const { key } of CLIENT_KEYS) {
        try {
          const v = await invoke<string | null>('get_setting', { key })
          if (v) loaded[key] = v
        } catch {
          /* ignore */
        }
      }
      setValues(loaded)
      await refreshConnectionStatus()
    })()
  }, [])

  const handleSave = async () => {
    setSaving(true)
    setError(null)
    setMessage(null)
    try {
      for (const { key } of CLIENT_KEYS) {
        const value = values[key] ?? ''
        if (value.trim()) {
          await invoke('set_setting', { key, value: value.trim() })
        }
      }
      setMessage('X OAuth app credentials saved.')
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : 'Failed to save')
    } finally {
      setSaving(false)
    }
  }

  const handleConnect = async () => {
    setConnecting(true)
    setError(null)
    setMessage(null)
    try {
      await handleSave()
      const result = await invoke<string>('connect_x_oauth', {})
      setMessage(result)
      setConnected(true)
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : 'X authorization failed')
      await refreshConnectionStatus()
    } finally {
      setConnecting(false)
    }
  }

  const handleDisconnect = async () => {
    setError(null)
    setMessage(null)
    try {
      await invoke('disconnect_x_oauth', {})
      setConnected(false)
      setMessage('Disconnected from X.')
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : 'Failed to disconnect')
    }
  }

  const handleTest = async () => {
    setTesting(true)
    setError(null)
    setMessage(null)
    try {
      const result = await invoke<string>('test_x_credentials', {})
      setMessage(result)
      setConnected(true)
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : 'X credential test failed')
      await refreshConnectionStatus()
    } finally {
      setTesting(false)
    }
  }

  return (
    <div className="mt-6" data-testid="x-credentials-settings">
      <h4 className="font-semibold mb-2">X (Twitter) posting — OAuth 2.0</h4>
      <p className="text-sm opacity-70 mb-3">
        Connect your X account with OAuth 2.0 (PKCE) to post approved drafts. Research uses Grok only.
        In the X Developer Portal, enable OAuth 2.0, set type to Web App or Native App, and add this callback URL:
      </p>
      <p className="text-xs font-mono bg-base-200 rounded px-2 py-1 mb-3 break-all" data-testid="x-oauth-redirect-uri">
        {REDIRECT_URI}
      </p>

      {CLIENT_KEYS.map(({ key, label, placeholder }) => (
        <label key={key} className="form-control w-full max-w-md mb-2">
          <span className="label-text text-xs">{label}</span>
          <input
            type={showSecrets ? 'text' : 'password'}
            className="input input-bordered input-sm font-mono"
            placeholder={placeholder}
            value={values[key] ?? ''}
            onChange={(e) => setValues((prev) => ({ ...prev, [key]: e.target.value }))}
            data-testid={`x-cred-${key}`}
          />
        </label>
      ))}

      <label className="label cursor-pointer justify-start gap-2 max-w-md">
        <input
          type="checkbox"
          className="checkbox checkbox-xs"
          checked={showSecrets}
          onChange={(e) => setShowSecrets(e.target.checked)}
        />
        <span className="label-text text-xs">Show secrets</span>
      </label>

      <p className="text-sm mt-2" data-testid="x-connection-status">
        Status:{' '}
        <span className={connected ? 'text-success font-medium' : 'opacity-70'}>
          {connected ? 'Connected' : 'Not connected'}
        </span>
      </p>

      <div className="flex gap-2 mt-3 flex-wrap">
        <button
          type="button"
          className="btn btn-primary btn-sm"
          onClick={() => void handleSave()}
          disabled={saving || connecting}
          data-testid="x-cred-save"
        >
          Save Client Credentials
        </button>
        <button
          type="button"
          className="btn btn-accent btn-sm"
          onClick={() => void handleConnect()}
          disabled={connecting || saving}
          data-testid="x-cred-connect"
        >
          {connecting ? 'Waiting for authorization…' : 'Connect with X'}
        </button>
        <button
          type="button"
          className="btn btn-outline btn-sm"
          onClick={() => void handleTest()}
          disabled={testing || !connected}
          data-testid="x-cred-test"
        >
          {testing ? 'Testing…' : 'Test Connection'}
        </button>
        <button
          type="button"
          className="btn btn-ghost btn-sm"
          onClick={() => void handleDisconnect()}
          disabled={!connected}
          data-testid="x-cred-disconnect"
        >
          Disconnect
        </button>
      </div>

      {message && (
        <div className="alert alert-success alert-sm mt-2" data-testid="x-cred-success">
          {message}
        </div>
      )}
      {error && (
        <div className="alert alert-error alert-sm mt-2" data-testid="x-cred-error">
          {error}
        </div>
      )}
    </div>
  )
}