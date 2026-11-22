import { existsSync, readFileSync, statSync } from 'node:fs'
import { resolve, join } from 'path'
import vuePlugin from '@vitejs/plugin-vue'
import { defineConfig } from 'vite'
import { fileURLToPath, pathToFileURL } from 'node:url'
import { dirname } from 'node:path'

const __filename = fileURLToPath(import.meta.url)
const __dirname = dirname(__filename)
// An explicit override supports local shared-UI development; otherwise use
// the integrity-locked package even when a sibling checkout exists.
const foundationUi = resolve(process.env.FOUNDATION_UI_DIR ||
  resolve(dirname(fileURLToPath(import.meta.resolve('foundation-ui'))), '..'))
const uiExports = JSON.parse(readFileSync(join(foundationUi, 'package.json'), 'utf8')).exports || {}
const escapePattern = text => text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
const uiAliases = Object.entries(uiExports).filter(([, target]) => typeof target === 'string').map(([key, target]) => {
  const name = key === '.' ? 'foundation-ui' : `foundation-ui/${key.slice(2)}`
  const [before, after] = name.split('*')
  return {
    find: new RegExp(`^${escapePattern(before)}${after === undefined ? '' : `(.+)${escapePattern(after)}`}$`),
    replacement: resolve(foundationUi, target.replace('*', '$1')),
  }
})
console.info(`[foundation-ui] ${foundationUi}`)

const rendererSrcPath = join(__dirname, 'src', 'renderer')

// The desktop language files other than English and Chinese are read one at a time when their language is chosen. Written out whole, a file
// repeats every key of the English table and the files together carry a third more than the one package did; the build writes each as the
// texts in the order of the English keys (src/renderer/desktop/i18n/compact.js, with the test that the merged table is unchanged).
const compactLocaleFile = /[\\/]desktop[\\/]i18n[\\/](zh_TW|en_GB|en_US|de|fr|es|it|ja|ko|ru|pt_BR|pt|bg|cs|pl|sv|tr|uk)\.js$/
const compactDesktopLocales = () => ({
  name: 'compact-desktop-locales',
  enforce: 'pre',
  // the development server keeps modules by address: the English and Chinese tables it compares against could go stale there, and nothing is gained from a compact form
  apply: 'build',
  async load(id) {
    const file = id.split('?')[0]
    const match = compactLocaleFile.exec(file)
    if (!match) return null
    const code = match[1]
    this.addWatchFile(file)
    const here = join(dirname(file), 'compact.js')
    const stamp = statSync(file).mtimeMs
    const [{ compactLocale }, { en }, { zh }, table] = await Promise.all([
      import(pathToFileURL(here).href),
      import(pathToFileURL(join(dirname(file), 'en.js')).href),
      import(pathToFileURL(join(dirname(file), 'zh.js')).href),
      import(`${pathToFileURL(file).href}?stamp=${stamp}`).then((module) => module[code]),
    ])
    // zh_TW is read from Traditional, then Simplified, then English; every other language from itself, then English
    const { values, extras } = compactLocale(table, en, { fallbacks: code === 'zh_TW' ? [zh, en] : [en] })
    return `import { expandLocale } from './compact.js'\nexport const ${code} = expandLocale(${JSON.stringify(values)}, ${JSON.stringify(extras)})\n`
  },
})
// The settings data (the search index of the settings and the label tables) belongs to the product this panel is built into, Foundation Sunshine (GPL-3.0): it is
// read at build time from that checkout (src_assets/common/assets/web/config/settingsData, one level up and over from this repository when it is the submodule, or
// SETTINGS_DATA_DIR) and is not part of this repository. Without it the panel builds with an empty set and shows the label keys.
const settingsDataDir = process.env.SETTINGS_DATA_DIR || join(__dirname, '..', 'assets', 'web', 'config', 'settingsData')
const settingsData = existsSync(join(settingsDataDir, 'index.js')) ? join(settingsDataDir, 'index.js') : join(rendererSrcPath, 'composables', 'settingsDataEmpty.js')

