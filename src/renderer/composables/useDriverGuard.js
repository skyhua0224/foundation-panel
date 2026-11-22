import { onBeforeUnmount, onMounted, ref } from 'vue'
import { createDriverGuardClient } from 'foundation-ui/driverGuard'
import { hostFetch } from './hostBase.js'

// The state of the graphics driver guard of the host (GET /api/gpu-driver-guard): the safe mode, the encoder check that runs after the start, and the notice that
// the last failure lay in a graphics driver. The overview draws it with MDriverGuardNotice (foundation-ui, the same component as the Web UI); this file only reads and answers.

/** The two calls over the panel's proxy to the host; a status other than 2xx is an error (an older host without the route answers 404). */
export function driverGuardApi(base, fetcher = hostFetch) {
  const json = async (response) => {
    if (!response.ok) throw Object.assign(new Error(`HTTP ${response.status}`), { status: response.status })
    return response.json()
  }
  return {
    get: async () => json(await fetcher(base, '/api/gpu-driver-guard')),
    post: async (body) => json(await fetcher(base, '/api/gpu-driver-guard', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) })),
  }
}

export function useDriverGuard(base) {
  const guard = ref(null)
  let timer = 0
  let closed = false
  const client = createDriverGuardClient(driverGuardApi(base), (next) => { guard.value = next })
  // one place makes the timer: an older one is cleared first, and nothing runs after the unmount
  const schedule = () => {
    clearTimeout(timer)
    if (!closed) timer = setTimeout(loop, client.nextDelay())
  }
  const loop = async () => {
    if (closed) return
    await client.refresh()
    schedule()
  }
  onMounted(loop)
  onBeforeUnmount(() => { closed = true; clearTimeout(timer) })
  // an answer to a button is a new state: the next look comes soon, the host may be checking the encoders now
  const answer = (call) => async (...args) => { try { await call(...args) } finally { schedule() } }
  return { guard, dismiss: answer(client.dismiss), retry: answer(client.retry) }
}
