import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { DEFAULT_GROK_MODEL, SETTING_KEYS } from '../lib/constants'
import { errorMessage } from '../lib/errors'
import { maskSecret } from '../lib/settingsUtils'

interface ApiKeySettingsProps {
  initialSavedKey?: string
  onKeySaved?: (key: string) => void
  onKeyCleared?: () => void
}

export default function ApiKeySettings({
  initialSavedKey = '',
  onKeySaved,
  onKeyCleared,
}: ApiKeySettingsProps) {
  const [savedXaiKey, setSavedXaiKey] = useState(initialSavedKey)
  const [xaiKeyInput, setXaiKeyInput] = useState('')
  const [showXaiKey, setShowXaiKey] = useState(false)
  const [keySaved, setKeySaved] = useState(false)
  const [testResult, setTestResult] = useState('')
  const [error, setError] = useState('')
  const [testing, setTesting] = useState(false)
  const [grokModel, setGrokModel] = useState(DEFAULT_GROK_MODEL)

  useEffect(() => {
    setSavedXaiKey(initialSavedKey)
  }, [initialSavedKey])

  useEffect(() => {
    void (async () => {
      try {
        const saved = await invoke<string | null>('get_setting', { key: SETTING_KEYS.grokModel })
        if (saved) {
          setGrokModel(saved)
        }
      } catch {
        /* ignore */
      }
    })()
  }, [])

  const effectiveSavedKey = savedXaiKey || initialSavedKey
  const effectiveXaiKey = xaiKeyInput.trim() || effectiveSavedKey
  const hasSavedKey = effectiveSavedKey.trim().length > 0

  async function handleSaveKey() {
    const keyToSave = xaiKeyInput.trim()
    if (!keyToSave) {
      setError('Please enter an API key before saving.')
      return
    }

    setError('')
    setTestResult('')

    try {
      await invoke('set_setting', { key: SETTING_KEYS.xaiApiKey, value: keyToSave })
      setSavedXaiKey(keyToSave)
      setXaiKeyInput('')
      setKeySaved(true)
      onKeySaved?.(keyToSave)
      setTimeout(() => setKeySaved(false), 3000)
    } catch (saveError: unknown) {
      setError(`Failed to save key: ${errorMessage(saveError)}`)
    }
  }

  async function handleTestConnection() {
    if (!effectiveXaiKey) {
      setTestResult('Enter a key above or save one first, then test.')
      return
    }

    setTesting(true)
    setTestResult('Testing xAI connection…')
    setError('')

    let modelToUse = grokModel
    try {
      const savedModel = await invoke<string | null>('get_setting', {
        key: SETTING_KEYS.grokModel,
      })
      if (savedModel) {
        modelToUse = savedModel
      }
    } catch {
      /* use local state */
    }

    try {
      const response = await fetch('https://api.x.ai/v1/chat/completions', {
        method: 'POST',
        headers: {
          Authorization: `Bearer ${effectiveXaiKey}`,
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          model: modelToUse,
          messages: [
            { role: 'system', content: 'You are a helpful assistant.' },
            { role: 'user', content: 'Say hello from x-poster in one short sentence.' },
          ],
          max_tokens: 50,
        }),
      })

      if (!response.ok) {
        const text = await response.text()
        throw new Error(`HTTP ${response.status}: ${text}`)
      }

      const data = await response.json()
      const reply = data.choices?.[0]?.message?.content || 'No content'
      setTestResult(`Success (${modelToUse}): "${reply.trim()}"`)
    } catch (testError: unknown) {
      setTestResult(`Connection failed: ${errorMessage(testError)}`)
    } finally {
      setTesting(false)
    }
  }

  async function handleResetKey() {
    if (!window.confirm('Remove the saved xAI API key from this device?')) {
      return
    }

    setError('')
    setTestResult('')

    try {
      await invoke('delete_setting', { key: SETTING_KEYS.xaiApiKey })
      setSavedXaiKey('')
      setXaiKeyInput('')
      setKeySaved(false)
      onKeyCleared?.()
    } catch (resetError: unknown) {
      setError(`Failed to remove key: ${errorMessage(resetError)}`)
    }
  }

  return (
    <div data-testid="xai-settings">
      {hasSavedKey && (
        <div className="alert alert-success alert-sm mb-4 py-2" data-testid="xai-key-saved-status">
          <span>
            Key saved: <span className="font-mono">{maskSecret(effectiveSavedKey)}</span>
          </span>
        </div>
      )}

      <label className="form-control w-full max-w-lg mb-4">
        <div className="label">
          <span className="label-text font-medium">xAI API key</span>
        </div>

        <div className="join w-full">
          <input
            type={showXaiKey ? 'text' : 'password'}
            className="input input-bordered join-item flex-1 font-mono text-sm"
            placeholder={hasSavedKey ? 'Enter a new key to replace the saved one' : 'sk-…'}
            value={xaiKeyInput}
            onChange={(event) => setXaiKeyInput(event.target.value)}
            data-testid="xai-key-input"
          />
          <button
            type="button"
            className="btn btn-ghost join-item border border-l-0"
            onClick={() => setShowXaiKey(!showXaiKey)}
            aria-label={showXaiKey ? 'Hide API key' : 'Show API key'}
            data-testid="toggle-visibility"
          >
            {showXaiKey ? 'Hide' : 'Show'}
          </button>
        </div>

        <div className="label">
          <span className="label-text-alt opacity-60">
            {hasSavedKey
              ? 'A key is saved. Enter a new value only if you want to replace it.'
              : 'Required before you can run research or generate drafts.'}
          </span>
        </div>
      </label>

      <label className="form-control w-full max-w-lg mb-4">
        <div className="label">
          <span className="label-text font-medium">Grok model</span>
        </div>
        <select
          className="select select-bordered w-full max-w-sm"
          value={grokModel}
          onChange={async (event) => {
            const newModel = event.target.value
            setGrokModel(newModel)
            try {
              await invoke('set_setting', { key: SETTING_KEYS.grokModel, value: newModel })
            } catch (modelError) {
              console.error('Failed to save model', modelError)
            }
          }}
          data-testid="grok-model-select"
        >
          <option value="grok-4.3">grok-4.3 — best quality (recommended)</option>
          <option value="grok-3">grok-3 — capable</option>
          <option value="grok-3-mini">grok-3-mini — fast & economical</option>
        </select>
        <div className="label">
          <span className="label-text-alt opacity-60">
            Used for X research via Grok and draft generation.
          </span>
        </div>
      </label>

      <div className="flex gap-2 items-center flex-wrap">
        <button
          type="button"
          className="btn btn-primary btn-sm"
          onClick={() => void handleSaveKey()}
          disabled={!xaiKeyInput.trim()}
          data-testid="save-key-button"
        >
          Save key
        </button>

        <button
          type="button"
          className="btn btn-accent btn-sm"
          onClick={() => void handleTestConnection()}
          disabled={!effectiveXaiKey || testing}
          data-testid="test-xai-connection"
        >
          {testing ? 'Testing…' : 'Test connection'}
        </button>

        {hasSavedKey && (
          <button
            type="button"
            className="btn btn-ghost btn-sm text-error"
            onClick={() => void handleResetKey()}
            data-testid="reset-xai-key"
          >
            Remove saved key
          </button>
        )}

        {keySaved && (
          <div className="badge badge-success whitespace-nowrap" data-testid="saved-badge">
            Saved
          </div>
        )}
      </div>

      {error && (
        <div className="alert alert-error alert-sm mt-3 text-sm" data-testid="error-message">
          {error}
        </div>
      )}

      {testResult && (
        <div
          className={`alert alert-sm mt-3 text-sm ${
            testResult.startsWith('Success') ? 'alert-success' : 'alert-warning'
          }`}
          data-testid="xai-test-result"
        >
          {testResult}
        </div>
      )}
    </div>
  )
}