import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import '../lib/i18n'
import { ProblemNotice } from './ProblemNotice'

afterEach(cleanup)

describe('ProblemNotice', () => {
  it('shows a human sentence and a fix, never the raw code', () => {
    const onFix = vi.fn()
    render(
      <ProblemNotice
        problem={{ code: 'RATE_LIMITED', params: { minutes: '3' }, fix: { kind: 'waitAndRetry', seconds: 180 }, detail: null }}
        onFix={onFix}
      />,
    )
    expect(screen.getByRole('alert').textContent).toContain('GitHub asked us to slow down.')
    expect(screen.getByRole('alert').textContent).toContain('about 3 minutes')
    expect(screen.queryByText(/RATE_LIMITED/)).toBeNull()
    fireEvent.click(screen.getByRole('button', { name: 'Try again' }))
    expect(onFix).toHaveBeenCalledOnce()
  })

  it('names the missing part when a project stops right away', () => {
    render(
      <ProblemNotice problem={{ code: 'EXITED_EARLY', params: { module: 'cv2' }, fix: { kind: 'repair' }, detail: 'trace' }} />,
    )
    expect(screen.getByRole('alert').textContent).toContain('“cv2”')
    expect(screen.getByRole('button', { name: 'Copy diagnostics' })).toBeTruthy()
  })
})
