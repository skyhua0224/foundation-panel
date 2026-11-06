import { onBeforeUnmount, onMounted, watch } from 'vue'
import { tauriInvoke } from './useTauri.js'
import { gamepadConnected } from './useGamepad.js'
import { hostPadTypeOf, setHostPadType } from './padType.js'

/**
 * 串流时手柄的真实类型：Moonlight 连着这台主机时，大屏在浏览器里看到的手柄是主机的虚拟手柄，
 * 不是人手里的那台。主机在 /api/runtime/sessions 的 gamepad_type 里报告活跃的虚拟手柄（x360 / ds4 / ds5），
 * 有报告就以它为准（padType.js），没有（没在串流、主机没报告、请求失败）就回到本机手柄的 id 与 mapping。
 *
 * 只在有手柄接着的时候问（空转要尽量便宜：大屏本身就是被编码进串流画面的那一帧）。
 */
export const HOST_PAD_TYPE_POLL_MS = 5000

export function useHostPadType({ intervalMs = HOST_PAD_TYPE_POLL_MS } = {}) {
  let timer = null
  let proxyUrl = ''
  let busy = false

  async function poll() {
    if (busy) return
    busy = true
    try {
      if (!proxyUrl) proxyUrl = await tauriInvoke('wait_for_proxy_ready')
      const resp = await fetch(`${proxyUrl}/api/runtime/sessions`)
      if (!resp.ok) throw new Error(`HTTP ${resp.status}`)
      setHostPadType(hostPadTypeOf(await resp.json()))
    } catch {
      // 主机不可达或没有这条路由：看本机的手柄
      setHostPadType(null)
    } finally {
      busy = false
    }
  }

  function stop() {
    if (timer !== null) clearInterval(timer)
    timer = null
  }

  function start() {
    if (timer !== null) return
    void poll()
    timer = setInterval(poll, intervalMs)
  }

  const stopWatch = watch(gamepadConnected, (connected) => {
    if (connected) start()
    else { stop(); setHostPadType(null) }
  })
  onMounted(() => { if (gamepadConnected.value) start() })
  onBeforeUnmount(() => { stopWatch(); stop() })
  return { poll }
}
