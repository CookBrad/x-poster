import { useState } from 'react'
import { invoke } from '@tauri-apps/api/core'

interface ApiKeySettingsProps {
  initialSavedKey?: string
  onKeySaved?: (key: string) => void
  onKeyCleared?: () => void
}

export default function ApiKeySettings({ initialSavedKey = '', onKeySaved, onKeyCleared }: ApiKeySettingsProps) {
  const [savedXaiKey, setSavedXaiKey] = useState(initialSavedKey)
  const [xaiKeyInput, setXaiKeyInput] = useState('')
  const [showXaiKey, setShowXaiKey] = useState(false)
  const [keySaved, setKeySaved] = useState(false)
  const [testResult, setTestResult] = useState('')
  const [error, setError] = useState('')

  const effectiveXaiKey = xaiKeyInput.trim() || savedXaiKey

  async function handleSaveKey() {
    const keyToSave = xaiKeyInput.trim()
    if (!keyToSave) {
      setError('Please enter an API key before saving.')
      return
    }

    setError('')
    setTestResult('')

    try {
      await invoke('set_setting', { key: 'xai_api_key', value: keyToSave })
      setSavedXaiKey(keyToSave)
      setXaiKeyInput('')
      setKeySaved(true)

      if (onKeySaved) {
        onKeySaved(keyToSave)
      }

      setTimeout(() => setKeySaved(false), 3000)
    } catch (e: any) {
      setError(`Failed to save key: ${e}`)
    }
  }

  async function handleTestConnection() {
    if (!effectiveXaiKey) {
      setTestResult('Please enter or select a key first.')
      return
    }

    setTestResult('Testing xAI connection...')
    setError('')

    try {
      const res = await fetch('https://api.x.ai/v1/chat/completions', {
        method: 'POST',
        headers: {
          'Authorization': `Bearer ${effectiveXaiKey}`,
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          model: 'grok-3',
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

  async function handleResetKey() {
    setError('')
    setTestResult('')

    try {
      await invoke('delete_setting', { key: 'xai_api_key' })
      setSavedXaiKey('')
      setXaiKeyInput('')
      setKeySaved(false)

      if (onKeyCleared) {
        onKeyCleared()
      }
    } catch (e: any) {
      setError(`Failed to reset key: ${e}`)
    }
  }

  return (
    <div>
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
            data-testid="xai-key-input"
          />
          <button
            type="button"
            className="btn btn-ghost join-item border border-l-0"
            onClick={() => setShowXaiKey(!showXaiKey)}
            data-testid="toggle-visibility"
          >
            {showXaiKey ? '🙈' : '👁️'}
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

      <div className="flex gap-2 items-center flex-wrap">
        <button
          className="btn btn-primary"
          onClick={handleSaveKey}
          disabled={!xaiKeyInput.trim()}
          data-testid="save-key-button"
        >
          Save Key
        </button>

        <button
          className="btn btn-accent"
          onClick={handleTestConnection}
          disabled={!effectiveXaiKey}
        >
          Test Connection
        </button>

        {savedXaiKey && (
          <button
            className="btn btn-ghost text-error"
            onClick={handleResetKey}
          >
            Reset Key
          </button>
        )}

        {keySaved && (
          <div className="badge badge-success gap-2 whitespace-nowrap" data-testid="saved-badge">
            Saved!
          </div>
        )}
      </div>

      {error && (
        <div className="alert alert-error alert-sm mt-2 text-sm" data-testid="error-message">
          {error}
        </div>
      )}

      {testResult && (
        <div className={`alert alert-sm mt-2 whitespace-pre-wrap text-sm ${testResult.startsWith('✅') ? 'alert-success' : 'alert-error'}`}>
          {testResult}
        </div>
      )}
    </div>
  )
}
