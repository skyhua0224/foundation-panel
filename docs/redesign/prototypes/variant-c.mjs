// Direction C: labelled icon rail, persistent status bar, sections with a label column instead of cards.
import { nav, footerNav, host, gamepadTypes, dsFacts, presets, micBackends } from './content.mjs'
import { icon, chip, btn, seg, sw, slider, winctl, statusDot, page } from './parts.mjs'

const railItem = (item, active) => `<a class="rail__item${item.id === active ? ' is-active' : ''}"><span class="rail__ic">${icon(item.icon)}${item.dot ? `<i class="navdot navdot--${item.dot}"></i>` : ''}</span><span class="rail__label">${item.label}</span></a>`
const groupOf = { overview: '串流', gamepad: '输入与外设', microphone: '输入与外设' }

const shell = (id, title, body, tabs = '') => `<div class="shell">
  <aside class="rail">
    <div class="rail__brand"><span class="brand__mark"><i class="sunmark"></i></span></div>
    <nav class="rail__nav" aria-label="主导航">${nav.map((g) => `<div class="rail__group">${g.items.map((i) => railItem(i, id)).join('')}</div>`).join('')}</nav>
    <nav class="rail__foot">${footerNav.map((i) => railItem(i, '')).join('')}</nav>
  </aside>
  <div class="frame">
    <header class="bar">
      <div class="bar__title"><span class="bar__group">${groupOf[id]}</span><h1>${title}</h1></div>
      <div class="bar__status" aria-label="主机状态">
        <span class="pill"><i class="dot dot--ok"></i>Sunshine 运行中</span>
        <span class="pill"><i class="dot dot--ok"></i>DualSense 已就绪</span>
        <span class="pill"><i class="dot dot--idle"></i>串流 空闲</span>
      </div>
      ${winctl()}
    </header>
    ${tabs}
    <main class="main"><div class="content">${body}</div></main>
  </div>
</div>`

const tabs = (items, on) => `<nav class="tabs" role="tablist">${items.map((t) => `<a class="tab${t === on ? ' is-on' : ''}" role="tab" aria-selected="${t === on}">${t}</a>`).join('')}</nav>`
const sec = (title, desc, body, id = '') => `<section class="sec"${id ? ` id="${id}"` : ''}><div class="sec__label"><h2>${title}</h2>${desc ? `<p>${desc}</p>` : ''}</div><div class="sec__body">${body}</div></section>`
const kv = (pairs) => `<dl class="kv">${pairs.map(([k, v]) => `<div><dt>${k}</dt><dd>${v}</dd></div>`).join('')}</dl>`
const state = (kind, text) => `<span class="state"><i class="dot dot--${kind}"></i>${text}</span>`

function overview() {
  const tile = (ic, title, kind, word, text) => `<a class="tile"><span class="tile__ic">${icon(ic)}</span><div class="tile__t">${title}</div><div class="tile__s">${state(kind, word)}</div><p>${text}</p></a>`
  return `<div class="tiles">
      ${tile('gamepad', '手柄', 'ok', '已就绪', '原生 DualSense 已内置在 Sunshine 中，当前没有手柄在使用。')}
      ${tile('microphone', '麦克风', 'idle', '等待使用', '开始串流并打开客户端麦克风后会自动创建。')}
      ${tile('plug', 'USB 设备', 'ok', '可用', 'USB/IP 传输 v0.9.8.1，0 台设备已挂载。')}
      ${tile('display', '虚拟显示器', 'ok', '已就绪', '每客户端专属 VDD，11 个分辨率，7 个刷新率。')}
    </div>` +
    sec('串流', '主机与客户端', kv([['主机', host.name], ['版本', `<span class="mono">${host.version}</span>`], ['已运行', host.uptime], ['已配对客户端', `${host.clients} 台`], ['串流应用', `${host.apps} 个`], ['当前串流', '空闲']])) +
    sec('维护', '组件与更新', kv([['组件', state('ok', '全部正常')], ['软件更新', '已是最新版本'], ['诊断工具', btn(icon('gamepad') + ' 手柄测试', '', 'btn--sm') + btn(icon('file-lines') + ' 查看日志', '', 'btn--sm')]]))
}

function gamepad() {
  const picks = `<div class="picks">${gamepadTypes.map((t) => `<div class="pick${t.id === 'ds5' ? ' is-on' : ''}" role="radio" aria-checked="${t.id === 'ds5'}"><i class="radio"></i><div><b>${t.title}</b><span>${t.desc}</span></div></div>`).join('')}</div><a class="link">每个游戏的单独设置 ${icon('chevron-right')}</a>`
  const field = (label, ctl) => `<div class="field"><span class="field__l">${label}</span><div class="field__c">${ctl}</div></div>`
  return sec('手柄类型', '串流时，游戏会把手柄识别为哪一种。', picks, 'type') +
    sec('原生 DualSense', 'DualSense 支持已内置在 Sunshine 主机中，这里显示的是主机和系统的实际状态。', kv(dsFacts.map(([k, v]) => [k, state(k === '手柄' ? 'idle' : 'ok', v)])) , 'status') +
    sec('预设', '一键套用常用手感，之后仍可逐项调整。', `<div class="presets">${presets.map((p) => `<div class="preset${p.on ? ' is-on' : ''}"><b>${p.title}</b><span>${p.desc}</span></div>`).join('')}</div>`, 'tuning') +
    sec('触觉与震动', '', field('触觉增益', slider(1)) + field('启用震动', sw(true)) + field('震动强度', slider(1))) +
    sec('灯光', '', field('灯条模式', seg(['跟随游戏', '固定', '关闭'], '跟随游戏')) + field('颜色', '<span class="muted">由游戏决定</span>')) +
    sec('高级', '另有 92 项设置，按功能分组。', `<div class="adv"><p>这些选项会改变虚拟手柄与 Windows、游戏之间的通信方式，填错数值可能引发难以追溯的问题。展开前会先说明风险。</p>${btn('展开高级选项 ' + icon('chevron-down'))}</div>`, 'advanced')
}

function microphone() {
  const field = (label, ctl) => `<div class="field"><span class="field__l">${label}</span><div class="field__c">${ctl}</div></div>`
  return sec('状态', '麦克风重定向', `<div class="statusline">${chip('idle', '等待使用')}<span>还没有使用中的麦克风。开始串流并打开客户端麦克风后会自动创建，也可以点“测试麦克风”。</span></div>` + kv([['当前使用', '暂无'], ['默认录音和播放设备', 'Windows 的默认设备不会被更改']]) + `<div class="actions">${btn('测试麦克风', 'primary')}${btn(icon('rotate') + ' 刷新')}</div>`) +
    sec('麦克风后端', '', `<div class="stack">${seg(micBackends, '自动')}<p class="muted">先使用本机自带的 USB 虚拟麦克风，失败时改用 VB-CABLE。</p></div>`) +
    sec('依赖组件', '', kv([['USB/IP 传输', state('ok', '已就绪 · usbip-win2')]]) + `<a class="link">管理组件 ${icon('chevron-right')}</a>`)
}

export const pages = { overview, gamepad, microphone }
export const render = (id, theme) => {
  const t = { overview: ['概览', ''], gamepad: ['手柄', tabs(['手柄类型', 'DualSense 状态', '调校', '高级'], '手柄类型')], microphone: ['麦克风', ''] }[id]
  return page('c', theme, id, shell(id, t[0], pages[id](), t[1]))
}
