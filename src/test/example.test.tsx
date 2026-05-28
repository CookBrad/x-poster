import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'

describe('Testing Setup', () => {
  it('renders a basic element correctly', () => {
    render(<div data-testid="test-element">x-poster test is working</div>)
    expect(screen.getByTestId('test-element')).toHaveTextContent('x-poster test is working')
  })

  it('supports jest-dom matchers', () => {
    render(<button>Click me</button>)
    expect(screen.getByRole('button')).toBeInTheDocument()
  })
})
