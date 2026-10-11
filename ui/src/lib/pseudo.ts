// The pseudo-locale: every English string accented, bracketed and about 30% longer, with
// {{placeholders}} kept. Text that isn't from en.json stays plain and stands out, and layouts
// that can't take longer translations show it. Must match `pseudo()` in app/src/strings.rs.
// Turn it on with `?lng=pseudo` in the browser, or `VITE_PSEUDO=1` when building the app.

const FROM = 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ'
const TO = [...'áƀçďéƒĝĥíĵķĺɱñóþǫŕšţúṽŵẋýžÁƁÇĎÉƑĜĤÍĴĶĹṀÑÓÞǪŔŠŢÚṼŴẊÝŽ']

export const PSEUDO_OPEN = '⟦'
export const PSEUDO_CLOSE = '⟧'

export function pseudo(text: string): string {
  let out = PSEUDO_OPEN
  let letters = 0
  for (let i = 0; i < text.length; ) {
    if (text.startsWith('{{', i)) {
      const end = text.indexOf('}}', i)
      if (end >= 0) {
        out += text.slice(i, end + 2)
        i = end + 2
        continue
      }
    }
    const c = text[i]
    const at = FROM.indexOf(c)
    if (at >= 0) {
      out += TO[at]
      letters++
    } else {
      out += c
    }
    i++
  }
  const pad = Math.ceil(letters / 3)
  if (pad > 0) out += ' ' + '~'.repeat(pad)
  return out + PSEUDO_CLOSE
}

type Tree = { [key: string]: string | Tree }

/** The whole string table, pseudo-localized. */
export function pseudoTable(table: Tree): Tree {
  return Object.fromEntries(
    Object.entries(table).map(([k, v]) => [k, typeof v === 'string' ? (v ? pseudo(v) : v) : pseudoTable(v)]),
  )
}
