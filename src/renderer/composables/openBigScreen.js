import { tauriInvoke } from '../desktop/composables/useTauri.js'

/**
 * 打开大屏（Foundation Desktop）。和托盘的「打开桌面 UI」是同一条路：已经开着就把窗口带到前台，不开第二个。
 * 失败（不在面板里、窗口创建失败）时调用方拿到原因，由它决定怎么告诉用户。
 */
export async function openBigScreen() {
  await tauriInvoke('open_desktop')
}

/** 打开大屏，失败时用 `say` 说出原因（一条消息，不抛）。返回是否打开了。 */
export async function openBigScreenWithNotice(say, failedText) {
  try {
    await openBigScreen()
    return true
  } catch (e) {
    say(`${failedText}${e?.message ? `: ${e.message}` : ''}`)
    return false
  }
}
