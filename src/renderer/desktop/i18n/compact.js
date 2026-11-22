// The language files other than English and Chinese hold the same keys as English: written out again in every file, the keys (and, for
// English (UK/US), every unchanged text) were about a third of what the package carries once each language is a file of its own.
// The build writes such a file as the texts in the order of the English table's keys (`compactLocale`, run by the Vite plugin in
// vite.config.js) and this puts the table back (`expandLocale`) when the language is read. A key the file does not have, or has empty, is
// left out: the merge (merge.js) takes it from the language it falls back to, exactly as before. The sources stay the plain tables
// that people edit and the tests read.
import { en } from './en.js'

const SEPARATOR = '\u0001'
const isContainer = (node) => !!node && typeof node === 'object'

// the paths of every text in a table, in the order the table lists them
function leafPaths(node, path = [], out = []) {
  if (Array.isArray(node)) node.forEach((item, index) => leafPaths(item, [...path, index], out))
  else if (isContainer(node)) for (const key of Object.keys(node)) leafPaths(node[key], [...path, key], out)
  else out.push(path)
  return out
}

let englishPaths = null
const orderOf = () => (englishPaths ??= leafPaths(en))

function put(root, path, value) {
  let node = root
  for (let i = 0; i < path.length - 1; i++) {
    const key = path[i]
    node[key] ??= typeof path[i + 1] === 'number' ? [] : {}
    node = node[key]
  }
  node[path[path.length - 1]] = value
}

/**
 * The compact form of one language's table, for the build: `values` are its texts in the order of the English table's keys (an empty
 * string where it has none, or none that differs from what the merge would take from the languages it falls back to), `extras` are texts
 * under keys English does not have. `fallbacks` are those languages in the order the merge reads them (`[zh, en]` for zh_TW, `[en]` for
 * the others): a text equal to the one it would fall back to is left out, which changes nothing a person sees.
 */
export function compactLocale(table, english = en, { fallbacks = [english] } = {}) {
  const known = new Map(leafPaths(english).map((path) => [path.join(SEPARATOR), path]))
  const own = new Map()
  const textAt = (root, path) => path.reduce((node, key) => node?.[key], root)
  for (const path of leafPaths(table)) {
    const value = textAt(table, path)
    if (typeof value === 'string' && value !== '') own.set(path.join(SEPARATOR), { path, value })
  }
  const fallbackText = (path) => {
    for (const source of fallbacks) {
      const text = textAt(source, path)
      if (typeof text === 'string' && text !== '') return text
    }
    return undefined
  }
  const values = [...known].map(([id, path]) => {
    const text = own.get(id)?.value
    if (text === undefined) return ''
    return text === fallbackText(path) ? '' : text
  })
  const extras = [...own].filter(([id]) => !known.has(id)).map(([, { path, value }]) => [path, value])
  return { values, extras }
}

/** The table a compact form stands for (see `compactLocale`). */
export function expandLocale(values, extras = []) {
  const table = {}
  orderOf().forEach((path, index) => {
    const text = values[index]
    if (text !== '' && text !== undefined && text !== null) put(table, path, text)
  })
  for (const [path, text] of extras) put(table, path, text)
  return table
}
