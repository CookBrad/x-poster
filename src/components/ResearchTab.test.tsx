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

  it('renders source mode, draft count, and action buttons', () => {
    render(<ResearchTab />)

    expect(screen.getByTestId('research-mode')).toHaveValue('both')
    expect(screen.getByTestId('draft-generation-count')).toHaveValue('3')
    expect(screen.getByTestId('research-button')).toHaveTextContent('Research')
    expect(screen.getByTestId('generate-button')).toHaveTextContent('Generate')
    expect(screen.getByTestId('run-all-button')).toHaveTextContent('Run All')
  })

  it('disables X research when no xAI key is saved', () => {
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'get_setting') return null
      if (cmd === 'get_latest_research_run') return null
      return null
    })

    render(<ResearchTab />)

    fireEvent.change(screen.getByTestId('research-mode'), { target: { value: 'rss' } })
    expect(screen.getByTestId('research-button')).not.toBeDisabled()

    fireEvent.change(screen.getByTestId('research-mode'), { target: { value: 'x' } })
    expect(screen.getByTestId('research-button')).toBeDisabled()
    expect(screen.getByTestId('run-all-button')).toBeDisabled()
  })

  it('persists draft count changes', () => {
    render(<ResearchTab />)

    fireEvent.change(screen.getByTestId('draft-generation-count'), {
      target: { value: '7' },
    })

    expect(screen.getByTestId('draft-generation-count')).toHaveValue('7')
    expect(localStorage.getItem('draft_generation_count')).toBe('7')
  })
})