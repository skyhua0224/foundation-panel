// Direction A: calm cards. Grouped sidebar, plain page header, two-column cards (the settings redesign the owner chose).
import { nav, footerNav, host, gamepadTypes, dsFacts, presets, micBackends } from './content.mjs'
import { icon, chip, btn, seg, sw, slider, row, winctl, notice, statusDot, page } from './parts.mjs'

const navItem = (item, active) => `<a class="navitem${item.id === active ? ' is-active' : ''}"${item.id === active ? ' aria-current="page"' : ''}>
  <span class="navitem__ic">${icon(item.icon)}</span><span class="navitem__label">${item.label}</span>${statusDot(item.dot)}</a>`

const shell = (active, content) => `<div class="shell">
  <aside class="side">
    <div class="brand"><span class="brand__mark"><i class="sunmark"></i></span><span class="brand__name"><b>Foundation</b><small>Sunshine</small></span></div>
    <nav class="nav" aria-label="主导航">
      ${nav.map((g) => `<div class="navgroup"><div class="navgroup__label">${g.label}</div>${g.items.map((i) => navItem(i, active)).join('')}</div>`).join('')}
    </nav>
    <nav class="nav nav--foot">${footerNav.map((i) => navItem(i, active)).join('')}</nav>
  </aside>
  <main class="main"><div class="topbar">${winctl()}</div><div class="content">${content}</div></main>
</div>`

const head = (title, desc, actions = '') => `<header class="page-head"><div class="page-head__text"><h1>${title}</h1><p>${desc}</p></div><div class="page-head__actions">${actions}</div></header>`
const card = ({ ic, title, hint = '', actions = '', cls = '' }, body) => `<section class="card ${cls}">
  <header class="card__head"><span class="card__chip">${icon(ic)}</span><h2>${title}</h2>${hint ? `<span class="card__hint">${hint}</span>` : ''}<div class="card__actions">${actions}</div></header>
  <div class="card__body">${body}</div></section>`

function overview() {
  const stats = [['已配对客户端', host.clients], ['串流应用', host.apps], ['当前串流', '空闲'], ['主机', host.name]]
  const tile = (ic, title, kind, state, text, link) => `<section class="card tile"><div class="tile__top"><span class="card__chip">${icon(ic)}</span><h2>${title}</h2>${chip(kind, state)}</div><p class="muted">${text}</p><a class="link">${link} ${icon('chevron-right')}</a></section>`
  return head('概览', `Foundation Sunshine ${host.version} · 已运行 ${host.uptime}`, chip('ok', '串流服务运行中')) +
    `<section class="card stats">${stats.map(([k, v]) => `<div class="stat"><span class="stat__k">${k}</span><span class="stat__v">${v}</span></div>`).join('')}</section>
    <h2 class="section-title">设备与外设</h2>
    <div class="grid-2">
      ${tile('gamepad', '手柄', 'ok', '已就绪', '原生 DualSense 已内置在 Sunshine 中；当前没有手柄在使用。', '管理手柄')}
      ${tile('microphone', '麦克风', 'idle', '等待使用', '还没有使用中的麦克风。开始串流并打开客户端麦克风后会自动创建。', '查看状态与测试')}
      ${tile('plug', 'USB 设备', 'ok', '可用', 'USB/IP 传输 v0.9.8.1 已就绪，0 台设备已挂载到这台主机。', '管理 USB 设备')}
      ${tile('display', '虚拟显示器', 'ok', '已就绪', '每客户端专属 VDD · 11 个分辨率 · 7 个刷新率。', '显示设置')}
    </div>
    <h2 class="section-title">快速入口</h2>
    <section class="card quick">${btn(icon('gamepad') + ' 手柄测试')}${btn(icon('file-lines') + ' 查看日志')}${btn(icon('rotate') + ' 检查更新')}</section>`
}

function gamepad() {
  const types = `<div class="opts">${gamepadTypes.map((t) => `<div class="opt${t.id === 'ds5' ? ' is-on' : ''}" role="radio" aria-checked="${t.id === 'ds5'}"><i class="opt__mark"></i><div><div class="opt__t">${t.title}</div><div class="opt__d">${t.desc}</div></div></div>`).join('')}</div>
    <a class="link link--row">每个游戏的单独设置 ${icon('chevron-right')}</a>`
  const facts = `<div class="facts">${dsFacts.map(([k, v]) => `<div class="fact"><span class="fact__k">${k}</span><span class="fact__v"><i class="dot dot--${k === '手柄' ? 'idle' : 'ok'}"></i>${v}</span></div>`).join('')}</div>`
  const presetCards = `<div class="presets">${presets.map((p) => `<div class="preset${p.on ? ' is-on' : ''}">${icon(p.icon)}<div><b>${p.title}</b><small>${p.desc}</small></div></div>`).join('')}</div>`
  return head('手柄', '选择串流时游戏识别的手柄，并调校 DualSense。', btn(icon('rotate-left') + ' 全部恢复默认', 'ghost')) +
    card({ ic: 'gamepad', title: '串流时的手柄类型' }, types) +
    card({ ic: 'circle-check', title: '原生 DualSense', hint: '这里显示的是主机和系统的实际状态', actions: chip('ok', '已就绪') + btn(icon('rotate'), 'ghost', 'btn--sm') }, facts) +
    card({ ic: 'wand-magic-sparkles', title: '预设', hint: '自定义' }, presetCards) +
    `<div class="grid-2">
      ${card({ ic: 'wave-square', title: '触觉与震动' }, row('触觉增益', '', slider(1)) + row('启用震动', '', sw(true)) + row('震动强度', '', slider(1)))}
      ${card({ ic: 'lightbulb', title: '灯光' }, row('灯条模式', '', seg(['跟随游戏', '固定', '关闭'], '跟随游戏')) + row('颜色', '', '<span class="muted">由游戏决定</span>'))}
    </div>` +
    card({ ic: 'sliders', title: '高级', hint: '另有 92 项设置，按功能分组 · 展开前会先说明风险', actions: btn('展开 ' + icon('chevron-down'), 'ghost', 'btn--sm') }, '').replace('<div class="card__body"></div>', '')
}

function microphone() {
  return head('麦克风', '把客户端麦克风的声音传到这台主机，用于游戏语音与通话。', btn(icon('rotate'), 'ghost') + btn('测试麦克风', 'primary')) +
    `<div class="grid-2">
      ${card({ ic: 'microphone', title: '麦克风重定向', actions: chip('idle', '等待使用') },
        `<p class="lead">还没有使用中的麦克风。开始串流并打开客户端麦克风后会自动创建，也可以点“测试麦克风”。</p>` +
        row('当前使用', '', '<span class="muted">暂无</span>') + row('默认录音和播放设备', 'Windows 的默认设备不会被更改。', chip('ok', '保持不变')))}
      ${card({ ic: 'sliders', title: '麦克风后端' },
        `<div class="stack">${seg(micBackends, '自动')}<p class="muted small">先使用本机自带的 USB 虚拟麦克风，失败时改用 VB-CABLE。</p></div>` +
        row('依赖组件', 'USB/IP 传输 · usbip-win2', chip('ok', '已就绪') + '<a class="link">管理组件</a>'))}
    </div>`
}

export const pages = { overview, gamepad, microphone }
export const render = (id, theme) => page('a', theme, id, shell(id, pages[id]()))
