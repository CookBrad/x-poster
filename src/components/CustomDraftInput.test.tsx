import { describe, it, expect, vi } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { CustomDraftInput } from './CustomDraftInput'

describe('CustomDraftInput', () => {
  it('disables generate without xAI key or empty input', () => {
    const onGenerate = vi.fn()
    render(
      <CustomDraftInput
        value=""
        onChange={() => {}}
        onGenerate={onGenerate}
        hasXaiKey={false}
      />
    )

    expect(screen.getByTestId('custom-draft-generate')).toBeDisabled()

    fireEvent.click(screen.getByTestId('custom-draft-generate'))
    expect(onGenerate).not.toHaveBeenCalled()
  })

  it('calls onGenerate when form is submitted with input', () => {
    const onGenerate = vi.fn()
    render(
      <CustomDraftInput
        value="Starship booster catch"
        onChange={() => {}}
        onGenerate={onGenerate}
        hasXaiKey
      />
    )

    fireEvent.click(screen.getByTestId('custom-draft-generate'))
    expect(onGenerate).toHaveBeenCalledTimes(1)
  })
})