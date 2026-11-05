import { reactive } from 'vue'

// The panel's own proxy to the Web UI does not answer the window (a proxy or TUN tool may be catching loopback traffic) and the gate (proxyGate.js) has given up for now. It is one of the three causes of the
// one status page (the host is down, the host does not answer, the proxy does not answer): SunshineFrame sets this, the shell reads it. `retryNow` asks the gate for the next check at once.
export const proxyBlock = reactive({ blocked: false, attempts: 0, checking: false, retryNow: () => {} })
