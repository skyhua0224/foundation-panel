// The frame moved by itself: a link inside a Web page, an address, a place set by the page's hash (/config#tab=audio). Whatever moves the frame, the panel's route has to be the page that is shown: the rail,
// the list row and the title are all computed from the route (one back stack, createNav), so a route that stayed behind is exactly the defect "the right side jumped, the left side stayed".
// This module decides, from the path the frame reports, whether the route has to move and where to. SidebarMenu applies it as a jump (Back returns to the page the link was on).
// The route tables are passed in (`routeForWebPath`, `routeOfPlace`, `parseHash`): the module itself needs no bundler, so the rules can be tested with the real tables' answers.

// the Web pages the panel has a place for; any other address of the frame (login, welcome, a download) is not a move of the panel
const KNOWN = [/^\/(?:pin|clients)$/, /^\/apps$/, /^\/config$/, /^\/downloads$/, /^\/password$/, /^\/(?:troubleshooting|diagnostics)$/, /^\/dualsense$/, /^\/(?:index)?$/]
const pathnameOf = (path) => String(path || '').split(/[?#]/, 1)[0].replace(/\.html$/, '').replace(/\/+$/, '') || '/'

/**
 * `path` = the address the frame reports ('/pin#client-list', '/config#tab=audio&sub=sound'); `current` = { route, params } of the panel;
 * deps = { routeForWebPath(path) -> route, routeOfPlace(tab, sub) -> route | null, parseHash(hash) -> { tab, sub?, focus? } | null, configRoute }.
 * Returns null (the route already is that page, or the address says nothing the panel can follow) or { route, params }.
 */
export function framedMove(path, current, { routeForWebPath, routeOfPlace, parseHash, configRoute }) {
  const p = String(path || '')
  if (!p.startsWith('/') || p.startsWith('//')) return null
  const name = pathnameOf(p)
  if (!KNOWN.some((re) => re.test(name))) return null
  let route = routeForWebPath(p)
  let params
  if (name === '/config') {
    const at = p.indexOf('#')
    params = at >= 0 ? parseHash(p.slice(at)) || undefined : undefined
    // a bare /config names no tab: the page keeps the one it has, there is nothing to follow
    if (!params) return null
    route = routeOfPlace(params.tab, params.sub) || configRoute
  }
  if (!route) return null
  if (route === current.route) {
    // a tab of the settings page is a move inside the page only when it is the same tab
    if (route !== configRoute) return null
    if (!params || (current.params && current.params.tab === params.tab)) return null
  }
  return { route, params: params ? { tab: params.tab, ...(params.sub ? { sub: params.sub } : {}), ...(params.focus ? { focus: params.focus } : {}) } : undefined }
}
