import { ElMessage, ElMessageBox } from './notifyMessage.js'
import { open as openDialog } from '@tauri-apps/plugin-dialog'
import { invoke } from '@tauri-apps/api/core'

// Sparxie is installed from a ZIP the person already has. There is no download.
// A known checksum is silent; any other package asks before it is installed.

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms))
// the host's English sentence is for logs: the person reads the panel's own words for its error code
const reason = (texts, result) => texts?.sparxie?.errors?.[result?.error_code] || ''
const stageText = (texts, stage) => texts?.sparxie?.stages?.[stage] || stage || ''

export async function postJson(url, body, timeoutMs = 120000) {
  const response = await fetch(url, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
    signal: AbortSignal.timeout(timeoutMs),
  })
  const payload = await response.json().catch(() => ({}))
  if (!response.ok && payload && payload.ok !== true) return { ...payload, ok: false, http: response.status }
  return payload
}

export async function getJson(url) {
  const response = await fetch(url, { signal: AbortSignal.timeout(15000) })
  return response.json().catch(() => ({}))
}

export async function runInstall(base, body, onStage) {
  const started = await postJson(base + '/api/sparxie/install', body)
  if (!started || started.ok === false || started.done || !started.job) return started
  const deadline = Date.now() + 15 * 60 * 1000
  let last = started
  while (Date.now() < deadline) {
    await sleep(700)
    last = await getJson(base + '/api/sparxie/install/' + started.job)
    if (onStage) onStage(last)
    if (last && last.done) return last
  }
  return { ok: false, error_code: 'busy', error: 'The install did not finish' }
}

// `progress(value | null)` puts the install's stage on the card and takes it off; it is the card's own state, never part of a read of the host's status.
export function sparxieActions({ base, texts, refresh, progress = () => {} }) {
  const T = () => texts().sparxie
  const busy = { on: false }
  const guard = async (work) => { if (busy.on) return; busy.on = true; try { await work() } finally { busy.on = false } }
  return {
    import: {
      label: (S, row) => (!row || row.state === 'missing' || row.state === 'unknown' ? S.sparxie.fromFile : S.sparxie.update),
      run: () => guard(async () => {
        const root = await base()
        const status = await getJson(root + '/api/sparxie/status')
        if (!status || status.noticeAck !== true) {
          const statement = (status && status.statement) || T().bring
          try { await ElMessageBox.confirm(statement, T().noticeTitle, { type: 'warning', confirmButtonText: T().noticeYes }) } catch { return }
        }
        const picked = await openDialog({ multiple: false, directory: false, title: T().fromFile, filters: [{ name: 'ZIP', extensions: ['zip'] }] })
        if (!picked) return
        const send = (confirmUnverified) => runInstall(root, { path: picked, confirm_unverified: !!confirmUnverified, acknowledged: true }, (row) => {
          if (!row || !row.stage || row.done === true) return
          progress({ stage: stageText(texts(), row.stage), percent: Number(row.percent) || 0 })
        })
        // the line goes away however the install ends: done, refused, cancelled, or a request that failed
        try {
          progress({ stage: stageText(texts(), 'dependencies'), percent: 0 })
          let result = await send(false)
          if (result && (result.error_code === 'unverified' || result.error_code === 'hash_mismatch')) {
            progress(null)
            try { await ElMessageBox.confirm(T().unverified + '\n' + (result.sha256 || ''), T().unverifiedTitle, { type: 'warning', confirmButtonText: T().unverifiedYes }) } catch { return }
            progress({ stage: stageText(texts(), 'dependencies'), percent: 0 })
            result = await send(true)
          }
          if (!result || result.ok === false) ElMessage.error((T().failed + ' ' + reason(texts(), result)).trim())
          else ElMessage.success(T().done)
        } catch (error) {
          ElMessage.error(T().failed)
        } finally {
          progress(null)
          await refresh()
        }
      }),
    },
    // Sparxie is a window of the person's session: the Control Panel starts it itself (a process of that session); the host service never does
    open: {
      label: (S) => S.sparxie.open,
      run: () => guard(async () => {
        const root = await base()
        const status = await getJson(root + '/api/sparxie/status')
        if (!status || status.installed === false || !status.exe) { ElMessage.error(T().openFailed); return }
        if (status.running) { ElMessage.info(T().alreadyOpen); return }
        try { await invoke('open_sparxie', { exe: status.exe }) } catch (error) { ElMessage.error(T().openFailed) }
        await refresh()
      }),
    },
    remove: {
      label: (S) => S.uninstall.yes,
      run: () => guard(async () => {
        const root = await base()
        let result = await postJson(root + '/api/sparxie/uninstall', {})
        if (result && result.error_code === 'confirm_config') {
          let deleteConfig = false
          try {
            await ElMessageBox.confirm(T().removeText, T().removeTitle, { type: 'warning', distinguishCancelAndClose: true, confirmButtonText: T().removeDelete, cancelButtonText: T().removeKeep })
            deleteConfig = true
          } catch (answer) {
            if (answer === 'cancel') deleteConfig = false
            else return
          }
          result = await postJson(root + '/api/sparxie/uninstall', { delete_config: deleteConfig })
        }
        if (!result || result.ok === false) ElMessage.error((T().failed + ' ' + reason(texts(), result)).trim())
        else ElMessage.success(T().removed)
        await refresh()
      }),
    },
  }
}
