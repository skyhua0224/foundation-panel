import { reactive, ref } from 'vue'
import { sunshine } from '../tauri-adapter.js'
import { diagnosisRequestBody, openLogDiagnosis } from 'foundation-ui/diagnostics'

// The log assistant of 诊断与日志 in the control panel. The request goes to the host through the local proxy, the same routes the Web UI uses.
// Local findings come from the shared diagnosis path before any request, and the request body is redacted there.
const DEFAULT_CONFIG = { enabled: false, provider: 'openai', apiBase: '', apiKey: '', apiKeyConfigured: false, model: '', compatibility: 'openai-chat', temperature: 0.3, max_tokens: 2048 }

const proxy = async () => String((await sunshine.getProxyUrl()) || '').replace(/\/$/, '')
async function fetchJson(path, options) {
  const response = await fetch(`${await proxy()}${path}`, options)
  const data = await response.json().catch(() => ({}))
  if (!response.ok || data.status === 'error') {
    const message = typeof data.error === 'string' ? data.error : data.error?.message
    throw new Error(message || `HTTP ${response.status}`)
  }
  return data
}

export function useLogDiagnosis() {
  const config = reactive({ ...DEFAULT_CONFIG })
  const isConfigLoading = ref(false)
  const isSavingConfig = ref(false)
  const isLoading = ref(false)
  const result = ref('')
  const error = ref('')
  const show = ref(false)
  const localFindings = ref([])
  const localSuggestions = ref([])

  async function loadConfig() {
    isConfigLoading.value = true
    error.value = ''
    try { Object.assign(config, { ...DEFAULT_CONFIG, ...(await fetchJson('/api/ai/config')) }) }
    catch (e) { error.value = e.message }
    finally { isConfigLoading.value = false }
  }

  function invalid() {
    if (!config.enabled) return 'Please enable the local AI proxy first.'
    if (!config.apiBase) return 'Please configure an API base first.'
    if (!config.model) return 'Please configure a model first.'
    const apiBase = config.apiBase || ''
    const keyRequired = config.provider !== 'ollama' && !apiBase.includes('localhost') && !apiBase.includes('127.0.0.1') && !apiBase.includes('[::1]')
    if (!(config.apiKeyConfigured || config.apiKey) && keyRequired) return 'Please configure an API key first.'
    return ''
  }

  async function open() { show.value = true; await loadConfig() }
  function close() { show.value = false }

  async function diagnose(logs) {
    if (!logs) { error.value = 'No log content is available.'; return }
    const prepared = openLogDiagnosis(logs)
    localFindings.value = prepared.findings
    localSuggestions.value = prepared.suggestions
    await loadConfig()
    const problem = invalid()
    if (problem) { error.value = problem; return }
    isLoading.value = true
    result.value = ''
    error.value = ''
    try {
      const data = await fetchJson('/api/ai/chat/completions', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(diagnosisRequestBody({ model: config.model, requestText: prepared.requestText, temperature: config.temperature, maxTokens: config.max_tokens })),
      })
      result.value = data.choices?.[0]?.message?.content || 'Unable to read the analysis result.'
    } catch (e) { error.value = e.message }
    finally { isLoading.value = false }
  }

  return { config, isConfigLoading, isSavingConfig, isLoading, result, error, show, localFindings, localSuggestions, open, diagnose, close }
}
