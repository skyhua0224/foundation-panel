import { existsSync } from 'node:fs'
import { resolve, join } from 'path'
import vuePlugin from '@vitejs/plugin-vue'
import { defineConfig } from 'vite'
import { fileURLToPath } from 'node:url'
import { dirname } from 'node:path'

const __filename = fileURLToPath(import.meta.url)
const __dirname = dirname(__filename)

const rendererSrcPath = join(__dirname, 'src', 'renderer')
const foundationUi = process.env.FOUNDATION_UI_DIR || join(__dirname, '..', 'ds5-ui-m3e')
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
    sourcemap: !!process.env.TAURI_DEBUG,
    rollupOptions: {
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
    },
  },
  plugins: [
    vuePlugin({ template: { compilerOptions: { isCustomElement: (tag) => tag.startsWith('md-') } } }),
  ],
  resolve: {
    // foundation-ui is the shared Material 3 Expressive package (sibling checkout); Vue must stay a single copy.
    dedupe: ['vue'],
    alias: [
      // every call to the host goes through this import: the timed one (src/renderer/tauri-core-timed.js) gives the package back with a timed invoke
      { find: /^@tauri-apps\/api\/core$/, replacement: join(rendererSrcPath, 'tauri-core-timed.js') },
      { find: '@', replacement: rendererSrcPath },
      { find: /^@settings-data$/, replacement: settingsData },
      { find: /^foundation-ui\/vue$/, replacement: join(foundationUi, 'src/vue/index.js') },
      { find: /^foundation-ui\/all\.css$/, replacement: join(foundationUi, 'src/css/all.css') },
      { find: /^foundation-ui\/(.*)$/, replacement: join(foundationUi, 'src/$1') },
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
