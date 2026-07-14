import { describe, it, expect, vi } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { CustomDraftInput } from './CustomDraftInput'

describe('CustomDraftInput', () => {
  it('disables both buttons without xAI key or empty input', () => {
    const onGeneratePost = vi.fn()
    const onGenerateReply = vi.fn()
    render(
      <CustomDraftInput
        value=""
        onChange={() => {}}
        onGeneratePost={onGeneratePost}
        onGenerateReply={onGenerateReply}
        hasXaiKey={false}
      />
    )

    expect(screen.getByTestId('custom-draft-generate')).toBeDisabled()
    expect(screen.getByTestId('custom-reply-generate')).toBeDisabled()

    fireEvent.click(screen.getByTestId('custom-draft-generate'))
    fireEvent.click(screen.getByTestId('custom-reply-generate'))
    expect(onGeneratePost).not.toHaveBeenCalled()
    expect(onGenerateReply).not.toHaveBeenCalled()
  })

  it('calls onGeneratePost when Generate Post is clicked', () => {
    const onGeneratePost = vi.fn()
    const onGenerateReply = vi.fn()
    render(
      <CustomDraftInput
        value="Starship booster catch"
        onChange={() => {}}
        onGeneratePost={onGeneratePost}
        onGenerateReply={onGenerateReply}
        hasXaiKey
      />
    )

    fireEvent.click(screen.getByTestId('custom-draft-generate'))
    expect(onGeneratePost).toHaveBeenCalledTimes(1)
    expect(onGenerateReply).not.toHaveBeenCalled()
  })

  it('calls onGenerateReply when Generate Reply is clicked', () => {
    const onGeneratePost = vi.fn()
    const onGenerateReply = vi.fn()
    render(
      <CustomDraftInput
        value="https://x.com/elonmusk/status/123"
        onChange={() => {}}
        onGeneratePost={onGeneratePost}
        onGenerateReply={onGenerateReply}
        hasXaiKey
      />
    )

    fireEvent.click(screen.getByTestId('custom-reply-generate'))
    expect(onGenerateReply).toHaveBeenCalledTimes(1)
    expect(onGeneratePost).not.toHaveBeenCalled()
  })
})
