// Direction B: settings list. Five categories, in-page list of destinations, flat grouped rows.
import { footerNav, host, gamepadTypes, dsFacts, presets, micBackends } from './content.mjs'
import { icon, chip, btn, seg, sw, slider, winctl, page } from './parts.mjs'

const cats = [
  { id: 'overview', label: '概览', icon: 'house' },
  { id: 'stream', label: '串流', icon: 'tower-broadcast' },
  { id: 'input', label: '输入与外设', icon: 'gamepad' },
  { id: 'display', label: '显示', icon: 'display' },
  { id: 'system', label: '系统', icon: 'screwdriver-wrench' },
]
const children = {
  input: [
    { id: 'gamepad', label: '手柄', icon: 'gamepad', dot: 'ok' },
    { id: 'microphone', label: '麦克风', icon: 'microphone', dot: 'idle' },
    { id: 'usb', label: 'USB 设备', icon: 'plug', dot: 'ok' },
  ],
}
const catOf = { overview: 'overview', gamepad: 'input', microphone: 'input' }

const navItem = (item, active) => `<a class="navitem${item.id === active ? ' is-active' : ''}"><span class="navitem__ic">${icon(item.icon)}</span><span class="navitem__label">${item.label}</span></a>`
const shell = (id, crumb, title, sub, body, list) => `<div class="shell">
  <aside class="side">
    <div class="brand"><span class="brand__mark"><i class="sunmark"></i></span><span class="brand__name"><b>Foundation</b><small>Sunshine</small></span></div>
    <nav class="nav">${cats.map((c) => navItem(c, catOf[id])).join('')}</nav>
    <nav class="nav nav--foot">${footerNav.map((i) => navItem(i, '')).join('')}</nav>
  </aside>
  <main class="main">
    <div class="topbar">${winctl()}</div>
    <header class="head"><div class="crumb">${crumb}</div><h1>${title}</h1>${sub ? `<p>${sub}</p>` : ''}</header>
    <div class="body${list ? ' body--split' : ''}">
      ${list ? `<nav class="locallist" aria-label="${crumb}">${list}</nav>` : ''}
      <div class="detail">${body}</div>
    </div>
  </main>
</div>`

const localList = (cat, active) => `${children[cat].map((c) => `<a class="local${c.id === active ? ' is-active' : ''}">${icon(c.icon)}<span>${c.label}</span><i class="navdot navdot--${c.dot}"></i></a>`).join('')}`

const sec = (title, desc, body, aside = '') => `<section class="sec"><header class="sec__head"><h2>${title}</h2>${aside}</header>${desc ? `<p class="sec__desc">${desc}</p>` : ''}${body}</section>`
const group = (rows) => `<div class="group">${rows}</div>`
const line = (label, desc, ctl, extra = '') => `<div class="line ${extra}"><div class="line__text"><div class="line__label">${label}</div>${desc ? `<div class="line__desc">${desc}</div>` : ''}</div><div class="line__ctl">${ctl}</div></div>`
const go = (label, desc, status, ic) => `<a class="line line--go">${ic ? `<span class="line__ic">${icon(ic)}</span>` : ''}<div class="line__text"><div class="line__label">${label}</div>${desc ? `<div class="line__desc">${desc}</div>` : ''}</div><div class="line__ctl">${status || ''}<span class="chev">${icon('chevron-right')}</span></div></a>`
const state = (kind, text) => `<span class="state state--${kind}"><i class="dot"></i>${text}</span>`

function overview() {
  return shell('overview', '概览', `${host.name} 运行中`, `Foundation Sunshine ${host.version} · 已运行 ${host.uptime}`,
    sec('设备与外设', '', group(
      go('手柄', '原生 DualSense 已内置在 Sunshine 中，当前没有手柄在使用', state('ok', '已就绪'), 'gamepad') +
      go('麦克风', '开始串流并打开客户端麦克风后会自动创建', state('idle', '等待使用'), 'microphone') +
      go('USB 设备', 'USB/IP 传输 v0.9.8.1，0 台设备已挂载', state('ok', '可用'), 'plug') +
      go('虚拟显示器', '每客户端专属 VDD，11 个分辨率，7 个刷新率', state('ok', '已就绪'), 'display'))) +
    sec('串流', '', group(
      go('客户端', '', `<span class="val">${host.clients} 台已配对</span>`, 'link') +
      go('串流应用', '', `<span class="val">${host.apps} 个</span>`, 'table-cells') +
      line('当前串流', '', '<span class="val">空闲</span>'))) +
    sec('维护', '', group(
      go('组件', 'USB/IP 传输、虚拟手柄驱动', state('ok', '全部正常'), 'puzzle-piece') +
      go('软件更新', `当前版本 ${host.version}`, '<span class="val">检查更新</span>', 'rotate'))))
}

function gamepad() {
  const picks = gamepadTypes.map((t) => `<div class="line pick${t.id === 'ds5' ? ' is-on' : ''}" role="radio" aria-checked="${t.id === 'ds5'}"><div class="line__text"><div class="line__label">${t.title}</div><div class="line__desc">${t.desc}</div></div><div class="line__ctl"><i class="radio"></i></div></div>`).join('')
  return shell('gamepad', '输入与外设 › 手柄', '手柄', '选择串流时游戏识别的手柄，并调校 DualSense。',
    sec('串流时的手柄类型', '', group(picks + go('每个游戏的单独设置', '', ''))) +
    sec('原生 DualSense', 'DualSense 支持已内置在 Sunshine 主机中，这里显示的是主机和系统的实际状态。', group(dsFacts.map(([k, v]) => line(k, '', state(k === '手柄' ? 'idle' : 'ok', v))).join('')), chip('ok', '已就绪')) +
    sec('调校', '', group(line('预设', '', seg(presets.map((p) => p.title), '默认'), 'line--stack')) +
      `<div class="gap"></div>` + group(`<div class="group__title">触觉与震动</div>` + line('触觉增益', '', slider(1)) + line('启用震动', '', sw(true)) + line('震动强度', '', slider(1))) +
      `<div class="gap"></div>` + group(`<div class="group__title">灯光</div>` + line('灯条模式', '', seg(['跟随游戏', '固定', '关闭'], '跟随游戏')) + line('颜色', '', '<span class="val">由游戏决定</span>'))) +
    sec('高级', '', group(go('高级选项', '另有 92 项设置，按功能分组。展开前会先说明风险。', '<span class="val">展开</span>'))),
    localList('input', 'gamepad'))
}

function microphone() {
  return shell('microphone', '输入与外设 › 麦克风', '麦克风', '把客户端麦克风的声音传到这台主机，用于游戏语音与通话。',
    sec('状态', '', group(
      line('麦克风重定向', '还没有使用中的麦克风。开始串流并打开客户端麦克风后会自动创建，也可以点“测试麦克风”。', state('idle', '等待使用')) +
      line('当前使用', 'Windows 的默认录音和播放设备不会被更改。', '<span class="val">暂无</span>') +
      `<div class="line line--actions">${btn('测试麦克风', 'primary')}${btn(icon('rotate') + ' 刷新')}</div>`)) +
    sec('麦克风后端', '', group(line('后端', '先使用本机自带的 USB 虚拟麦克风，失败时改用 VB-CABLE。', seg(micBackends, '自动'), 'line--stack'))) +
    sec('依赖组件', '', group(go('USB/IP 传输', 'usbip-win2', state('ok', '已就绪')))),
    localList('input', 'microphone'))
}

export const pages = { overview, gamepad, microphone }
export const render = (id, theme) => page('b', theme, id, pages[id]())
