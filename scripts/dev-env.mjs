// dev-env-mise, dev-env-npm, dev-env-gtk: version selection and platform prerequisites.
import { spawnSync } from 'node:child_process'
import { readFileSync, mkdirSync, existsSync } from 'node:fs'
import { join, resolve, dirname } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'
import { tmpdir, platform } from 'node:os'
import { createHash } from 'node:crypto'

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const pkg = JSON.parse(readFileSync(join(root, 'package.json')))
const miseConfig = readFileSync(join(root, 'mise.toml'), 'utf8')
const toolPin = name => miseConfig.split('\n').find(line => line.split(' = ')[0] === name)?.split(' = ')[1]?.replaceAll('"', '') || ''
const toolchain = readFileSync(join(root, 'rust-toolchain.toml'), 'utf8').match(/channel = "([^"]+)"/)[1]
const cache = process.env.PANEL_DEPENDENCY_DIR || join(tmpdir(), `panel-deps-${createHash('sha256').update(root).digest('hex').slice(0, 12)}`)
const env = { ...process.env, CARGO_TARGET_DIR: process.env.CARGO_TARGET_DIR || join(cache, 'target') }
const windows = platform() === 'win32'

function run(command, args = [], options = {}) {
  const result = spawnSync(command, args, { cwd: root, env, stdio: 'inherit', ...options })
  if (result.error) throw new Error(`${command}: ${result.error.message}`)
  if (result.status !== 0) throw new Error(`${command} exited ${result.status}`)
  return result.stdout?.trim() || ''
}
function read(command, args) {
  const probeEnv = command.startsWith('corepack') ? { ...env, COREPACK_ENABLE_NETWORK: '0' } : env
  const result = spawnSync(command, args, { cwd: root, env: probeEnv, encoding: 'utf8', shell: windows && command.endsWith('.cmd') })
  return result.status === 0 ? result.stdout.trim() : ''
}
const npm = (...args) => run(windows ? 'corepack.cmd' : 'corepack', ['npm', ...args], { shell: windows })

// libspa keeps the pkg-config ABI version 0.2 across releases. Check the actual
// header package as well: dev-env-libspa08, system-deps; dev-env-pipewire-native, SPA.
export function linuxChecks(probe = read) {
  const pins = readFileSync(join(root, '.devcontainer/apt-packages.lock'), 'utf8')
  const expectedPackage = name => pins.match(new RegExp(`^${name}=(.+)$`, 'm'))?.[1]
  const checks = []
  for (const name of pins.split('\n').filter(line => line && !line.startsWith('#')).map(line => line.split('=')[0])) {
    checks.push([name, probe('dpkg-query', ['-W', '-f=${Version}', name]), expectedPackage(name)])
  }
  const pw = expectedPackage('libpipewire-0.3-dev').split('-')[0]
  checks.push(['PipeWire pkg-config', probe('pkg-config', ['--modversion', 'libpipewire-0.3']), pw])
  checks.push(['SPA pkg-config', probe('pkg-config', ['--modversion', 'libspa-0.2']), '0.2'])
  return checks.map(([name, actual, expected]) => ({ name, actual, expected,
    ok: actual === expected, fix: 'just setup (rebuild the pinned Dev Container); run just doctor inside it' }))
}

export function manifestChecks(packageData = pkg, rust = toolchain, mise = readFileSync(join(root, 'mise.toml'), 'utf8')) {
  const pin = name => mise.match(new RegExp(`^${name} = "([^"\\n]+)"`, 'm'))?.[1] || ''
  return [
    { name: 'mise Node pin', actual: pin('node'), expected: packageData.engines.node, fix: 'Align mise.toml with package.json, then mise install' },
    { name: 'mise Rust pin', actual: pin('rust'), expected: rust, fix: 'Align mise.toml with rust-toolchain.toml, then mise install' },
    { name: 'npm manager pin', actual: packageData.packageManager, expected: `npm@${packageData.engines.npm}`, fix: 'Align packageManager with engines.npm, then corepack install' },
  ]
}

