import test from 'node:test'
import assert from 'node:assert/strict'
import { withUsbipOwnership } from './usbipOwnership.js'

for (const locale of ['en', 'zh', 'zh_TW', 'de']) {
  test(`USB/IP ownership messages are additive and preserve shared locale tables (${locale})`, () => {
    const original = { deviceHub: { usb: { transportReady: 'ready', errors: { 'USBIP-SETUP-001': 'kept' } }, components: { maintenance: 'manage' } }, other: 'keep' }
    const table = withUsbipOwnership(original, locale)
    assert.equal(table.other, 'keep')
    assert.equal(table.deviceHub.usb.transportReady, 'ready')
    assert.equal(table.deviceHub.components.maintenance, 'manage')
    assert.ok(table.deviceHub.usb.externallyManaged)
    assert.ok(table.deviceHub.usb.externalDecision.includes('0.9.8.1'))
    assert.ok(table.deviceHub.usb.externalReuse)
    assert.equal(table.deviceHub.usb.errors['USBIP-SETUP-001'], 'kept')
    for (const key of ['replaceLegacy', 'replaceConfirm', 'replaceSuccess', 'externalReplaceable', 'installRestartPending', 'resumeInstall', 'lastInstallError']) assert.ok(table.deviceHub.usb[key], key)
    for (const code of ['001', '002', '003', '004', '005']) assert.ok(table.deviceHub.usb.errors[`USBIP-REPLACE-${code}`], code)
    for (const code of ['007', '010', '011', '012', '013', '014', '015', '016', '017']) assert.ok(table.deviceHub.usb.errors[`USBIP-SETUP-${code}`], code)
    assert.equal(original.deviceHub.usb.externallyManaged, undefined)
    assert.equal(withUsbipOwnership(original, 'en').deviceHub.usb.externallyManaged, 'Externally managed')
  })
}
