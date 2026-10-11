import i18n from 'i18next'
import { initReactI18next } from 'react-i18next'
import en from '../locales/en.json'
import { pseudoTable } from './pseudo'

const wantsPseudo =
  import.meta.env.VITE_PSEUDO === '1' ||
  (typeof window !== 'undefined' && new URLSearchParams(window.location.search).get('lng') === 'pseudo')

void i18n.use(initReactI18next).init({
  lng: wantsPseudo ? 'en-XA' : 'en',
  fallbackLng: 'en',
  resources: { en: { translation: en }, 'en-XA': { translation: pseudoTable(en) } },
  interpolation: { escapeValue: false },
  returnNull: false,
})

if (typeof document !== 'undefined') document.documentElement.lang = wantsPseudo ? 'en-XA' : 'en'

export default i18n
