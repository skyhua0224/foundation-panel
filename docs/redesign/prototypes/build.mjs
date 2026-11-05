// Builds the static prototypes: node build.mjs <outDir>
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const here = path.dirname(fileURLToPath(import.meta.url))
const out = path.resolve(process.argv[2] || path.join(here, 'out'))
const variants = { a: await import('./variant-a.mjs'), b: await import('./variant-b.mjs'), c: await import('./variant-c.mjs') }
const only = process.argv[3] ? process.argv[3].split(',') : Object.keys(variants)

fs.mkdirSync(out, { recursive: true })
fs.copyFileSync(path.join(here, 'tokens.css'), path.join(out, 'tokens.css'))
for (const key of only) {
  const dir = path.join(out, key)
  fs.mkdirSync(dir, { recursive: true })
  fs.copyFileSync(path.join(here, `${key}.css`), path.join(dir, `${key}.css`))
  for (const id of Object.keys(variants[key].pages)) {
    for (const theme of ['light', 'dark']) {
      fs.writeFileSync(path.join(dir, `${id}-${theme}.html`), variants[key].render(id, theme))
    }
  }
}
console.log('built', only.join(','), 'into', out)
