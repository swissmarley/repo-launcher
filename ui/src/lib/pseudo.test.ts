import { describe, expect, it } from 'vitest'
import { pseudo, pseudoTable } from './pseudo'

describe('pseudo-locale', () => {
  it('matches the Rust shell', () => {
    // Same expectation as `pseudo_keeps_placeholders` in app/src/strings.rs.
    expect(pseudo('Open {{name}}')).toBe('⟦Óþéñ {{name}} ~~⟧')
    expect(pseudo('')).toBe('⟦⟧')
  })

  it('keeps empty strings empty and nests', () => {
    expect(pseudoTable({ a: '', b: { c: 'Hi' } })).toEqual({ a: '', b: { c: '⟦Ĥí ~⟧' } })
  })
})
