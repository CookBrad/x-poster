import { describe, it, expect, vi } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { SecretInput } from './SecretInput'

describe('SecretInput', () => {
  it('toggles between password and text input', () => {
    const onChange = vi.fn()
    render(
      <SecretInput
        value="secret-value"
        onChange={onChange}
        inputTestId="secret-input"
        toggleTestId="secret-toggle"
      />
    )

    const input = screen.getByTestId('secret-input') as HTMLInputElement
    expect(input.type).toBe('password')

    fireEvent.click(screen.getByTestId('secret-toggle'))
    expect(input.type).toBe('text')

    fireEvent.click(screen.getByTestId('secret-toggle'))
    expect(input.type).toBe('password')
  })
})