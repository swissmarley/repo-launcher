import { useTranslation } from 'react-i18next'
import { AddBar } from './components/AddBar'
import { ProjectCard } from './components/ProjectCard'
import { useProjects } from './state/useProjects'

export default function App() {
  const { t } = useTranslation()
  const projects = useProjects()
  const empty = projects.loaded && projects.projects.length === 0

  return (
    <div className="flex h-full flex-col">
      <header className="border-b border-border bg-surface/80 px-6 py-4 backdrop-blur">
        <div className="mx-auto flex max-w-5xl flex-col gap-4">
          <h1 className="text-lg font-semibold">{t('app.title')}</h1>
          <AddBar onAdd={projects.add} autoFocus={empty} />
        </div>
      </header>

      <main className="flex-1 overflow-auto px-6 py-6">
        <div className="mx-auto max-w-5xl">
          {empty ? (
            <section className="mx-auto mt-16 max-w-md text-center">
              <h2 className="text-2xl font-semibold">{t('empty.title')}</h2>
              <p className="mt-3 text-muted">{t('empty.body')}</p>
            </section>
          ) : (
            <section aria-labelledby="library-heading">
              <h2 id="library-heading" className="sr-only">
                {t('library.heading')}
              </h2>
              <div className="grid grid-cols-[repeat(auto-fill,minmax(20rem,1fr))] gap-4">
                {projects.projects.map((p) => (
                  <ProjectCard key={p.id} project={p} live={projects.live[p.id]} actions={projects} />
                ))}
              </div>
            </section>
          )}
        </div>
      </main>
    </div>
  )
}
