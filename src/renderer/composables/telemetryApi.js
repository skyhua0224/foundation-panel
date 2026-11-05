import { sunshine } from '../tauri-adapter.js'

// The transport of the choice about usage counts and error information (useTelemetryConsent): the host's own config, through the proxy of the panel. Shared by the shell and the first-run flow.
const base = async () => String((await sunshine.getProxyUrl()) || '').replace(/\/$/, '')
export const telemetryApi = {
  getJson: async (path) => (await fetch(`${await base()}${path}`)).json(),
  postJson: async (path, body) => (await fetch(`${await base()}${path}`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) })).json(),
}
