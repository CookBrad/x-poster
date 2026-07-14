import { describe, it, expect, vi } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { ResearchSourceCard } from './ResearchSourceCard'

describe('ResearchSourceCard', () => {
  const baseProps = {
    sourceId: 'source-1',
    title: 'Cybertruck update',
    content: 'Smart Summon expands to more owners.',
    url: 'https://example.com/story',
    sourceName: 'Not A Tesla App',
    sourceType: 'rss',
    dateLabel: '6/13/2026',
  }

  it('renders post and reply actions when enabled', () => {
    const onGeneratePost = vi.fn()
    const onGenerateReply = vi.fn()
    render(
      <ResearchSourceCard
        {...baseProps}
        canGenerate
        onGeneratePost={onGeneratePost}
        onGenerateReply={onGenerateReply}
      />
    )

    fireEvent.click(screen.getByTestId('generate-post-source-1'))
    expect(onGeneratePost).toHaveBeenCalledWith('source-1')

    fireEvent.click(screen.getByTestId('generate-reply-source-1'))
    expect(onGenerateReply).toHaveBeenCalledWith('source-1')
  })

  it('does not render generate actions when disabled', () => {
    render(<ResearchSourceCard {...baseProps} />)
    expect(screen.queryByTestId('generate-post-source-1')).not.toBeInTheDocument()
    expect(screen.queryByTestId('generate-reply-source-1')).not.toBeInTheDocument()
  })

  it('shows used badge and hides generate actions for used stories', () => {
    render(
      <ResearchSourceCard
        {...baseProps}
        canGenerate
        isUsed
        onGeneratePost={vi.fn()}
        onGenerateReply={vi.fn()}
      />
    )

    expect(screen.getByTestId('used-badge-source-1')).toHaveTextContent('Used')
    expect(screen.queryByTestId('generate-post-source-1')).not.toBeInTheDocument()
    expect(screen.queryByTestId('generate-reply-source-1')).not.toBeInTheDocument()
  })
})
