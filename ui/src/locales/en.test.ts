import { describe, expect, it } from 'vitest'
import problemCodes from '../bindings/ProblemCode.ts?raw'
import en from './en.json'

describe('English strings', () => {
  // Every problem the engine can report must have human wording (no codes in the UI).
  it('cover every ProblemCode', () => {
    const codes = [...problemCodes.matchAll(/"([A-Z_]+)"/g)].map((m) => m[1])
    expect(codes.length).toBeGreaterThan(10)
    const problems = en.problem as Record<string, unknown>
    const missing = codes.filter((c) => {
      const entry = problems[c] as { title?: string } | undefined
      return !entry?.title
    })
    expect(missing).toEqual([])
  })
})
