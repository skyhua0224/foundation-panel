// Every call of the renderer to the host goes through the `invoke` of @tauri-apps/api/core. vite.config.js points that import at this file, which gives the package's
// exports back unchanged except `invoke`, which is timed: a call that takes 500 ms or more is reported (kind 'invoke', its command's name and time), and the calls
// that have not come back are listed with their age when a timer is late (slowCalls.js). Nothing of the Tauri runtime (__TAURI_INTERNALS__) is touched.
import { invoke as packageInvoke } from '../../node_modules/@tauri-apps/api/core.js'
import { trackCalls } from './composables/slowCalls.js'

export * from '../../node_modules/@tauri-apps/api/core.js'

let sink = null
const build = ({ now, limit } = {}) => trackCalls(packageInvoke, { report: (...line) => sink?.(...line), kind: 'invoke', nameOf: (args) => String(args[0]), now, limit })
let tracker = build()

/** The timed `invoke` the whole renderer imports. */
export const invoke = (...args) => tracker.wrapped(...args)

/** The commands that have not returned, with their age in ms. */
export const invokePending = () => tracker.pending()

/** Where the reports go (`report(kind, name, ms)`); `now` and `limit` are for tests. */
export function configureInvokeTiming({ report = null, now, limit } = {}) {
  sink = report
  if (now || limit) tracker = build({ now, limit })
}
