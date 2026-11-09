// The pages of the installer's setup UI (installer-ui/, a Tauri 2 window started by the installer shell).
// The pages are foundation-ui src/vue/installer (root MInstaller.vue); until that root exists in the foundation-ui
// checkout this builds with src/renderer/installer/Placeholder.vue, which speaks the same bridge.
import { existsSync } from 'node:fs'
import { join, dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import vuePlugin from '@vitejs/plugin-vue'
import { defineConfig } from 'vite'

const __dirname = dirname(fileURLToPath(import.meta.url))
const root = join(__dirname, 'src', 'renderer', 'installer')
const foundationUi = process.env.FOUNDATION_UI_DIR || join(__dirname, '..', 'ds5-ui-m3e')
const pagesRoot = join(foundationUi, 'src/vue/installer/MInstaller.vue')
const installerRoot = existsSync(pagesRoot) ? pagesRoot : join(root, 'Placeholder.vue')

export default defineConfig(() => ({
  root,
  base: './',
  server: { port: 8091, strictPort: true, host: '127.0.0.1' },
  build: {
    outDir: join(__dirname, 'dist-installer'),
    emptyOutDir: true,
    target: ['es2021', 'chrome100'],
    minify: 'esbuild',
    rollupOptions: { input: { installer: resolve(root, 'index.html') } },
  },
  plugins: [vuePlugin({ template: { compilerOptions: { isCustomElement: (tag) => tag.startsWith('md-') } } })],
  resolve: {
    dedupe: ['vue'],
    alias: [
      { find: /^installer-root$/, replacement: installerRoot },
      { find: /^foundation-ui\/vue$/, replacement: join(foundationUi, 'src/vue/index.js') },
      { find: /^foundation-ui\/all\.css$/, replacement: join(foundationUi, 'src/css/all.css') },
      { find: /^foundation-ui\/(.*)$/, replacement: join(foundationUi, 'src/$1') },
      { find: /^vue$/, replacement: join(__dirname, 'node_modules/vue/dist/vue.runtime.esm-bundler.js') },
    ],
  },
  define: { __INSTALLER_ROOT__: JSON.stringify(installerRoot === pagesRoot ? 'pages' : 'placeholder') },
  clearScreen: false,
}))
