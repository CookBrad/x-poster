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
      if (cmd === 'test_x_credentials') return 'Connected as @testuser'
      return null
    })
  })

  it('renders credential fields', () => {
    render(<XCredentialsSettings />)
    expect(screen.getByTestId('x-credentials-settings')).toBeInTheDocument()
    expect(screen.getByTestId('x-cred-x_consumer_key')).toBeInTheDocument()
  })

  it('saves and tests credentials when all fields are filled', async () => {
    render(<XCredentialsSettings />)

    fireEvent.change(screen.getByTestId('x-cred-x_consumer_key'), {
      target: { value: 'key123' },
    })
    fireEvent.change(screen.getByTestId('x-cred-x_consumer_secret'), {
      target: { value: 'secret123' },
    })
    fireEvent.change(screen.getByTestId('x-cred-x_access_token'), {
      target: { value: 'token123' },
    })
    fireEvent.change(screen.getByTestId('x-cred-x_access_token_secret'), {
      target: { value: 'tokensecret123' },
    })
    fireEvent.click(screen.getByTestId('x-cred-test'))

    await waitFor(() => {
      expect(screen.getByTestId('x-cred-success')).toHaveTextContent(/Connected as @testuser/i)
    })
  })
})