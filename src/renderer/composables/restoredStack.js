/** The back stack a window that was let go for memory left (src-tauri/inject-viewstate.js keeps it, view_state.rs hands it to the window that is made again), or null. Only destinations this build knows, at most eight. */
export function restoredStack(restore, known) {
  const list = restore && Array.isArray(restore.nav) ? restore.nav : null
  if (!list || !list.length || list.length > 8) return null
  const out = []
  for (const item of list) {
    if (!item || typeof item.id !== 'string' || !Object.prototype.hasOwnProperty.call(known, item.id)) return null
    out.push({ id: item.id, ...(item.params && typeof item.params === 'object' ? { params: item.params } : {}), ...(item.root ? { root: true } : {}) })
  }
  return out
}
