import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import XCredentialsSettings from './XCredentialsSettings'
import { invoke } from '@tauri-apps/api/core'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

const mockInvoke = vi.mocked(invoke)

describe('XCredentialsSettings', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'get_setting') return null
      if (cmd === 'set_setting') return undefined
      if (cmd === 'has_x_credentials') return false
      if (cmd === 'test_x_credentials') return 'Connected as @testuser'
      if (cmd === 'connect_x_oauth') return 'Connected as @testuser'
      return null
    })
  })

  it('renders OAuth client fields and redirect URI', () => {
    render(<XCredentialsSettings />)
    expect(screen.getByTestId('x-credentials-settings')).toBeInTheDocument()
    expect(screen.getByTestId('x-cred-x_oauth_client_id')).toBeInTheDocument()
    expect(screen.getByTestId('x-oauth-redirect-uri')).toHaveTextContent(/127\.0\.0\.1:14555/)
  })

  it('connects via OAuth flow', async () => {
    render(<XCredentialsSettings />)

    fireEvent.change(screen.getByTestId('x-cred-x_oauth_client_id'), {
      target: { value: 'client123' },
    })
    fireEvent.click(screen.getByTestId('x-cred-connect'))

    await waitFor(() => {
      expect(screen.getByTestId('x-cred-success')).toHaveTextContent(/Connected as @testuser/i)
    })
    expect(mockInvoke).toHaveBeenCalledWith('connect_x_oauth', {})
  })
})