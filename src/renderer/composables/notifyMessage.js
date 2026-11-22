// The Element Plus message and notification calls of the earlier windows, answered by the one snackbar of the window (foundation-ui notify): a result that
// passes shows at the bottom, never as a banner over the caption bar. Same call shapes, so a caller only changes its import. The store is loaded when the first
// message is shown, so a module that only imports this (a test, a worker) does not load the UI framework.
// `duration: 0` keeps a message until its handle is closed (the old call did that). `id` names a message: a later call with the same id replaces it where it stands
// (a progress sentence that becomes its result changes its text and restarts its time; it does not leave and enter again as a second message would), and the handle of
// either call closes it. Without an id every call is a message of its own. A notification that waits for the person (duration 0 with an action) is a
// standing state, not a passing result: it goes to the status notices of the caption bar (the bell), where the person can dismiss it and it never occupies the
// one snackbar of the queue, so every other notice keeps appearing while it waits. A dismissed one stays away until what it reports changes (its revision).
let serial = 0
// the one message of a manual update check: its sentence, then its failure
export const UPDATE_CHECK_MESSAGE = 'update-check'
const textOf = (options) => (typeof options === 'string' ? options : [options?.title, options?.message].filter(Boolean).join(' · '))
const loadNotify = () => import('foundation-ui/notify').then(({ useNotify }) => useNotify()).catch(() => null)
const timeoutOf = (tone, options) => (options?.duration === 0 ? 0 : tone === 'error' || tone === 'warning' ? 8000 : 5000)
const TONES = { error: 'error', warning: 'warn', success: 'info', info: 'info' }
const waitsForThePerson = (options) => typeof options === 'object' && options?.duration === 0 && typeof options?.onClick === 'function' && options.actionText
const showStanding = (tone, options) => {
  const id = options.standing || `notice-${++serial}`
  const title = options?.title || options?.message || ''
  const text = options?.title && options?.message ? options.message : ''
  const ready = title ? loadNotify().then((notify) => {
    notify?.system(id, {
      tone: TONES[tone] || 'info',
      title,
      text,
      revision: options?.revision || '',
      dismissible: true,
      ...(options.actionText && typeof options.onClick === 'function' ? { actions: [{ id: `${id}-go`, label: options.actionText, onClick: options.onClick }] } : {}),
    })
    return notify
  }).catch(() => null) : Promise.resolve(null)
  return { close: () => ready.then((notify) => notify?.clearSystem(id)) }
}
const show = (tone) => (options) => {
  if (waitsForThePerson(options)) return showStanding(tone, options)
  const text = textOf(options)
  const id = (typeof options === 'object' && options?.id) || `message-${++serial}`
  const action = typeof options?.onClick === 'function' && options.actionText ? { action: options.actionText, onAction: options.onClick } : {}
  const ready = text ? loadNotify().then((notify) => { notify?.snack(text, { id, timeout: timeoutOf(tone, options), ...action }); return notify }).catch(() => null) : Promise.resolve(null)
  return { close: () => ready.then((notify) => notify?.dismissSnack(id)) }
}
export const ElMessage = Object.assign(show('info'), { success: show('success'), warning: show('warning'), error: show('error'), info: show('info'), closeAll: () => {} })
export const ElNotification = Object.assign(show('info'), { success: show('success'), warning: show('warning'), error: show('error'), info: show('info'), closeAll: () => {} })

// The message boxes of the earlier windows (confirm, alert, and the call with an options object), answered by the Material 3 dialog of the window
// (foundation-ui dialog): same call shapes and the same results (resolves 'confirm'; rejects 'cancel' or, for a caller that tells them apart, 'close').
const ask = (options) => import('foundation-ui/dialog').then(({ useDialogs }) => useDialogs().ask({
  ...options,
  type: options.type || '',
  cancel: options.showCancelButton !== false && options.cancel !== false,
  distinguish: !!options.distinguishCancelAndClose,
  persistent: options.closeOnClickModal === false,
}))
const box = (options) => ask(options || {})
export const ElMessageBox = Object.assign(box, {
  confirm: (message, title, options = {}) => ask({ ...options, message, title, cancel: true }),
  alert: (message, title, options = {}) => ask({ ...options, message, title, cancel: false }),
  prompt: () => Promise.reject(new Error('prompt is not offered')),
})