/**
 * https://vitejs.dev/config
 */
export default defineConfig(() => ({
  root: rendererSrcPath,
  publicDir: 'public',
  server: {
    fs: { allow: [__dirname, foundationUi] },
    port: 8080,
    strictPort: true,
    host: '127.0.0.1',
    // HMR 配置：确保 WebSocket 直接连接到 Vite 服务器
    hmr: {
      protocol: 'ws',
      host: '127.0.0.1',
      port: 8080,
    },
  },
  envPrefix: ['VITE_', 'TAURI_'],
  open: false,
  build: {
    outDir: join(__dirname, 'dist'),
    emptyOutDir: true,
    target: ['es2021', 'chrome100', 'safari15'],
    minify: !process.env.TAURI_DEBUG ? 'esbuild' : false,
    // Release bundles must never carry source maps; debug builds remain
    // inspectable through the explicit dev server instead. Source:
    // panel-vite-build-options.
    sourcemap: false,
    rolldownOptions: {
      input: {
        sunshineFrame: resolve(rendererSrcPath, 'sunshine-frame.html'),
        latencyClock: resolve(rendererSrcPath, 'latency-clock/index.html'),
        home: resolve(rendererSrcPath, 'home/index.html'),
        toolbar: resolve(rendererSrcPath, 'toolbar/index.html'),
        ctxMenu: resolve(rendererSrcPath, 'ctxmenu/index.html'),
        toolWindow: resolve(rendererSrcPath, 'tool-window/index.html'),
        desktop: resolve(rendererSrcPath, 'desktop/index.html'),
        launchSplash: resolve(rendererSrcPath, 'launch-splash/index.html'),
      },
      output: {
        // The launch splash window is on screen with the game's launch and its page loads in the tray state: it must not pull the one big shared
        // chunk of the panel's pages. The icon set, the icon and loading components it draws with are a small chunk of their own, which the
        // other pages share too (nothing is loaded twice).
        codeSplitting: {
          groups: [
            { name: 'ui-core', test: /foundation-ui[\\/]src[\\/](vue[\\/](MIcon|MLoading|useOnScreen)\.|icons[\\/]|launchsplash[\\/])/, priority: 20 },
          ],
        },
      },
    },
  },
  plugins: [
    compactDesktopLocales(),
    vuePlugin({ template: { compilerOptions: { isCustomElement: (tag) => tag.startsWith('md-') } } }),
  ],
  resolve: {
    // Resolve every shared module from the same source; Vue stays a single copy.
    dedupe: ['vue'],
    alias: [
      ...uiAliases,
      { find: /^foundation-ui\/(.*)$/, replacement: join(foundationUi, 'src/$1') },
      // every call to the host goes through this import: the timed one (src/renderer/tauri-core-timed.js) gives the package back with a timed invoke
      { find: /^@tauri-apps\/api\/core$/, replacement: join(rendererSrcPath, 'tauri-core-timed.js') },
      { find: '@', replacement: rendererSrcPath },
      { find: /^@settings-data$/, replacement: settingsData },
      { find: /^vue$/, replacement: join(__dirname, 'node_modules/vue/dist/vue.runtime.esm-bundler.js') },
    ],
  },
  define: {
    // 根据构建模式设置环境变量
    __PROD__: JSON.stringify(process.env.NODE_ENV === 'production'),
    __DEV__: JSON.stringify(process.env.NODE_ENV === 'development'),
    __BETA__: JSON.stringify(process.env.BETA === 'true'),
  },
  css: {
    preprocessorOptions: {
      less: {
        math: 'always',
        relativeUrls: true,
        javascriptEnabled: true,
        additionalData: `
          @import "@/styles/variables.less";
          @import "@/styles/mixins.less";
        `,
      },
    },
  },
  clearScreen: false,
}))
