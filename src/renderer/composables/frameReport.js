// What happens inside the Web UI frame reaches the panel's log. The page cannot write to it; the script the proxy puts into every page posts a small report to the panel
// (`panel-frame-report`), and the panel writes it with the others (`[renderer] frame-<kind> <page> <ms> ms <detail>`). A blank pane or a lag of a Web page is then a line with a time,
// a page, a script and a position, not a guess. Nothing leaves the machine: the line goes to the panel's own log file.

export const FRAME_REPORT = 'panel-frame-report'
export const FRAME_KINDS = Object.freeze(['error', 'frame', 'fetch'])
/** A reveal slower than this (ms) is written; a reveal that had to be forced by the ceiling always is. */
export const REVEAL_SLOW_MS = 600

const one = (value, max) => String(value ?? '').replace(/[\r\n\t]+/g, ' ').replace(/\s{2,}/g, ' ').trim().slice(0, max)
/** an address with no query and no hash: a query may carry a token, a PIN or a name */
const bare = (text) => String(text ?? '').replace(/(https?:\/\/[^\s?#]+|\/[^\s?#]*)[?#][^\s|]*/g, '$1')
const number = (value) => (Number.isFinite(Number(value)) ? Math.max(0, Math.min(3_600_000, Math.round(Number(value)))) : 0)

/** The long poll of the pairing list (`/api/pairing/pending?wait=25`) is a call that is meant to take 25 s: it is not a slow call. */
export const isLongPoll = (target) => /\/api\/pairing\/pending/.test(String(target ?? '')) || /[?&]wait=\d+/.test(String(target ?? ''))

/** The report of a frame as the panel writes it, or null for anything that is not one. Everything is clipped: the frame is another origin. */
export function normalizeFrameReport(data) {
  if (!data || typeof data !== 'object' || data.type !== FRAME_REPORT || !FRAME_KINDS.includes(data.kind)) return null
  const page = one(bare(data.page), 80) || '/'
  const name = one(bare(data.name), 160)
  if (!name) return null
  return { kind: `frame-${data.kind}`, name: `${page} ${name}`, ms: number(data.ms), detail: one(bare(data.detail), 300) }
}

/** At most `perMinute` reports from one source: a page that fails in a loop must not fill the log. */
export function createReportLimiter({ perMinute = 30, now = () => Date.now() } = {}) {
  let start = 0
  let count = 0
  return {
    allow() {
      const t = now()
      if (!start || t - start >= 60_000) { start = t; count = 0 }
      count += 1
      return count <= perMinute
    },
  }
}

/** How the frame was shown: null when it was quick and the page said it had painted, else the line for the log. `by` = painted | load | settle | ceiling. */
export function revealReport({ page = '', ms = 0, by = '', fcp = 0 } = {}) {
  const took = number(ms)
  if (by !== 'ceiling' && took < REVEAL_SLOW_MS) return null
  const detail = by === 'ceiling' ? 'forced after the ceiling: the page never said it had painted' : `shown by ${by || 'unknown'}${fcp ? `, first contentful paint at ${number(fcp)} ms in the page` : ''}`
  return { kind: 'reveal', name: one(bare(page), 80) || '/', ms: took, detail }
}

/** An error or a rejection of the panel's own page as one line. */
export function faultReport(source) {
  const error = source?.error ?? source?.reason
  const message = one(bare(error?.message ?? source?.message ?? error ?? 'error'), 200)
  const where = source?.filename ? `${one(bare(source.filename), 80).split('/').pop()}:${number(source.lineno)}:${number(source.colno)}` : ''
  const stack = one(String(error?.stack ?? '').split('\n').slice(1, 4).join(' | '), 240)
  return { kind: 'error', name: message || 'error', ms: 0, detail: [where, stack].filter(Boolean).join(' ') }
}
