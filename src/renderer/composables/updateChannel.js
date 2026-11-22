// The update channel of the panel (稳定版 or 测试版) and whether the person ever chose one. The preference itself lives in the native side (`get/set_include_prerelease_preference`) and in
// localStorage (the 个人设置 copy); the first-run guide and the 关于 page both write it through `saveUpdateChannel`, which also leaves a mark that it was chosen. The upgrade flow of the guide skips the
// channel step for a person who chose before; a stored 测试版 counts as chosen too, since it can only have been picked.
export const INCLUDE_PRERELEASE_KEY = 'sunshine-include-prerelease'
export const CHANNEL_CHOSEN_KEY = 'foundation.update-channel.chosen'
export const CHANNEL_EVENT = 'foundation-update-channel'

const read = (key, storage) => { try { return (storage || globalThis.localStorage)?.getItem(key) } catch { return null } }

/** True when the person chose the channel before: a mark left by a choice, or a stored 测试版. */
export function channelChosen(storage) {
  return read(CHANNEL_CHOSEN_KEY, storage) === '1' || read(INCLUDE_PRERELEASE_KEY, storage) === 'true'
}

/** Stores the channel and the mark that it was chosen. Each write on its own: a full storage that refuses one must not stop the other, nor the native save that goes with it. */
export function rememberChannel(include, storage) {
  const s = storage || globalThis.localStorage
  for (const [key, value] of [[INCLUDE_PRERELEASE_KEY, String(!!include)], [CHANNEL_CHOSEN_KEY, '1']]) {
    try { s?.setItem(key, value) } catch { /* not remembered: the native side holds it */ }
  }
}

/** Saves the channel (`include`: 测试版) in the native side and in localStorage, marks it as chosen and tells the window (CHANNEL_EVENT). Rejects when the native side refuses. */
export async function saveUpdateChannel(invoke, include, { storage, win = globalThis.window } = {}) {
  await invoke('set_include_prerelease_preference', { include: !!include })
  rememberChannel(include, storage)
  try { win?.dispatchEvent?.(new CustomEvent(CHANNEL_EVENT, { detail: !!include })) } catch { /* no window */ }
}
