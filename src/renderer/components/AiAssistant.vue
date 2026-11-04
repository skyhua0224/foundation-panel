<template>
  <!-- 米塔 AI is a destination of the rail: the chat takes the whole pane (header, the conversation that scrolls, the input pinned at the bottom);
       its settings are pages behind the gear of the header -->
  <div v-if="!current" class="ai-shell">
    <header class="ai-head" :class="{ 'is-scrolled': scrolled }">
      <div class="ai-head__id">
        <h2 class="ai-head__title">{{ t.aiAssistant.title }}</h2>
        <p class="ai-head__line">{{ headLine }}</p>
      </div>
      <div class="ai-head__actions">
        <MIconButton v-if="chatHistory.length" icon="delete" :label="t.aiAssistant.clearHistory" @click="clearHistory" />
        <MIconButton icon="settings" :label="t.aiAssistant.modelConfig" @click="open('config')" />
      </div>
    </header>

    <div ref="chatContainer" class="ai-scroll" aria-live="polite" @wheel="handleChatWheel" @scroll.passive="scrolled = $event.target.scrollTop > 2">
      <div class="ai-col" data-wp-cover>
        <div v-if="chatHistory.length === 0 && !isLoading" class="ai-hint">
          <img src="/mita-pixel.png" :alt="t.aiAssistant.mitaName" class="ai-avatar ai-avatar--lg ai-avatar--pixel">
          <b>{{ t.aiAssistant.welcomeMsg }}</b>
          <MButton v-if="!config.enabled" variant="tonal" icon="settings" @click="open('config')">{{ t.aiAssistant.modelConfig }}</MButton>
          <div class="ai-examples">
            <button v-for="example in exampleQueries" :key="example" type="button" class="ai-example" :disabled="!config.enabled" @click="selectExample(example)">{{ example }}</button>
          </div>
        </div>
        <div v-for="(msg, idx) in chatHistory" :key="idx" class="ai-msg" :class="msg.role">
          <span v-if="msg.role === 'user'" class="ai-avatar ai-avatar--user"><MIcon name="person" /></span>
          <img v-else src="/mita-pixel.png" :alt="t.aiAssistant.mitaName" class="ai-avatar ai-avatar--pixel">
          <div class="ai-bubble">
            <div class="ai-text" v-html="formatMessage(msg.content, msg.parsedAction)"></div>
            <div v-if="msg.parsedAction" class="ai-action">
              <MButton size="sm" variant="tonal" @click="applyAction(msg.parsedAction)">{{ getActionButtonText(msg.parsedAction) }}</MButton>
              <span>{{ getActionHint(msg.parsedAction) }}</span>
            </div>
            <span class="ai-time">{{ formatTime(msg.timestamp) }}</span>
            <MButton v-if="msg.isError" size="sm" variant="text" icon="refresh" @click="retryLastMessage">{{ t.aiAssistant.retry }}</MButton>
          </div>
        </div>
        <div v-if="isLoading" class="ai-msg assistant"><img src="/mita-pixel.png" :alt="t.aiAssistant.mitaName" class="ai-avatar ai-avatar--pixel"><div class="ai-bubble"><MLoading size="28px" /></div></div>
      </div>
    </div>

    <div class="ai-dock">
      <div class="ai-composer">
        <textarea ref="chatInput" v-model="currentInput" rows="1" :placeholder="t.aiAssistant.inputPlaceholder" :disabled="!config.enabled" :aria-label="t.aiAssistant.inputPlaceholder" @keydown.enter.exact.prevent="handleSend"></textarea>
        <MButton icon="send" :loading="isLoading" :disabled="!config.enabled || !currentInput.trim()" @click="handleSend">{{ t.aiAssistant.send }}</MButton>
      </div>
    </div>
  </div>

  <MPage v-else :title="pageTitle" back :back-label="t.m3e.back" @back="back">
    <!-- Settings: a short page (switch and three rows), the connection and the model are pages of their own -->
    <template v-if="current === 'config'">
      <MGroup>
        <MSwitchItem v-model="config.enabled" icon="smart_toy" :title="t.aiAssistant.title" :text="config.enabled ? t.aiAssistant.enabled : t.aiAssistant.disabled" :disabled="isClearingApiKey" @update:model-value="saveConfig(false)" />
        <MChoiceItem v-model="config.provider" icon="cloud" :title="t.aiAssistant.provider" :options="providers" :cancel-label="t.common.cancel" @update:model-value="onProviderChange" />
        <MItem action icon="link" :title="labels.connection" :text="connectionLine" @click="open('connection')" />
        <MItem action icon="memory" :title="t.aiAssistant.model" :text="config.model || t.aiAssistant.modelPlaceholder" @click="open('model')" />
      </MGroup>
      <div class="m-actions-row">
        <MButton variant="tonal" icon="wifi" :loading="isLoading" @click="testConnection">{{ t.aiAssistant.testConnection }}</MButton>
        <MButton icon="save" :disabled="isClearingApiKey" @click="saveConfig()">{{ t.aiAssistant.saveConfig }}</MButton>
        <MChip v-if="isConnected" tone="ok" icon="check_circle">{{ t.aiAssistant.connected }}</MChip>
      </div>
    </template>

    <template v-else-if="current === 'connection'">
      <MGroup>
        <MFieldItem v-model="config.apiBase" :label="t.aiAssistant.apiUrl" placeholder="https://api.openai.com/v1" />
        <MFieldItem v-model="config.apiKey" type="password" :label="t.aiAssistant.apiKey" :placeholder="config.apiKeyConfigured ? '••••••••' : 'sk-...'" :supporting="t.aiAssistant.apiKeyHint" />
        <MChoiceItem v-model="config.compatibility" icon="swap_horiz" title="Compatibility" :options="[{ value: 'openai-chat', label: 'OpenAI Chat Completions' }, { value: 'anthropic-messages', label: 'Anthropic Messages' }]" :cancel-label="t.common.cancel" />
        <MItem v-if="config.apiKeyConfigured" action icon="delete" :title="t.aiAssistant.clearApiKey" :disabled="isClearingApiKey" @click="clearApiKey" />
      </MGroup>
      <div class="m-actions-row"><MButton icon="save" :disabled="isClearingApiKey" @click="saveConfig()">{{ t.aiAssistant.saveConfig }}</MButton></div>
    </template>

    <template v-else-if="current === 'model'">
      <MGroup>
        <MFieldItem v-model="config.model" :label="t.aiAssistant.model" :placeholder="t.aiAssistant.modelPlaceholder" :supporting="t.aiAssistant.modelHint" />
        <MChoiceItem v-if="availableModels.length" :model-value="config.model" icon="list" :title="t.aiAssistant.availableModels" :options="availableModels.map((m) => ({ value: m, label: m }))" :cancel-label="t.common.cancel" @update:model-value="(v) => { config.model = v }" />
        <MItem action icon="refresh" :title="t.aiAssistant.fetchModels" :disabled="isFetchingModels" @click="fetchRemoteModels" />
      </MGroup>
      <div class="m-actions-row"><MButton icon="save" @click="saveConfig()">{{ t.aiAssistant.saveConfig }}</MButton></div>
    </template>
  </MPage>