// The app is a Windows program: its Cargo manifest declares the `windows` and `webview2-com` crates only for
// cfg(windows) and the sources call them unconditionally, so a Linux or macOS build stops at "cannot find crate
// `windows`" (43 errors, the same on the unmodified main). Not a missing tool or header. A test keeps this note
// honest: when the manifest stops being Windows-only the test fails and the skip has to go.
export function appIsWindowsOnly(manifest = readFileSync(join(root, 'src-tauri/Cargo.toml'), 'utf8')) {
  let table = ''
  const where = new Map()
  for (const line of manifest.split('\n')) {
    const header = line.match(/^\[(.+)\]\s*$/)
    if (header) table = header[1]
    const crate = line.match(/^(windows|webview2-com)\s*=/)
    if (crate) where.set(crate[1], table)
  }
  return where.size > 0 && [...where.values()].every(t => t === "target.'cfg(windows)'.dependencies")
}
function appPlatformNote() {
  if (!appIsWindowsOnly()) throw new Error('src-tauri/Cargo.toml no longer limits the Windows crates to cfg(windows): remove the app skip in scripts/dev-env.mjs and test the app here')
  return 'the app depends on Windows-only crates (windows, webview2-com); build and test it on Windows (the Build workflow does)'
}

export function doctor() {
  const checks = [
    ...manifestChecks(),
    { name: 'Corepack', actual: read(windows ? 'corepack.cmd' : 'corepack', ['--version']), expected: toolPin('"npm:corepack"'), fix: 'mise install' },
    { name: 'just', actual: read('just', ['--version']).split(' ')[1] || '', expected: toolPin('just'), fix: 'mise install' },
    { name: 'Node', actual: process.versions.node, expected: pkg.engines.node, fix: 'mise install' },
    { name: 'npm', actual: read(windows ? 'corepack.cmd' : 'corepack', ['npm', '--version']), expected: pkg.engines.npm, fix: 'corepack install' },
    { name: 'Rust', actual: read('rustc', ['--version']).split(' ')[1] || '', expected: toolchain, fix: 'mise install' },
  ]
  if (platform() === 'linux') checks.push(...linuxChecks())
  if (platform() === 'darwin') checks.push({ name: 'Xcode tools', actual: read('xcode-select', ['-p']) ? 'present' : '', expected: 'present', fix: 'xcode-select --install' })
  if (windows) {
    checks.push({ name: 'MSVC developer environment', actual: process.env.VCToolsInstallDir ? 'present' : '', expected: 'present', fix: 'winget configure -f configuration.dsc.yaml; open a Developer PowerShell' })
  }
  let failures = 0
  for (const c of checks) {
    const ok = c.actual === c.expected
    console.log(`${ok ? 'OK' : 'FAIL'} ${c.name}: ${c.actual || 'missing'} (expected ${c.expected})${ok ? '' : `; fix: ${c.fix}`}`)
    if (!ok) failures++
  }
  if (failures) throw new Error(`doctor: ${failures} check(s) failed`)
}

function settings() {
  const sha = readFileSync(join(root, 'scripts/ci/foundation-sunshine.sha'), 'utf8').trim()
  const dir = join(cache, `settings-${sha}`)
  if (!existsSync(join(dir, '.git'))) {
    mkdirSync(dir, { recursive: true })
    run('git', ['init', dir])
  }
  const git = (...args) => run('git', ['-C', dir, ...args])
  if (read('git', ['-C', dir, 'rev-parse', 'HEAD']) !== sha) {
    git('fetch', '--depth', '1', '--no-tags', 'https://github.com/AlkaidLab/foundation-sunshine.git', sha)
    git('sparse-checkout', 'set', 'src_assets/common/assets/web/config/settingsData')
    git('checkout', '--detach', sha)
  }
  if (read('git', ['-C', dir, 'status', '--porcelain'])) throw new Error('Pinned settings cache has local changes; remove its task cache and rerun just setup')
  env.SETTINGS_DATA_DIR = join(dir, 'src_assets/common/assets/web/config/settingsData')
  if (!existsSync(join(env.SETTINGS_DATA_DIR, 'index.js'))) throw new Error('Pinned settings index missing; rerun just setup')
}

