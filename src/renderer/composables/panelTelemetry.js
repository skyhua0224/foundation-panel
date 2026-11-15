import { createEventSender, installEventListeners, setReportTransport } from 'foundation-ui/telemetry'

// Usage counts, experience numbers and errors of the control panel (telemetry v1, docs/privacy_telemetry.md). The panel hands its events to the host it controls (POST /api/telemetry/events through its
// local proxy) instead of sending them itself. Reason: the host already owns the two choices (config keys telemetry_consent and telemetry_errors), the install identity and the queue that survives a restart and
// an offline stretch; a second sender in the panel would need the same identity through GET /api/telemetry/identity, its own queue and its own consent check, and could disagree with the host about all three.
// When the host is not running there is nothing to send to and the panel has bigger problems than a statistic: the events are dropped after one retry.
// The sender and its rules are the library's (foundation-ui/telemetry events.js): fixed words and numbers only, scrubbed error text, each kind under its own choice.
// The Rust shell writes an error report (location, stack, recent actions, scrubbed log tail) when it panics. `drainShellErrors` takes what it wrote since the last call (a fatal panic left it for this
// start), `onShellError(fn)` calls `fn` when the shell survived a panic and returns the function that stops listening. The reports are queued in this sender like every other error event, so they follow the
// same error-report choice, the same endpoint and the same install identity: the shell has no sender or consent check of its own.
export function createPanelTelemetry({ getBase, getVersion = async () => '', getUpdateSource = () => '', drainShellErrors = async () => [], onShellError = async () => () => {}, fetchFn = globalThis.fetch, win = globalThis.window, doc = globalThis.document, now } = {}) {
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
  let stopShellListener = () => {}
  /** The shell's reports into the queue; anything that is not an error_report event is ignored. */
  const takeShellErrors = async () => {
    try {
      let taken = 0
      for (const event of await drainShellErrors()) {
        if (event && event.kind === 'error' && event.name === 'error_report' && event.props && typeof event.props === 'object') { sender.queue('error', 'error_report', event.props); taken++ }
      }
      // a report is sent now: the process may be about to end
      if (taken) void sender.flush()
    } catch { /* the shell has nothing to give, or is not the app window */ }
  }
  return {
    sender,
    /** Once, when the shell is up: the version, the listeners, the reports a panic of the shell left, and the start of the panel (how long it took to show, and the update source setting). */
    async start({ startMs } = {}) {
      removeListeners = installEventListeners(sender, win, doc)
      sender.setContext({ version: await getVersion() })
      sender.track.appStart({ startMs, updateSource: getUpdateSource() })
      try { stopShellListener = (await onShellError(() => { void takeShellErrors() })) || (() => {}) } catch { /* not in the app window */ }
      await takeShellErrors()
    },
    /** A page of the panel was opened (the id of the route). */
    page: (id) => sender.track.pageView(id),
    feature: (id) => sender.feature(`panel.${id}`),
    stop() { removeListeners(); stopShellListener(); return sender.flush({ keepalive: true }) },
  }
}
