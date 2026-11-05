import { ref } from 'vue'

// When the panel last checked for an update (a check the person asked for, or one the panel ran): kept in the browser storage so the 软件更新 page can say so. The updater itself
// checks on its own at start and then at an interval; only the checks that went through this window are recorded here.
const KEY = 'foundation.update.last-check'
const read = () => { try { const v = Number(localStorage.getItem(KEY)); return Number.isFinite(v) && v > 0 ? v : 0 } catch { return 0 } }
const at = ref(read())

export const recordUpdateCheck = (time = Date.now()) => { at.value = time; try { localStorage.setItem(KEY, String(time)) } catch { /* not remembered */ } }
const pad = (n) => String(n).padStart(2, '0')
export const formatCheckTime = (time) => { if (!time) return ''; const d = new Date(time); return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}` }
export const useUpdateCheckTime = () => ({ at, text: () => formatCheckTime(at.value) })
