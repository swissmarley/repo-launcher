import { describe, expect, it } from 'vitest'
import starters from '../../../crates/launcher-core/src/starters.rs?raw'
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

  // The engine's starter list names its wording by key; each needs a name and description.
  it('cover every starter project', () => {
    const keys = [...starters.matchAll(/\(\s*"(\w+)",\s*"https:/g)].map((m) => m[1])
    expect(keys.length).toBe(5)
    const items = en.starters.items as Record<string, { name?: string; description?: string }>
    expect(keys.filter((k) => !items[k]?.name || !items[k]?.description)).toEqual([])
  })
})
