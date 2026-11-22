import { test } from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { appIsWindowsOnly, linuxChecks, manifestChecks } from './dev-env.mjs'

const pins = readFileSync(new URL('../.devcontainer/apt-packages.lock', import.meta.url), 'utf8')
const version = name => pins.match(new RegExp(`^${name}=(.+)$`, 'm'))[1]
const good = (command, args) => command === 'dpkg-query' ? version(args.at(-1)) : args.at(-1) === 'libspa-0.2' ? '0.2' : version('libpipewire-0.3-dev').split('-')[0]

test('doctor accepts the paired locked PipeWire and SPA headers', () => {
  assert.ok(linuxChecks(good).every(c => c.ok))
})
test('doctor rejects newer PipeWire and provides a container repair command', () => {
  const checks = linuxChecks((command, args) => args.at(-1) === 'libpipewire-0.3' ? '1.6.8' : good(command, args))
  const failure = checks.find(c => !c.ok)
  assert.equal(failure.name, 'PipeWire pkg-config')
  assert.match(failure.fix, /just setup/)
})
test('doctor detects a mismatched SPA header package despite an unchanged ABI version', () => {
  const checks = linuxChecks((command, args) => command === 'dpkg-query' && args.at(-1) === 'libspa-0.2-dev' ? '1.6.8-1' : good(command, args))
  assert.equal(checks.filter(c => !c.ok).length, 1)
  assert.equal(checks.find(c => !c.ok).name, 'libspa-0.2-dev')
})
test('doctor reports missing development headers', () => {
  assert.ok(linuxChecks(() => '').every(c => !c.ok && c.fix.includes('just setup')))
})

for (const [name, mise, manager] of [
  ['Node pin drift', 'node = "22.0.0"\nrust = "1.100.0"', 'npm@11.19.1'],
  ['Rust pin drift', 'node = "24.22.3"\nrust = "1.99.0"', 'npm@11.19.1'],
  ['npm manager drift', 'node = "24.22.3"\nrust = "1.100.0"', 'npm@10.0.0'],
]) {
  test(`doctor rejects ${name}`, () => {
    const checks = manifestChecks({ engines: { node: '24.22.3', npm: '11.19.1' }, packageManager: manager }, '1.100.0', mise)
    assert.equal(checks.filter(c => c.actual !== c.expected).length, 1)
  })
}

test('the app stays Windows-only while its Windows crates are cfg(windows) dependencies', () => {
  assert.equal(appIsWindowsOnly(), true)
  assert.equal(appIsWindowsOnly("[dependencies]\nwindows = \"0.62\"\n"), false)
  assert.equal(appIsWindowsOnly("[dependencies]\nserde = \"1\"\n"), false)
  assert.equal(appIsWindowsOnly("[target.'cfg(windows)'.dependencies]\nwindows = \"0.62\"\nwebview2-com = \"0.38\"\n"), true)
})
