import { createEventSender, installEventListeners, setReportTransport } from 'foundation-ui/telemetry'

// Usage counts, experience numbers and errors of the control panel (telemetry v1, docs/privacy_telemetry.md). The panel hands its events to the host it controls (POST /api/telemetry/events through its
// local proxy) instead of sending them itself. Reason: the host already owns the two choices (config keys telemetry_consent and telemetry_errors), the install identity and the queue that survives a restart and
// an offline stretch; a second sender in the panel would need the same identity through GET /api/telemetry/identity, its own queue and its own consent check, and could disagree with the host about all three.
// When the host is not running there is nothing to send to and the panel has bigger problems than a statistic: the events are dropped after one retry.
// The sender and its rules are the library's (foundation-ui/telemetry events.js): fixed words and numbers only, scrubbed error text, each kind under its own choice.
export function createPanelTelemetry({ getBase, getVersion = async () => '', getUpdateSource = () => '', getUpdateSourceMiss = () => '', fetchFn = globalThis.fetch, win = globalThis.window, doc = globalThis.document, now } = {}) {
  const sender = createEventSender({
    product: 'panel',
    ...(now ? { now } : {}),
    post: async (body, { keepalive }) => {
      const base = String((await getBase()) || '').replace(/\/$/, '')
      if (!base) return { ok: false, status: 0 }
      return fetchFn(`${base}/api/telemetry/events`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body), keepalive })
    },
  })
  // the reports the person sends by hand and the automatic issues travel the same way: to the host through the proxy
  setReportTransport((events) => sender.postNow(events))
  let removeListeners = () => {}
  return {
    sender,
    /** Once, when the shell is up: the version, the listeners, and the start of the panel (how long it took to show, and the update source setting). */
    async start({ startMs } = {}) {
      const offListeners = installEventListeners(sender, win, doc)
      const onErrorEvent = (event) => {
        const detail = event?.detail || {}
        sender.errorEvent({
          component: detail.component || 'panel',
          code: detail.code || 'unknown',
          severity: detail.severity || 'error',
        })
      }
      win?.addEventListener?.('foundation-error-event', onErrorEvent)
      removeListeners = () => {
        offListeners()
        win?.removeEventListener?.('foundation-error-event', onErrorEvent)
      }
      sender.setContext({ version: await getVersion() })
      sender.track.appStart({ startMs, updateSource: await getUpdateSource(), updateSourceMiss: await getUpdateSourceMiss() })
    },
    /** What the host would send next (GET /api/telemetry/preview): the records waiting and the fields every batch carries. For 关于 › 隐私 › 查看将发送的内容. */
    async preview() {
      const base = String((await getBase()) || '').replace(/\/$/, '')
      if (!base) throw new Error('no host')
      const response = await fetchFn(`${base}/api/telemetry/preview`)
      if (!response.ok) throw new Error(`http ${response.status}`)
      return response.json()
    },
    /** A page of the panel was opened (the id of the route). */
    page: (id) => sender.track.pageView(id),
    feature: (id) => sender.feature(`panel.${id}`),
    stop() { removeListeners(); return sender.flush({ keepalive: true }) },
  }
}
