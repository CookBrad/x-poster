import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { invoke } from '@tauri-apps/api/core'
import { ResearchTab } from './ResearchTab'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

const mockInvoke = vi.mocked(invoke)

describe('ResearchTab', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    localStorage.clear()
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'get_setting') return 'test-key'
      if (cmd === 'get_latest_research_run') return null
      return null
    })
  })

  it('renders adjustable draft count and combined action', () => {
    render(<ResearchTab />)

    expect(screen.getByTestId('draft-generation-count')).toHaveValue(3)
    expect(screen.getByTestId('research-and-generate')).toHaveTextContent(
      'Research & Generate Posts'
    )
  })

  it('persists draft count changes', () => {
    render(<ResearchTab />)

    fireEvent.change(screen.getByTestId('draft-generation-count'), {
      target: { value: '7' },
    })

    expect(screen.getByTestId('draft-generation-count')).toHaveValue(7)
    expect(localStorage.getItem('draft_generation_count')).toBe('7')
  })
})