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

  it('renders generate action when enabled', () => {
    const onGeneratePost = vi.fn()
    render(
      <ResearchSourceCard
        {...baseProps}
        canGenerate
        onGeneratePost={onGeneratePost}
      />
    )

    fireEvent.click(screen.getByTestId('generate-post-source-1'))
    expect(onGeneratePost).toHaveBeenCalledWith('source-1')
  })

  it('does not render generate action when disabled', () => {
    render(<ResearchSourceCard {...baseProps} />)
    expect(screen.queryByTestId('generate-post-source-1')).not.toBeInTheDocument()
  })
})