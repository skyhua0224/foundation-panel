/**
 * 主机应用列表里的「大屏 / Big Screen」条目：桌面串流 + 分离启动 `sunshine-gui.exe --desktop`，串流结束时的 undo 是
 * `sunshine-gui.exe --close-desktop`（主机的 default_apps.h）。它是给 Moonlight 客户端启动大屏用的入口；
 * 在大屏自己的应用库里再启动它没有意义，而且它的 undo 会关掉正在用的这个窗口，所以应用库不列它。
 *
 * 认的是命令，不是名字：名字跟随语言、人也可以改名；Desktop 应用上的「串流后自动打开大屏」开关只加 `--desktop`
 * 而没有 `--close-desktop`，不会被误认。
 */
const has = (cmd, flag) => /sunshine-gui/i.test(String(cmd || '')) && String(cmd).split(/\s+/).includes(flag)

export function isBigScreenEntry(app) {
  const opens = (Array.isArray(app?.detached) ? app.detached : []).some((cmd) => has(cmd, '--desktop'))
  const closes = (Array.isArray(app?.['prep-cmd']) ? app['prep-cmd'] : []).some((prep) => has(prep?.undo, '--close-desktop'))
  return opens && closes
}

/** 应用库要列的应用：去掉大屏入口。 */
export const libraryApps = (list) => (Array.isArray(list) ? list.filter((app) => !isBigScreenEntry(app)) : list)