function container(action) {
  const imageHash = createHash('sha256')
  for (const file of ['.devcontainer/Dockerfile', '.devcontainer/apt-packages.lock', 'mise.toml', 'rust-toolchain.toml']) imageHash.update(readFileSync(join(root, file)))
  const image = `foundation-panel-dev:${imageHash.digest('hex').slice(0, 20)}`
  const containerCache = join(cache, 'container')
  if (action === 'setup') mkdirSync(join(containerCache, 'tmp'), { recursive: true })
  if (action === 'setup') run('docker', ['build', '--platform=linux/amd64', '--file', '.devcontainer/Dockerfile', '--tag', image, '.'])
  run('docker', ['run', '--platform=linux/amd64', '--rm', '--init', '--mount', `type=bind,source=${root},target=/workspaces/panel`,
    '--mount', `type=bind,source=${containerCache},target=/workspaces/.cache`,
    '--env', 'CARGO_HOME=/workspaces/.cache/cargo',
    '--env', `CARGO_BUILD_JOBS=${process.env.CARGO_BUILD_JOBS || '2'}`,
    '--workdir', '/workspaces/panel', image, 'just', action])
}

function main(action, ...extra) {
  if (action === 'package' && !windows) throw new Error('package builds the Windows installer: run it on Windows')
  if (!['doctor', 'doctor-native'].includes(action)) mkdirSync(cache, { recursive: true })
  if (platform() === 'linux' && !process.env.PANEL_DEV_CONTAINER && action !== 'doctor-native') return container(action)
  if (action === 'setup') {
    if (windows && !process.env.VCToolsInstallDir) {
      run('winget', ['configure', '-f', 'configuration.dsc.yaml', '--accept-configuration-agreements'])
      // A child process cannot change this process's environment: the MSVC variables exist only in a new Developer PowerShell.
      console.log('MSVC build tools are installed. Open a new Developer PowerShell and run just setup again to finish.')
      return
    }
    if (platform() === 'darwin') run('brew', ['bundle', '--file', 'Brewfile'])
    run(windows ? 'corepack.cmd' : 'corepack', ['install'], { shell: windows })
    doctor()
    npm('ci')
    settings()
    return
  }
  doctor()
  if (['doctor', 'doctor-native'].includes(action)) return
  settings()
  if (action === 'build') {
    npm('run', 'build:renderer')
    if (!windows) {
      console.log(`SKIP Tauri app build: ${appPlatformNote()}`)
      return
    }
    run('powershell.exe', ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', 'scripts/build-native-plugins.ps1', '-Configuration', 'Debug'])
    run('cargo', ['build', '--locked', '--manifest-path', 'src-tauri/Cargo.toml'])
    return
  }
  if (action === 'test') {
    run('node', ['--test', 'scripts/dev-env.test.mjs'])
    npm('run', 'test:renderer')
    const nativeTests = ['test', '--locked', '--manifest-path', 'native-plugins/Cargo.toml']
    if (!windows) {
      console.log('SKIP Windows stylus plugin tests: requires Windows; testing the portable plugin API')
      nativeTests.push('-p', 'alkaidlab-native-tool-plugin-api')
    }
    run('cargo', nativeTests)
    if (windows) run('cargo', ['test', '--locked', '--manifest-path', 'src-tauri/Cargo.toml'])
    else console.log(`SKIP Tauri app tests: ${appPlatformNote()}`)
    return
  }
  if (action === 'package') {
    // The installer script builds the renderer unless -SkipBuild is given; running it here keeps the pinned settings data (settings()).
    run('powershell.exe', ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', 'scripts/build-gui-component-installer.ps1', ...extra])
    return
  }
  throw new Error(`Unknown action: ${action}`)
}
if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  try { main(...process.argv.slice(2)) } catch (error) { console.error(error.message); process.exitCode = 1 }
}
