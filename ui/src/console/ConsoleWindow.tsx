import { FitAddon } from '@xterm/addon-fit'
import { Terminal } from '@xterm/xterm'
import '@xterm/xterm/css/xterm.css'
import { useEffect, useRef, useState, type FormEvent } from 'react'
import { useTranslation } from 'react-i18next'
import type { Problem } from '../bindings/Problem'
import { Button } from '../components/Button'
import { ProblemNotice } from '../components/ProblemNotice'
import { api, toProblem, type ProjectView } from '../lib/api'

type Status = 'running' | 'finished' | 'stopped'

function cssVar(name: string) {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim()
}

/** The built-in console: a real terminal for command-line projects, in its own window. */
export function ConsoleWindow({ projectId }: { projectId: string }) {
  const { t } = useTranslation()
  const host = useRef<HTMLDivElement>(null)
  const term = useRef<Terminal | null>(null)
  const [project, setProject] = useState<ProjectView | null>(null)
  const [status, setStatus] = useState<Status>('running')
  const [options, setOptions] = useState('')
  const [problem, setProblem] = useState<Problem | null>(null)

  useEffect(() => {
    api
      .getProject(projectId)
      .then((p) => {
        setProject(p)
        setStatus(p.running ? 'running' : 'finished')
        document.title = p.name
      })
      .catch((err) => {
        // The project was removed while this window was open.
        setStatus('finished')
        setProblem(toProblem(err))
      })
  }, [projectId])

  useEffect(() => {
    if (!host.current) return
    const terminal = new Terminal({
      fontFamily: cssVar('--font-mono') || 'monospace',
      fontSize: 13,
      cursorBlink: true,
      convertEol: false,
      scrollback: 5000,
      // Each run starts by clearing the screen; keep the previous run's output above it.
      scrollOnEraseInDisplay: true,
      theme: { background: cssVar('--surface-sunken'), foreground: cssVar('--fg'), cursor: cssVar('--accent') },
    })
    const fit = new FitAddon()
    terminal.loadAddon(fit)
    terminal.open(host.current)
    fit.fit()
    terminal.focus()
    term.current = terminal

    void api.consoleScrollback(projectId).then((text) => terminal.write(text))
    void api.consoleResize(projectId, terminal.cols, terminal.rows)
    const input = terminal.onData((data) => void api.consoleWrite(projectId, data))
    const resized = terminal.onResize(({ cols, rows }) => void api.consoleResize(projectId, cols, rows))
    const observer = new ResizeObserver(() => fit.fit())
    observer.observe(host.current)
    const off = api.onEvent((e) => {
      if (e.projectId !== projectId) return
      if (e.type === 'console') terminal.write(e.data)
      if (e.type === 'runState') {
        if (e.state === 'starting' || e.state === 'running') setStatus('running')
        else setStatus(e.state === 'stopped' && e.exitCode === null ? 'stopped' : 'finished')
      }
    })
    return () => {
      input.dispose()
      resized.dispose()
      observer.disconnect()
      void off.then((f) => f())
      terminal.dispose()
    }
  }, [projectId])

  const runAgain = async (e?: FormEvent) => {
    e?.preventDefault()
    setProblem(null)
    try {
      setStatus('running')
      await api.launchProject(projectId, options)
    } catch (err) {
      setStatus('finished')
      setProblem(toProblem(err))
    }
    term.current?.focus()
  }

  return (
    <div className="flex h-full flex-col bg-sunken">
      <header className="flex items-center gap-3 border-b border-border bg-surface px-4 py-2">
        <h1 className="min-w-0 flex-1 truncate text-sm font-semibold">{project?.name ?? ''}</h1>
        <span className="text-sm text-muted" aria-live="polite">
          {t(`console.status.${status}`)}
        </span>
        {status === 'running' && <Button onClick={() => void api.stopProject(projectId)}>{t('action.stop')}</Button>}
      </header>
      <div ref={host} className="min-h-0 flex-1 px-2 py-1" aria-label={t('console.terminal')} />
      {status !== 'running' && (
        <form onSubmit={runAgain} className="flex flex-col gap-2 border-t border-border bg-surface px-4 py-3">
          {problem && <ProblemNotice problem={problem} onDismiss={() => setProblem(null)} />}
          <div className="flex gap-2">
            <label htmlFor="console-options" className="sr-only">
              {t('console.options')}
            </label>
            <input
              id="console-options"
              value={options}
              onChange={(e) => setOptions(e.target.value)}
              placeholder={t('console.optionsPlaceholder')}
              spellCheck={false}
              autoCapitalize="off"
              autoCorrect="off"
              className="min-h-9 flex-1 rounded-lg border border-border bg-surface px-3 font-mono text-sm text-fg placeholder:font-sans placeholder:text-muted"
            />
            <Button type="submit" variant="primary">
              {t('console.runAgain')}
            </Button>
          </div>
        </form>
      )}
    </div>
  )
}
