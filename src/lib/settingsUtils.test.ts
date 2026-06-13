import { describe, it, expect } from 'vitest'
import { countFilledFields, maskSecret } from './settingsUtils'

describe('settingsUtils', () => {
  it('masks secrets while keeping a short prefix', () => {
    expect(maskSecret('sk-abcdefghijklmnop')).toBe('sk-abcd••••••••')
    expect(maskSecret('')).toBe('')
  })

  it('counts filled credential fields', () => {
    expect(
      countFilledFields(
        { a: 'value', b: '  ', c: 'other' },
        ['a', 'b', 'c', 'd']
      )
    ).toBe(2)
  })
})