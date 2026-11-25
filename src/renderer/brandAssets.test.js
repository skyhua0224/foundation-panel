import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'

// foundation-ui draws no picture of 薇薇 and has no wallpaper of hers until a product registers them: the windows that show her do it, the others carry none of her files
const read = (path) => readFileSync(new URL(path, import.meta.url), 'utf8')
const registers = (path) => /import \{ registerViviAssets \} from 'foundation-ui\/brand-assets'/.test(read(path)) && /^registerViviAssets\(\)$/m.test(read(path))

test('the desktop window and the installer page register 薇薇 and her wallpapers before they mount', () => {
  for (const main of ['desktop/main.js', 'installer/main.js']) assert.ok(registers(main), main)
  assert.ok(read('desktop/main.js').indexOf('registerViviAssets()') < read('desktop/main.js').indexOf('createApp(App)'))
})

test('the small windows (menu, toolbar, pin, latency clock, launch splash, tool window, home) do not import the art', () => {
  for (const main of ['ctxmenu', 'toolbar', 'pin', 'latency-clock', 'launch-splash', 'tool-window', 'home']) assert.doesNotMatch(read(`${main}/main.js`), /brand-assets/, main)
})

test('the components that show the product name get it from the panel', () => {
  assert.match(read('config/brand.js'), /PRODUCT_NAME = 'Foundation Sunshine'/)
  assert.match(read('components/SidebarMenu.vue'), /<MBrand class="panel-caption__brand" :name="PRODUCT_NAME"/)
  assert.match(read('components/SidebarMenu.vue'), /<MScene [^\n]*:brand-name="PRODUCT_NAME"/)
  for (const page of ['DiagnosticsPage', 'DoctorPage']) assert.match(read(`components/m3e/${page}.vue`), /:product="PRODUCT_NAME"/)
})
