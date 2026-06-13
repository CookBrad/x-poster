import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import { SettingsTab } from './SettingsTab'
import { invoke } from '@tauri-apps/api/core'
import { hasXCredentials } from '../lib/db'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

vi.mock('../lib/db', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../lib/db')>()
  return {
    ...actual,
    hasXCredentials: vi.fn(),
  }
})

const mockInvoke = vi.mocked(invoke)
const mockHasXCredentials = vi.mocked(hasXCredentials)

describe('SettingsTab', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockInvoke.mockImplementation(async (cmd: string, args?: { key?: string }) => {
      if (cmd === 'get_setting' && args?.key === 'xai_api_key') return 'sk-saved-key'
      if (cmd === 'get_setting') return null
      return undefined
    })
    mockHasXCredentials.mockResolvedValue(false)
  })

  it('shows setup status and both configuration sections', async () => {
    render(<SettingsTab />)

    await waitFor(() => {
      expect(screen.getByTestId('setup-status-card')).toBeInTheDocument()
    })

    expect(
      screen.getByRole('heading', { name: 'Research & draft generation' })
    ).toBeInTheDocument()
    expect(screen.getByRole('heading', { name: 'Posting to X' })).toBeInTheDocument()
    expect(screen.getByTestId('xai-settings')).toBeInTheDocument()
    expect(screen.getByTestId('x-credentials-settings')).toBeInTheDocument()
  })
})