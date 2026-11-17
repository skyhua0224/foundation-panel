import test from 'node:test'
import assert from 'node:assert/strict'
import { withUsbipOwnership } from './usbipOwnership.js'

for (const locale of ['en', 'zh', 'zh_TW', 'de']) {
  test(`USB/IP ownership messages are additive and preserve shared locale tables (${locale})`, () => {
    const original = { deviceHub: { usb: { transportReady: 'ready' }, components: { maintenance: 'manage' } }, other: 'keep' }
    const table = withUsbipOwnership(original, locale)
    assert.equal(table.other, 'keep')
    assert.equal(table.deviceHub.usb.transportReady, 'ready')
    assert.equal(table.deviceHub.components.maintenance, 'manage')
    assert.ok(table.deviceHub.usb.externallyManaged)
    assert.ok(table.deviceHub.usb.externalDecision.includes('0.9.8.1'))
    assert.ok(table.deviceHub.usb.externalReuse)
    assert.equal(original.deviceHub.usb.externallyManaged, undefined)
    assert.equal(withUsbipOwnership(original, 'en').deviceHub.usb.externallyManaged, 'Externally managed')
  })
}
