// Lets the Node test runner resolve the pure modules of the shared package the way the build does (the locked package).
// Import this first and load the code under test with dynamic imports afterwards.
import { register } from 'node:module'
import { pathToFileURL, fileURLToPath } from 'node:url'
import { resolve, dirname } from 'node:path'

// dev-env-npm: resolve the same immutable package as the application build.
const root = resolve(dirname(fileURLToPath(import.meta.resolve('foundation-ui'))), '..')
// the router only needs the back stack and the compact-navigation rule from the component index, which cannot load in Node
const routerStub = 'data:text/javascript,' + encodeURIComponent(`export { createNav } from ${JSON.stringify(pathToFileURL(root + '/src/nav/backstack.js').href)}\nexport { isCompactNav } from ${JSON.stringify(pathToFileURL(root + '/src/nav/scenes.js').href)}`)
const i18nStub = 'data:text/javascript,' + encodeURIComponent('export const useI18n = () => ({ t: { value: {} } })')
const hook = `
const i18nStub = ${JSON.stringify(i18nStub)}
const routerStub = ${JSON.stringify(routerStub)}
const root = ${JSON.stringify(pathToFileURL(root + '/').href)}
export async function resolve(specifier, context, next) {
  const m = /^foundation-ui\\/(vue\\/guide\\.js|settings\\/.+\\.js|nav\\/.+\\.js)$/.exec(specifier)
  if (m) return { url: new URL('src/' + m[1], root).href, shortCircuit: true }
  if (specifier === 'foundation-ui/vue') return { url: routerStub, shortCircuit: true }
  // the panel's language module reaches for the window and the tray; the router only needs useI18n for titles
  if (/desktop\\/i18n\\/index\\.js$/.test(specifier) && /useRouter\\.js$/.test(context.parentURL || '')) return { url: i18nStub, shortCircuit: true }
  return next(specifier, context)
}
`
register('data:text/javascript,' + encodeURIComponent(hook))