</template>

<script setup>
import { ref, nextTick, watch, computed, onMounted, onBeforeUnmount } from 'vue'
import MarkdownIt from 'markdown-it'
import { MPage, MGroup, MItem, MChoiceItem, MFieldItem, MSwitchItem, MButton, MIconButton, MChip, MIcon, MLoading, useSubpage } from 'foundation-ui/vue'
import { useAiAssistant, AI_PROVIDERS, MITA_PENDING_PROMPT } from '../composables/useAiAssistant.js'
import { useI18n } from '../desktop/i18n/index.js'
import { mitaOpened } from '../composables/mitaState.js'

const { t, locale } = useI18n()

const {
  config,
  needsConfiguration,
  isConnected,
  isLoading,
  isClearingApiKey,
  isFetchingModels,
  chatHistory,
  currentInput,
  availableModels,
  onProviderChange,
  fetchRemoteModels,
  testConnection,
  sendMessage,
  retryLastMessage,
  applyAction,
  clearHistory,
  clearApiKey,
  saveConfig,
} = useAiAssistant()

const providers = AI_PROVIDERS
// the caption bar's Mita button shows an unread answer only while the chat is closed
onMounted(() => {
  mitaOpened(true)
  let pending = ''
  try { pending = sessionStorage.getItem(MITA_PENDING_PROMPT) || ''; sessionStorage.removeItem(MITA_PENDING_PROMPT) } catch { /* nothing handed over */ }
  if (pending) { currentInput.value = pending; nextTick(handleSend) }
})
onBeforeUnmount(() => mitaOpened(false))
const chatContainer = ref(null)
const scrolled = ref(false) // the header gains its surface only when the conversation scrolls under it
const chatInput = ref(null)
const { current, open, back } = useSubpage('ai')
// the chat page opens first; a banner on it leads to the settings when the assistant is not set up yet
const labels = computed(() => (/^zh/i.test(String(locale.value)) ? { connection: '连接' } : { connection: 'Connection' }))
const connectionLine = computed(() => {
  let host = ''
  try { host = new URL(config.apiBase).host } catch { host = config.apiBase || '' }
  return [host, config.apiKeyConfigured ? '••••' : ''].filter(Boolean).join(' · ') || '—'
})
const headLine = computed(() => (config.enabled ? [activeProviderLabel.value, config.model].filter(Boolean).join(' · ') : t.value.aiAssistant.disabled))
const pageTitle = computed(() => ({ config: t.value.aiAssistant.modelConfig, connection: labels.value.connection, model: t.value.aiAssistant.model }[current.value] || t.value.aiAssistant.title))
const markdown = new MarkdownIt({
  breaks: true,
  html: false,
  linkify: false,
  typographer: false,
})

