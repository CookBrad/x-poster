import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { SETTING_KEYS } from '../lib/constants'
import { errorMessage } from '../lib/errors'

const CREDENTIAL_FIELDS = [
  {
    key: SETTING_KEYS.xConsumerKey,
    label: 'API Key (Consumer Key)',
    placeholder: 'From X Developer Portal',
  },
  {
    key: SETTING_KEYS.xConsumerSecret,
    label: 'API Secret (Consumer Secret)',
    placeholder: 'Keep private',
  },
  { key: SETTING_KEYS.xAccessToken, label: 'Access Token', placeholder: 'User context token' },
  {
    key: SETTING_KEYS.xAccessTokenSecret,
    label: 'Access Token Secret',
    placeholder: 'User context secret',
  },
] as const

export default function XCredentialsSettings() {
  const [values, setValues] = useState<Record<string, string>>({})
  const [showSecrets, setShowSecrets] = useState(false)
  const [saving, setSaving] = useState(false)
  const [testing, setTesting] = useState(false)
  const [message, setMessage] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    void (async () => {
      const loaded: Record<string, string> = {}
      for (const { key } of CREDENTIAL_FIELDS) {
        try {
          const v = await invoke<string | null>('get_setting', { key })
          if (v) loaded[key] = v
        } catch {
          /* ignore */
        }
      }
      setValues(loaded)
    })()
  }, [])

  const handleSave = async () => {
    setSaving(true)
    setError(null)
    setMessage(null)
    try {
      for (const { key } of CREDENTIAL_FIELDS) {
        const value = values[key] ?? ''
        if (value.trim()) {
          await invoke('set_setting', { key, value: value.trim() })
        }
      }
      setMessage('X credentials saved.')
    } catch (saveError: unknown) {
      setError(errorMessage(saveError, 'Failed to save'))
    } finally {
      setSaving(false)
    }
  }

  const handleTest = async () => {
    setTesting(true)
    setError(null)
    setMessage(null)
    try {
      await handleSave()
      const result = await invoke<string>('test_x_credentials', {})
      setMessage(result)
    } catch (testError: unknown) {
      setError(errorMessage(testError, 'X credential test failed'))
    } finally {
      setTesting(false)
    }
  }

  return (
    <div className="mt-6" data-testid="x-credentials-settings">
      <h4 className="font-semibold mb-2">X (Twitter) posting credentials</h4>
      <p className="text-sm opacity-70 mb-3">
        OAuth 1.0a user context keys for posting approved drafts. Research uses Grok only — these are for posting.
      </p>
      <ul className="text-xs opacity-70 mb-3 list-disc list-inside max-w-lg space-y-1" data-testid="x-cred-setup-hints">
        <li>In the X Developer Portal, set app permissions to <strong>Read and write</strong> (not Read only).</li>
        <li>Enable <strong>OAuth 1.0a</strong> under User authentication setup.</li>
        <li>After changing permissions, <strong>regenerate</strong> Access Token and Secret — old tokens keep the old scope.</li>
        <li>Test Connection only checks identity; posting needs write permission on the token.</li>
      </ul>

      {CREDENTIAL_FIELDS.map(({ key, label, placeholder }) => (
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

      <div className="flex gap-2 mt-3 flex-wrap">
        <button
          type="button"
          className="btn btn-primary btn-sm"
          onClick={() => void handleSave()}
          disabled={saving}
          data-testid="x-cred-save"
        >
          Save X Credentials
        </button>
        <button
          type="button"
          className="btn btn-accent btn-sm"
          onClick={() => void handleTest()}
          disabled={testing}
          data-testid="x-cred-test"
        >
          {testing ? 'Testing…' : 'Test X Connection'}
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