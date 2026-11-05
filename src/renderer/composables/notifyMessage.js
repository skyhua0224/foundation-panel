// The Element Plus message and notification calls of the earlier windows, answered by the one snackbar of the window (foundation-ui notify): a result that
// passes shows at the bottom, never as a banner over the caption bar. Same call shapes, so a caller only changes its import. The store is loaded when the first
// message is shown, so a module that only imports this (a test, a worker) does not load the UI framework.
// `duration: 0` keeps a message until its handle is closed (the old call did that); an `onClick` of a notification becomes the action of the snackbar,
// worded by `actionText`.
let serial = 0
const textOf = (options) => (typeof options === 'string' ? options : [options?.title, options?.message].filter(Boolean).join(' · '))
const loadNotify = () => import('foundation-ui/notify').then(({ useNotify }) => useNotify()).catch(() => null)
const timeoutOf = (tone, options) => (options?.duration === 0 ? 0 : tone === 'error' || tone === 'warning' ? 8000 : 5000)
const show = (tone) => (options) => {
  const text = textOf(options)
  const id = `message-${++serial}`
  const action = typeof options?.onClick === 'function' && options.actionText ? { action: options.actionText, onAction: options.onClick } : {}
  const ready = text ? loadNotify().then((notify) => { notify?.snack(text, { id, timeout: timeoutOf(tone, options), ...action }); return notify }).catch(() => null) : Promise.resolve(null)
  return { close: () => ready.then((notify) => notify?.dismissSnack(id)) }
}
export const ElMessage = Object.assign(show('info'), { success: show('success'), warning: show('warning'), error: show('error'), info: show('info'), closeAll: () => {} })
export const ElNotification = Object.assign(show('info'), { success: show('success'), warning: show('warning'), error: show('error'), info: show('info'), closeAll: () => {} })

// A wait with words (a download with its progress) in the place of the old full-window loading layer: one snackbar that stays, is rewritten in place as the text
// changes (the same id) and is taken away by close(). Nothing covers the window, so a caller that must not run twice keeps its own flag.
export const showBusy = (text) => {
  const id = `busy-${++serial}`
  const ready = loadNotify()
  let closed = false
  const put = (words) => ready.then((notify) => { if (!closed) notify?.snack(words, { id, timeout: 0 }) }).catch(() => {})
  put(text)
  return {
    setText: put,
    close: () => { closed = true; return ready.then((notify) => notify?.dismissSnack(id)) },
  }
}

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