const isChatting = computed(() => chatHistory.value.length > 0 || isLoading.value)
const activeProviderLabel = computed(
  () => providers.find((provider) => provider.value === config.provider)?.label || config.provider,
)

// 示例问题
const exampleQueries = computed(() => [
  t.value.aiAssistant.exampleQueries.addKeyboardCmd,
  t.value.aiAssistant.exampleQueries.closeGameBar,
  t.value.aiAssistant.exampleQueries.highPerfPlan,
  t.value.aiAssistant.exampleQueries.optimizeGame,
  t.value.aiAssistant.exampleQueries.toggleHdr,
  t.value.aiAssistant.exampleQueries.checkLogs,
  t.value.aiAssistant.exampleQueries.analyzeConnection,
])

/**
 * 获取操作按钮文案
 */
function getActionButtonText(action) {
  const map = {
    add_menu_cmd: t.value.aiAssistant.actions.addMenuCmd,
    add_prep_cmd: t.value.aiAssistant.actions.addPrepCmd,
    enhance_apps: t.value.aiAssistant.actions.enhanceApps,
    modify_config: t.value.aiAssistant.actions.modifyConfig,
  }
  return map[action.action] || t.value.aiAssistant.actions.apply
}

/**
 * 获取操作提示文案
 */
function getActionHint(action) {
  const appName = action.app_name || 'Desktop'
  const count = action.commands?.length || action.changes?.length || action.apps?.length || 0
  const map = {
    add_menu_cmd: t.value.aiAssistant.actionHints.addMenuCmd.replace('{count}', count).replace('{appName}', appName),
    add_prep_cmd: t.value.aiAssistant.actionHints.addPrepCmd.replace('{count}', count).replace('{appName}', appName),
    enhance_apps: t.value.aiAssistant.actionHints.enhanceApps.replace('{count}', count),
    modify_config: t.value.aiAssistant.actionHints.modifyConfig.replace('{count}', count),
  }
  return map[action.action] || ''
}

/**
 * 发送消息
 */
async function handleSend() {
  if (!currentInput.value.trim() || isLoading.value) return
  const accepted = await sendMessage(currentInput.value)
  if (!accepted) return
  await nextTick()
  scrollToBottom()
}

/**
 * 选择示例问题并交给用户确认
 */
function selectExample(text) {
  currentInput.value = text
  nextTick(() => chatInput.value?.focus())
}

/**
 * 滚动到底部
 */
function scrollToBottom() {
  if (chatContainer.value) {
    chatContainer.value.scrollTop = chatContainer.value.scrollHeight
  }
}

/**
 * Keep horizontal scrolling local to code blocks and support both a mouse
 * tilt wheel (deltaX) and Shift + wheel in WebView/Chromium.
 */
function handleChatWheel(event) {
  const rawDelta = Math.abs(event.deltaX) > 0.01 ? event.deltaX : event.shiftKey ? event.deltaY : 0
  if (!rawDelta) return

  const eventTarget = event.target instanceof Element ? event.target : null
  const codeBlock = eventTarget?.closest('pre')
  const scrollTarget = codeBlock?.scrollWidth > codeBlock?.clientWidth ? codeBlock : chatContainer.value
  if (!scrollTarget || scrollTarget.scrollWidth <= scrollTarget.clientWidth) return

  const multiplier = event.deltaMode === WheelEvent.DOM_DELTA_LINE
    ? 24
    : event.deltaMode === WheelEvent.DOM_DELTA_PAGE
      ? scrollTarget.clientWidth
      : 1
  const previousScrollLeft = scrollTarget.scrollLeft
  scrollTarget.scrollLeft += rawDelta * multiplier

  if (scrollTarget.scrollLeft !== previousScrollLeft) event.preventDefault()
}

/**
 * Render readable Markdown. Once an action is parsed, its machine-readable
 * JSON is represented by the action card instead of being dumped into chat.
 */
function formatMessage(content, parsedAction) {
  if (!content) return ''

  let displayContent = content
  if (parsedAction) {
    displayContent = displayContent.replace(/```json\s*[\s\S]*?\s*```/i, '').trim()
    if (!displayContent) displayContent = parsedAction.explanation || ''
  }

  return markdown.render(displayContent)
}

/**
 * 格式化时间
 */
function formatTime(ts) {
  if (!ts) return ''
  const d = new Date(ts)
  return `${d.getHours().toString().padStart(2, '0')}:${d.getMinutes().toString().padStart(2, '0')}`
}

// 自动滚动到底部
watch(
  () => chatHistory.value.length,
  () => {
    nextTick(scrollToBottom)
  },
)

</script>


<style src="./AiAssistant.css"></style>
