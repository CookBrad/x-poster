import { useEffect, useMemo, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { SETTING_KEYS } from '../lib/constants'
import { errorMessage } from '../lib/errors'
import { countFilledFields } from '../lib/settingsUtils'

const CREDENTIAL_FIELDS = [
  {
    key: SETTING_KEYS.xConsumerKey,
    label: 'API key (consumer key)',
    placeholder: 'From X Developer Portal',
    hint: 'App → Keys and tokens',
  },
  {
    key: SETTING_KEYS.xConsumerSecret,
    label: 'API secret (consumer secret)',
    placeholder: 'Keep private',
    hint: 'Shown once when created',
  },
  {
    key: SETTING_KEYS.xAccessToken,
    label: 'Access token',
    placeholder: 'User context token',
    hint: 'Generate under OAuth 1.0a',
  },
  {
    key: SETTING_KEYS.xAccessTokenSecret,
    label: 'Access token secret',
    placeholder: 'User context secret',
    hint: 'Regenerate after permission changes',
  },
] as const

const CREDENTIAL_KEYS = CREDENTIAL_FIELDS.map((field) => field.key)

interface XCredentialsSettingsProps {
  onCredentialsChanged?: () => void
}

export default function XCredentialsSettings({ onCredentialsChanged }: XCredentialsSettingsProps) {
  const [values, setValues] = useState<Record<string, string>>({})
  const [savedKeys, setSavedKeys] = useState<Set<string>>(new Set())
  const [showSecrets, setShowSecrets] = useState(false)
  const [saving, setSaving] = useState(false)
  const [testing, setTesting] = useState(false)
  const [message, setMessage] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)

  const filledCount = useMemo(
    () => countFilledFields(values, CREDENTIAL_KEYS),
    [values]
  )
  const allFieldsFilled = filledCount === CREDENTIAL_KEYS.length

  useEffect(() => {
    void loadCredentials()
  }, [])

  async function loadCredentials() {
    const loaded: Record<string, string> = {}
    const saved = new Set<string>()

    for (const { key } of CREDENTIAL_FIELDS) {
      try {
        const value = await invoke<string | null>('get_setting', { key })
        if (value) {
          loaded[key] = value
          saved.add(key)
        }
      } catch {
        /* ignore */
      }
    }

    setValues(loaded)
    setSavedKeys(saved)
  }

  async function handleSave() {
    setSaving(true)
    setError(null)
    setMessage(null)

    try {
      const newlySaved = new Set<string>()
      for (const { key } of CREDENTIAL_FIELDS) {
        const value = values[key] ?? ''
        if (value.trim()) {
          await invoke('set_setting', { key, value: value.trim() })
          newlySaved.add(key)
        }
      }

      setSavedKeys(newlySaved)
      setMessage(
        newlySaved.size === CREDENTIAL_KEYS.length
          ? 'All X credentials saved. You can test the connection or approve a post.'
          : `Saved ${newlySaved.size} of ${CREDENTIAL_KEYS.length} fields. Fill in all four to post.`
      )
      onCredentialsChanged?.()
    } catch (saveError: unknown) {
      setError(errorMessage(saveError, 'Failed to save credentials'))
    } finally {
      setSaving(false)
    }
  }

  async function handleTest() {
    if (!allFieldsFilled) {
      setError('Fill in all four credential fields, then save, before testing.')
      return
    }

    setTesting(true)
    setError(null)
    setMessage(null)

    try {
      await handleSave()
      const result = await invoke<string>('test_x_credentials', {})
      setMessage(result)
      onCredentialsChanged?.()
    } catch (testError: unknown) {
      setError(errorMessage(testError, 'X credential test failed'))
    } finally {
      setTesting(false)
    }
  }

  async function handleClear() {
    if (
      !window.confirm(
        'Remove all saved X credentials from this device?\n\nYou will need to re-enter them to post.'
      )
    ) {
      return
    }

    setError(null)
    setMessage(null)

    try {
      for (const { key } of CREDENTIAL_FIELDS) {
        await invoke('delete_setting', { key })
      }
      setValues({})
      setSavedKeys(new Set())
      setMessage('X credentials removed.')
      onCredentialsChanged?.()
    } catch (clearError: unknown) {
      setError(errorMessage(clearError, 'Failed to clear credentials'))
    }
  }

  return (
    <div data-testid="x-credentials-settings">
      <div className="text-sm mb-4 opacity-80" data-testid="x-cred-progress">
        {filledCount} of {CREDENTIAL_KEYS.length} fields filled
        {savedKeys.size > 0 && ` · ${savedKeys.size} saved locally`}
      </div>

      <details className="mb-4 max-w-2xl">
        <summary className="cursor-pointer text-sm font-medium opacity-80 hover:opacity-100">
          X Developer Portal setup tips
        </summary>
        <ul
          className="text-xs opacity-70 mt-2 list-disc list-inside space-y-1 pl-1"
          data-testid="x-cred-setup-hints"
        >
          <li>
            Set app permissions to <strong>Read and write</strong> (not Read only).
          </li>
          <li>
            Enable <strong>OAuth 1.0a</strong> under User authentication setup.
          </li>
          <li>
            After changing permissions, <strong>regenerate</strong> the access token and secret.
          </li>
          <li>Test connection verifies identity; posting also needs write permission.</li>
        </ul>
      </details>

      <div className="grid gap-3 max-w-lg">
        {CREDENTIAL_FIELDS.map(({ key, label, placeholder, hint }) => {
          const isSaved = savedKeys.has(key)
          const hasValue = (values[key] ?? '').trim().length > 0

          return (
            <label key={key} className="form-control">
              <div className="label py-0 min-h-0">
                <span className="label-text text-sm font-medium">{label}</span>
                {isSaved && hasValue && (
                  <span className="label-text-alt text-success text-xs">Saved</span>
                )}
              </div>
              <input
                type={showSecrets ? 'text' : 'password'}
                className="input input-bordered input-sm font-mono"
                placeholder={placeholder}
                value={values[key] ?? ''}
                onChange={(event) =>
                  setValues((previous) => ({ ...previous, [key]: event.target.value }))
                }
                data-testid={`x-cred-${key}`}
              />
              <div className="label py-0 min-h-0">
                <span className="label-text-alt text-xs opacity-60">{hint}</span>
              </div>
            </label>
          )
        })}
      </div>

      <label className="label cursor-pointer justify-start gap-2 max-w-lg mt-2">
        <input
          type="checkbox"
          className="checkbox checkbox-xs"
          checked={showSecrets}
          onChange={(event) => setShowSecrets(event.target.checked)}
        />
        <span className="label-text text-xs">Show credential values</span>
      </label>

      <div className="flex gap-2 mt-4 flex-wrap">
        <button
          type="button"
          className="btn btn-primary btn-sm"
          onClick={() => void handleSave()}
          disabled={saving || filledCount === 0}
          data-testid="x-cred-save"
        >
          {saving ? 'Saving…' : 'Save credentials'}
        </button>
        <button
          type="button"
          className="btn btn-accent btn-sm"
          onClick={() => void handleTest()}
          disabled={testing || !allFieldsFilled}
          title={!allFieldsFilled ? 'Fill in all four fields first' : undefined}
          data-testid="x-cred-test"
        >
          {testing ? 'Testing…' : 'Save & test connection'}
        </button>
        {savedKeys.size > 0 && (
          <button
            type="button"
            className="btn btn-ghost btn-sm text-error"
            onClick={() => void handleClear()}
            data-testid="x-cred-clear"
          >
            Remove saved credentials
          </button>
        )}
      </div>

      {message && (
        <div className="alert alert-success alert-sm mt-3" data-testid="x-cred-success">
          {message}
        </div>
      )}
      {error && (
        <div className="alert alert-error alert-sm mt-3" data-testid="x-cred-error">
          {error}
        </div>
      )}
    </div>
  )
}