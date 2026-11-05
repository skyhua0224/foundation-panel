// Realistic state shared by every prototype: the values come from the panel's own screenshots (Skyindows host).
export const host = { name: 'Skyindows', version: '2026.1101.141735', uptime: '3 小时 12 分', clients: 30, apps: 4 }

// One information architecture for all three directions (see the proposal).
export const nav = [
  { id: 'stream', label: '串流', items: [
    { id: 'overview', label: '概览', icon: 'house' },
    { id: 'clients', label: '客户端', icon: 'link' },
    { id: 'apps', label: '串流应用', icon: 'table-cells' },
    { id: 'settings', label: '串流设置', icon: 'sliders' },
  ] },
  { id: 'input', label: '输入与外设', items: [
    { id: 'gamepad', label: '手柄', icon: 'gamepad', dot: 'ok' },
    { id: 'microphone', label: '麦克风', icon: 'microphone', dot: 'idle' },
    { id: 'usb', label: 'USB 设备', icon: 'plug', dot: 'ok' },
  ] },
  { id: 'display', label: '显示', items: [
    { id: 'vdd', label: '虚拟显示器', icon: 'display', dot: 'ok' },
    { id: 'quality', label: '画质增强', icon: 'wand-magic-sparkles' },
  ] },
  { id: 'system', label: '系统', items: [
    { id: 'components', label: '组件', icon: 'puzzle-piece' },
    { id: 'diagnostics', label: '诊断工具', icon: 'stethoscope' },
    { id: 'assistant', label: 'AI 助手', icon: 'wand-magic' },
  ] },
]
export const footerNav = [
  { id: 'panel', label: '面板设置', icon: 'gear' },
  { id: 'help', label: '帮助与关于', icon: 'circle-question' },
]

export const gamepadTypes = [
  { id: 'auto', title: '自动（推荐）', desc: '不确定时选这个，优先采用兼容性最好的输出。' },
  { id: 'x360', title: 'Xbox 360', desc: '兼容大多数 Windows 游戏，适合优先考虑兼容性的场景。' },
  { id: 'ds4', title: 'DualShock 4', desc: '适合需要 PlayStation 按键布局、触控板或体感的游戏。' },
  { id: 'ds5', title: 'DualSense（DS5 / PS5 手柄）', desc: '模拟 PlayStation 5 手柄，支持触控板、体感与自适应扳机；由 Sunshine 内置的原生通道提供，状态与调校见下方。' },
]
export const dsFacts = [
  ['原生 DualSense', '已内置在 Sunshine 中'],
  ['USB/IP 传输', '0.9.8.1，可用'],
  ['驱动签名', '已签名 · Microsoft Windows Hardware Compatibility Publisher'],
  ['手柄', '当前没有手柄在使用'],
]
export const presets = [
  { title: '默认', desc: '标准手感', icon: 'rotate-left', on: true },
  { title: '竞技', desc: '更紧的震动', icon: 'bolt' },
  { title: '沉浸', desc: '更强的触觉', icon: 'wave-square' },
  { title: '旧版', desc: '仅普通震动', icon: 'clock-rotate-left' },
]
export const micBackends = ['自动', 'VB-CABLE', 'USB 虚拟麦克风', '禁用']
