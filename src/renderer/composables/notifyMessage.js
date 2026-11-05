// The Element Plus message and notification calls of the earlier windows, answered by the one snackbar of the window (foundation-ui notify): a result that
// passes shows at the bottom, never as a banner over the caption bar. Same call shapes, so a caller only changes its import. The store is loaded when the first
// message is shown, so a module that only imports this (a test, a worker) does not load the UI framework.
const textOf = (options) => (typeof options === 'string' ? options : [options?.title, options?.message].filter(Boolean).join(' · '))
const show = (tone) => (options) => {
  const text = textOf(options)
  if (text) import('foundation-ui/notify').then(({ useNotify }) => useNotify().snack(text, { timeout: tone === 'error' || tone === 'warning' ? 8000 : 5000 })).catch(() => {})
  return { close: () => {} }
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
