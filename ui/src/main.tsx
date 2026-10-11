import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import App from './App.tsx'
import { ConsoleWindow } from './console/ConsoleWindow.tsx'
import './index.css'
import './lib/i18n'

// Console windows load the same bundle with `#console/<project id>`.
const consoleId = window.location.hash.match(/^#console\/([a-z0-9]+)$/)?.[1]

createRoot(document.getElementById('root')!).render(
  <StrictMode>{consoleId ? <ConsoleWindow projectId={consoleId} /> : <App />}</StrictMode>,
)
