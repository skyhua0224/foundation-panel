export const WEBUI_PAGES = Object.freeze([
  { id: 'overview', path: '/', label: 'overview' },
  { id: 'downloads', path: '/downloads', label: 'clientDownloads' },
  { id: 'pairing', path: '/pin', label: 'clientPairing' },
  { id: 'apps', path: '/apps', label: 'applications' },
  { id: 'stream-settings', path: '/config', label: 'streamSettings' },
])

export function getWebuiPage(path) {
  if (typeof path !== 'string' || !path.startsWith('/') || path.startsWith('//')) return null
  const pathname =
    path
      .split(/[?#]/, 1)[0]
      .replace(/\.html$/, '')
      .replace(/\/$/, '') || '/'
  if (pathname === '/index') return WEBUI_PAGES[0]
  return (
    WEBUI_PAGES.find((page) => pathname === page.path || (page.path !== '/' && pathname.startsWith(`${page.path}/`))) ||
    null
  )
}

// A place in the stream settings: /config#tab=encoders&focus=hevc_mode, or of the display and audio pages: /config#tab=display&sub=hdr
const CONFIG_PLACE = /^\/config#tab=[a-z]+(?:&sub=[\w-]+)?(?:&focus=[\w.-]+)?$/

export function isWebuiNavigationPath(path) {
  return (
    WEBUI_PAGES.some((page) => page.path === path) ||
    (typeof path === 'string' && CONFIG_PLACE.test(path)) ||
    path === '/password' ||
    path === '/troubleshooting'
  )
}
