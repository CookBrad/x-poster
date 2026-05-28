import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import ApiKeySettings from './ApiKeySettings'
import { invoke } from '@tauri-apps/api/core'

// Mock the Tauri invoke
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

describe('ApiKeySettings', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders the input and save button', () => {
    render(<ApiKeySettings />)

    expect(screen.getByTestId('xai-key-input')).toBeInTheDocument()
    expect(screen.getByTestId('save-key-button')).toBeInTheDocument()
    expect(screen.getByText(/No key saved yet/i)).toBeInTheDocument()
  })

  it('disables save button when input is empty', () => {
    render(<ApiKeySettings />)

    const saveButton = screen.getByTestId('save-key-button')
    expect(saveButton).toBeDisabled()
  })

  it('enables save button when user types a key', () => {
    render(<ApiKeySettings />)

    const input = screen.getByTestId('xai-key-input')
    const saveButton = screen.getByTestId('save-key-button')

    fireEvent.change(input, { target: { value: 'sk-test-key' } })

    expect(saveButton).toBeEnabled()
  })

  it('calls set_setting and shows success badge on happy path', async () => {
    const mockInvoke = vi.mocked(invoke)
    mockInvoke.mockResolvedValueOnce(undefined) // set_setting succeeds

    render(<ApiKeySettings />)

    const input = screen.getByTestId('xai-key-input')
    const saveButton = screen.getByTestId('save-key-button')

    fireEvent.change(input, { target: { value: 'sk-happy-path-key' } })
    fireEvent.click(saveButton)

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('set_setting', {
        key: 'xai_api_key',
        value: 'sk-happy-path-key',
      })
    })

    expect(screen.getByTestId('saved-badge')).toBeInTheDocument()
  })

  it('shows error message on unhappy path (failed save)', async () => {
    const mockInvoke = vi.mocked(invoke)
    mockInvoke.mockRejectedValueOnce('Some backend error')

    render(<ApiKeySettings />)

    const input = screen.getByTestId('xai-key-input')
    const saveButton = screen.getByTestId('save-key-button')

    fireEvent.change(input, { target: { value: 'sk-bad-key' } })
    fireEvent.click(saveButton)

    await waitFor(() => {
      expect(screen.getByTestId('error-message')).toHaveTextContent(/Failed to save key/i)
    })
  })

  it('toggles key visibility', () => {
    render(<ApiKeySettings />)

    const input = screen.getByTestId('xai-key-input') as HTMLInputElement
    const toggleBtn = screen.getByTestId('toggle-visibility')

    expect(input.type).toBe('password')

    fireEvent.click(toggleBtn)
    expect(input.type).toBe('text')

    fireEvent.click(toggleBtn)
    expect(input.type).toBe('password')
  })
})
