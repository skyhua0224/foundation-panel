// A language other than English and Chinese is a file of its own: the window waits for it, the others need no read, and a switch
// applies the language once it has arrived.
import assert from 'node:assert/strict'
import test from 'node:test'
import 'vue' // loaded before the fake document below, which Vue's DOM runtime would take for a real one

const store = new Map()
globalThis.localStorage = { getItem: (k) => store.get(k) ?? null, setItem: (k, v) => store.set(k, String(v)), removeItem: (k) => store.delete(k) }
globalThis.document = { documentElement: {} }
globalThis.window = { addEventListener() {}, dispatchEvent() {}, matchMedia: () => ({ addEventListener() {}, matches: false }), __TAURI_INTERNALS__: { transformCallback: () => 1, invoke: async () => 1 } }
Object.defineProperty(globalThis, 'navigator', { value: { language: 'de-DE', languages: ['de-DE'] }, configurable: true })
store.set('language', 'de')

const { i18nReady, loadLocale, useI18n } = await import('./index.js')

test('the language of the window is read before the window draws, and then its texts are the table', async () => {
  const { t } = useI18n()
  await i18nReady
  assert.equal(t.value.toolActions.adminRequiredTitle, 'Administratorrechte erforderlich')
})

test('English and Chinese need no read and a language read once is not read again', async () => {
  assert.equal(await loadLocale('en'), 'en')
  assert.equal(await loadLocale('zh'), 'zh')
  const both = await Promise.all([loadLocale('fr'), loadLocale('fr')])
  assert.deepEqual(both, ['fr', 'fr'], 'two asks together are answered by one read')
  await loadLocale('fr')
})

test('an unknown code is English', async () => {
  assert.equal(await loadLocale('xx'), 'en')
})
