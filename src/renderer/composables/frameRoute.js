// Where the panel's Web frame goes when the route or a request changes. SunshineFrame owns the iframe; the decisions are here so that they can be tested with the real route tables
// and so that the test harness drives its frame with the same rules (a harness with a frame of its own rules missed the blank frame of the rail, see below).
//
// The frame sleeps (about:blank, its page and timers stopped) while the panel shows a page of its own after the overview. It must never sleep while the destination is a Web page:
// the rail of 串流 asks the frame for /pin and then switches the route; the route change used to put the frame to sleep after the request, and nothing asked again (the panel
// believed the frame showed /pin), so the right pane stayed blank until a row was clicked.

export const BLANK = 'about:blank'
export const isBlank = (url) => !url || url === BLANK

/**
 * The frame's answer to a route change. `frame` = { currentPath, url, saved }; `deps` = { urlOf(path) -> address or '', webPathOf(route) -> path or null, routeForWebPath(path) -> route }.
 * Returns null (nothing to do) or { kind: 'show' | 'suspend' | 'resume', url, currentPath?, saved }.
 */
export function routeChanged({ from, to }, frame, { urlOf, webPathOf, routeForWebPath }) {
  const wanted = webPathOf(to)
  if (wanted) {
    // a Web page: the frame shows it, whatever it was doing (asleep from the overview, left blank by a window that was hidden)
    if (!isBlank(frame.url)) return null
    const path = frame.currentPath && routeForWebPath(frame.currentPath) === to ? frame.currentPath : wanted
    const url = urlOf(path)
    return url ? { kind: 'show', url, currentPath: path, saved: '' } : null
  }
  if (from === 'home' && to !== 'home') {
    if (isBlank(frame.url) && !frame.saved) return null
    return { kind: 'suspend', url: BLANK, saved: urlOf(frame.currentPath) || frame.url || frame.saved || '' }
  }
  if (from !== 'home' && to === 'home' && frame.saved && frame.saved !== BLANK) return { kind: 'resume', url: frame.saved, saved: '' }
  return null
}

/** A page of the Web UI is asked for (`path`, maybe with a hash). Returns { currentPath, url, saved } where `url` is null when the frame stays as it is. */
export function webuiNavigated(path, frame, { urlOf }) {
  if (path === frame.currentPath && !isBlank(frame.url)) return { currentPath: path, url: null, saved: '' }
  return { currentPath: path, url: urlOf(path) || null, saved: '' }
}
