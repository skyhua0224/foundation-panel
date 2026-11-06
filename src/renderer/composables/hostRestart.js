import { actionConfirm, serviceTexts } from 'foundation-ui/service'

/**
 * 重启 Sunshine and the switch to user mode from anywhere in the panel (the offline page, the search actions, the home tile): the same
 * confirmation as 系统 › Sunshine 服务 (how many streams are live, administrator approval), then the hardened back end
 * (src-tauri/src/sunshine_service; no PowerShell, no `net stop`). The back end refuses a restart during a live stream without the
 * acknowledgement the confirmation gives.
 *
 *   createHostRestart({ locale, ask, status, restart, userMode })
 *     ask(options) -> Promise   the window's dialog (foundation-ui useDialogs().ask): resolves on confirm, rejects on cancel
 *     status() -> Promise       get_sunshine_service_status (for the live stream count; a failure means "unknown", no count is shown)
 *     restart(ack), userMode(ack) the panel commands
 *   .confirm(kind) -> { acknowledgeLiveStream } | null (cancelled)      kind: 'restart' | 'user-mode'
 *   .run(kind)     -> true when it ran, false when the person cancelled; a failure of the command throws
 */
export function createHostRestart({ locale, ask, status, restart, userMode }) {
  const T = () => serviceTexts(typeof locale === 'function' ? locale() : locale?.value ?? locale)
  async function confirm(kind) {
    let current = null
    try { current = await status() } catch { current = null }
    const c = actionConfirm(kind, current, T())
    try {
      await ask({ title: c.title, message: c.text, type: 'warning', danger: c.danger, confirmText: c.action, cancelText: T().cancel })
    } catch { return null }
    return { acknowledgeLiveStream: c.acknowledgeLiveStream }
  }
  async function run(kind) {
    const answer = await confirm(kind)
    if (!answer) return false
    await (kind === 'user-mode' ? userMode(answer.acknowledgeLiveStream) : restart(answer.acknowledgeLiveStream))
    return true
  }
  return { confirm, run }
}
