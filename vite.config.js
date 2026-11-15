import { resolve, join } from 'path'
import vuePlugin from '@vitejs/plugin-vue'
import { defineConfig } from 'vite'
import { fileURLToPath, pathToFileURL } from 'node:url'
import { dirname } from 'node:path'

const __filename = fileURLToPath(import.meta.url)
const __dirname = dirname(__filename)

const rendererSrcPath = join(__dirname, 'src', 'renderer')
const foundationUi = process.env.FOUNDATION_UI_DIR || join(__dirname, '..', 'ds5-ui-m3e')

// Release builds keep their source maps OUT of the shipped files: with SOURCEMAP_DIR set, the maps are built "hidden" (no reference in the bundle) and moved to that folder, which CI keeps as a private
// artifact; error reports carry minified positions and the private pipeline maps them back to source lines (foundation-ui src/build/privateSourcemaps.js, tools/map-stack.mjs). Unset: nothing changes.
const privateMapsDir = process.env.SOURCEMAP_DIR
const privateSourcemaps = privateMapsDir ? (await import(pathToFileURL(join(foundationUi, 'src', 'build', 'privateSourcemaps.js')).href)).privateSourcemaps : null

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
    ...(privateSourcemaps ? [privateSourcemaps({ dir: privateMapsDir })] : []),
  ],
  resolve: {
    // foundation-ui is the shared Material 3 Expressive package (sibling checkout); Vue must stay a single copy.
    dedupe: ['vue'],
    alias: [
      // every call to the host goes through this import: the timed one (src/renderer/tauri-core-timed.js) gives the package back with a timed invoke
      { find: /^@tauri-apps\/api\/core$/, replacement: join(rendererSrcPath, 'tauri-core-timed.js') },
      { find: '@', replacement: rendererSrcPath },
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
