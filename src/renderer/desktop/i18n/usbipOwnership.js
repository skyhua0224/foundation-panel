const en = {
  externallyManaged: 'Externally managed',
  externalReuse: 'Using your compatible USB/IP installation. You manage its installation, repair and removal.',
  externalDecision: 'Your USB/IP installation is preserved. Only 0.9.8.1 is supported. If its version or driver is incompatible, choose whether to replace or repair it in Windows Installed apps.',
}
const zh = {
  externallyManaged: '外部管理',
  externalReuse: '使用现有的兼容 USB/IP 安装；安装、修复和卸载均由您管理。',
  externalDecision: '现有 USB/IP 安装将保留。仅支持 0.9.8.1；版本或驱动不兼容时，请自行决定是否先在 Windows「已安装的应用」中替换或修复。',
}
const zh_TW = {
  externallyManaged: '外部管理',
  externalReuse: '使用現有的相容 USB/IP 安裝；安裝、修復和解除安裝均由您管理。',
  externalDecision: '現有 USB/IP 安裝將保留。僅支援 0.9.8.1；版本或驅動程式不相容時，請自行決定是否先在 Windows「已安裝的應用程式」中替換或修復。',
}

/** Extend the small capability vocabulary without growing the legacy locale tables. */
export function withUsbipOwnership(table, locale) {
  const messages = locale === 'zh' ? zh : locale === 'zh_TW' ? zh_TW : en
  return {
    ...table,
    deviceHub: { ...table.deviceHub, usb: { ...table.deviceHub?.usb, ...messages } },
  }
}
