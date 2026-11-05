import { computed, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { SOURCE_KEY, createMirrorChyanClient, normalizeSource } from 'foundation-ui/update'

// The update source (automatic, CNB, GitHub, Mirror酱) and the Mirror酱 CDK of the 软件更新 page.
// The CDK is kept by the native side in the operating system's secure store and is used there for the HTTP call: this window only sets it once and asks about its state, it never reads it back.
// STUBS (to be built with the Windows build, see docs/update-source-design.md): the commands `update_cdk_present`, `update_cdk_set`, `update_cdk_clear`, `update_mirrorchyan_check` and
// `set_update_source` do not exist in the Rust side yet. While they do not, `available` is false: the source can be chosen (and is remembered here), the CDK field is disabled with a note.
const read = () => { try { return normalizeSource(localStorage.getItem(SOURCE_KEY)) } catch { return 'auto' } }
const source = ref(read())
const available = ref(true)
const hasCdk = ref(false)
const status = ref({ state: 'none', expiresAt: null, kind: '' })
// the last check that found the CDK valid: shown when a later check cannot reach the service
const lastValid = ref(null)
const setStatus = (value) => { status.value = value; if (value.state === 'valid') lastValid.value = { expiresAt: value.expiresAt } }
const busy = ref(false)
const call = async (command, args) => { try { return await invoke(command, args) } catch (error) { if (/not found|unknown|not allowed|command/i.test(String(error))) available.value = false; throw error } }

const client = createMirrorChyanClient({
  hasCdk: async () => hasCdk.value,
  transport: (args) => call('update_mirrorchyan_check', { currentVersion: args.currentVersion || '', channel: args.channel || 'stable' }),
})

export function useUpdateSource() {
  const setSource = async (value) => {
    source.value = normalizeSource(value)
    try { localStorage.setItem(SOURCE_KEY, source.value) } catch { /* not remembered */ }
    try { await call('set_update_source', { source: source.value }) } catch { /* the native side does not know it yet: it is kept here */ }
  }
  const refresh = async () => {
    try { hasCdk.value = !!(await call('update_cdk_present')); available.value = true } catch { hasCdk.value = false }
    setStatus(available.value ? await client.cdkStatus() : { state: 'none', expiresAt: null, kind: '' })
  }
  const saveCdk = async (value) => {
    busy.value = true
    try { await call('update_cdk_set', { cdk: String(value || '').trim() }); hasCdk.value = true; await refresh(); return true } catch { return false } finally { busy.value = false }
  }
  const clearCdk = async () => {
    busy.value = true
    try { await call('update_cdk_clear'); hasCdk.value = false; lastValid.value = null; status.value = { state: 'none', expiresAt: null, kind: '' }; return true } catch { return false } finally { busy.value = false }
  }
  const checkCdk = async () => { busy.value = true; try { setStatus(await client.cdkStatus()) } finally { busy.value = false } }
  return { source: computed(() => source.value), available: computed(() => available.value), hasCdk: computed(() => hasCdk.value), status: computed(() => status.value), lastValid: computed(() => lastValid.value), busy: computed(() => busy.value), setSource, refresh, saveCdk, clearCdk, checkCdk }
}
