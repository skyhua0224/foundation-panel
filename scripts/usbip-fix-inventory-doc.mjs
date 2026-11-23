// Writes docs/windows/usbip-fix.zh.md from the inventory of the one-click USB/IP repair, so the document says what the code does:
//   node scripts/usbip-fix-inventory-doc.mjs          write the file
//   node scripts/usbip-fix-inventory-doc.mjs --check  fail when the file is not what the inventory gives (the renderer tests run this)
import { readFileSync, writeFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { USBIP_FIX_INVENTORY } from '../src/renderer/composables/usbipFixInventory.js'

export const DOC_PATH = fileURLToPath(new URL('../docs/windows/usbip-fix.zh.md', import.meta.url))
const cell = (text) => String(text).replace(/\|/g, '\\|').replace(/\n/g, ' ')

export function renderInventoryDoc() {
  const rows = USBIP_FIX_INVENTORY.map((row) => `| \`${row.id}\` | ${row.codes.length ? row.codes.map((code) => `\`${cell(code)}\``).join('、') : '—'} | ${cell(row.detect)} | ${cell(row.repair)} | ${cell(row.confirm)} | ${cell(row.reboot)} | ${cell(row.verify)} |`)
  return `# USB/IP 一键修复：状态清单

此文件由 \`scripts/usbip-fix-inventory-doc.mjs\` 从 \`src/renderer/composables/usbipFixInventory.js\` 生成，不要手改。清单里每一行都有测试（\`usbipFix.test.js\`）：能由状态判定的行，测试检查修复计划；只在修复过程中出现的行，测试检查它的错误给出哪份处理步骤。

一个按钮、一条流程：读取状态并判定 → 用大白话列出会结束、释放、移除的东西（只问一次）→ 结束串流 → 按顺序修复（释放已挂载端口、清理共享传输或只移除多余控制器、替换用户自己装的完整版、安装配套的 USB/IP）→ 检查结果（传输就绪，且 DualSense 原生通道不再报告 USB/IP 问题）→ 需要重启时说明并提供重启，重启后继续剩下的步骤。软件无法处理的情况（系统策略拦截、根集线器回滚、系统盘在 USB 上、用户自己的安装不允许替换）给出一步一步的处理说明，并保留原始诊断。

| 行 | 代码或状态 | 怎么发现 | 修复做什么 | 确认里写什么 | 重启 | 怎么验证 |
| --- | --- | --- | --- | --- | --- | --- |
${rows.join('\n')}
`
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  if (process.argv.includes('--check')) {
    if (readFileSync(DOC_PATH, 'utf8') !== renderInventoryDoc()) {
      console.error('docs/windows/usbip-fix.zh.md is out of date: run node scripts/usbip-fix-inventory-doc.mjs')
      process.exit(1)
    }
  } else {
    writeFileSync(DOC_PATH, renderInventoryDoc())
  }
}
