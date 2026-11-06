<template>
  <MSheet
    ref="sheetRef"
    class="helper-sheet"
    wide
    :model-value="open"
    :title="t.launchHelper.title + (appName ? ' — ' + appName : '')"
    :close-label="t.windowActions.close"
    @update:model-value="onModel"
  >
    <p class="panel-desc">{{ t.launchHelper.description }}<span class="panel-hint">{{ t.launchHelper.orderHint }}</span></p>

    <MBanner v-if="saveError" tone="error" icon="error" role="alert">{{ saveError }}</MBanner>

    <div
      v-for="(helper, idx) in editingHelpers"
      :key="helper.templateId"
      class="helper-card"
      :class="{ active: helper.enabled }"
    >
      <div class="helper-header">
        <div class="order-buttons" :class="{ 'is-off': !helper.enabled }">
          <MIconButton
            icon="arrow_upward"
            :label="t.launchHelper.moveUp"
            :disabled="!helper.enabled || idx === 0"
            data-focusable
            @click.stop="moveHelper(idx, -1)"
          />
          <MIconButton
            icon="arrow_downward"
            :label="t.launchHelper.moveDown"
            :disabled="!helper.enabled || idx === editingHelpers.length - 1"
            data-focusable
            @click.stop="moveHelper(idx, 1)"
          />
        </div>

        <LaunchHelperIcon
          class="helper-icon"
          :template-id="helper.templateId"
          :size="24"
          @click="toggleHelper(helper.templateId)"
        />
        <div class="helper-info" @click="toggleHelper(helper.templateId)">
          <span class="helper-name">
            {{ getTemplate(helper.templateId)?.name }}
            <MChip
              v-if="getTemplate(helper.templateId)?.typeLabel"
              class="type-badge"
              :tone="getTemplate(helper.templateId)?.elevated ? 'warn' : 'neutral'"
              :title="getTemplate(helper.templateId)?.typeDesc"
            >{{ getTemplate(helper.templateId)?.typeLabel }}</MChip>
          </span>
          <span class="helper-desc">{{ getTemplate(helper.templateId)?.description }}</span>
        </div>
        <MSwitch
          class="helper-switch"
          :model-value="helper.enabled"
          :label="getTemplate(helper.templateId)?.name"
          data-focusable
          @update:model-value="toggleHelper(helper.templateId)"
        />
      </div>

      <div v-if="helper.enabled" class="helper-config">
        <div
          v-for="param in getTemplate(helper.templateId)?.params"
          :key="param.key"
          class="config-field"
        >
          <div class="field-input-row">
            <MField
              class="field-input"
              :label="param.label + (param.required ? ' *' : '')"
              :placeholder="param.placeholder || ''"
              :model-value="getParamValue(helper.templateId, param.key)"
              :error="hasFieldError(helper.templateId, param.key) ? getFieldError(helper.templateId, param.key) : ''"
              :supporting="fieldNote(helper.templateId, param.key)"
              data-focusable
              @update:model-value="setParamValue(helper.templateId, param.key, $event)"
            />
            <MIconButton
              v-if="param.key === 'path' && hasTauri"
              icon="folder_open"
              :label="t.launchHelper.browse"
              data-focusable
              @click="browseFile(helper.templateId, param.key)"
            />
          </div>
        </div>
      </div>
    </div>

    <div v-if="hasAnyEnabled" class="preview-section">
      <MButton variant="text" :icon="showPreview ? 'keyboard_arrow_up' : 'chevron_right'" data-focusable @click="showPreview = !showPreview">
        {{ t.launchHelper.preview }}
      </MButton>
      <div v-if="showPreview" class="preview-content">
        <div v-for="(cmd, i) in previewCommands" :key="i" class="preview-cmd">
          <div class="cmd-label">{{ cmd.label }}</div>
          <code class="cmd-code">{{ cmd.value }}</code>
        </div>
      </div>
    </div>

    <template #actions>
      <MButton variant="text" size="lg" data-focusable data-focus-key="helper-cancel" @click="$emit('close')">{{ t.launchHelper.cancel }}</MButton>
      <MButton size="lg" icon="save" :loading="saving" data-focusable data-focus-key="helper-save" @click="handleSave">
        {{ saving ? t.launchHelper.saving : t.launchHelper.save }}
      </MButton>
    </template>
  </MSheet>
</template>

<script setup>
import { ref, computed, watch, onMounted } from 'vue'
import { MBanner, MButton, MChip, MField, MIconButton, MSheet, MSwitch, useNotify } from 'foundation-ui/vue'
import { useLaunchHelpers } from '../composables/useLaunchHelpers.js'
import { isTauriRuntime } from '../composables/useTauri.js'
import { useModalFocusScope } from '../composables/useFocusNav.js'
import { useI18n } from '../i18n/index.js'
import LaunchHelperIcon from './LaunchHelperIcon.vue'

const props = defineProps({
  open: { type: Boolean, default: false },
  appName: { type: String, default: '' },
  app: { type: Object, default: () => ({}) },
  proxyUrl: { type: String, default: '' },
})

const emit = defineEmits(['close', 'saved'])

// 手柄导航只在栈顶作用域内移动，避免焦点跑到面板背后的卡片上
const sheetRef = ref(null)
// MSheet 的根就是 <dialog>：手柄焦点栈以它为作用域
const panelRef = computed(() => sheetRef.value?.$el || null)
useModalFocusScope(panelRef, () => props.open)

const notify = useNotify()
const saveError = ref('')

// Esc / 点遮罩 / 标题栏关闭 → 关闭
function onModel(open) {
  if (!open && props.open) emit('close')
}

const { t } = useI18n()
const lhText = computed(() => t.value.launchHelper)

const {
  templates,
  getGlobalPath,
  setGlobalPath,
  getAppHelpers,
  setAppHelpers,
  validateHelpers,
  generateAppCommands,
  buildPreviewCommands,
} = useLaunchHelpers(t)

const hasTauri = ref(false)
const saving = ref(false)
const showPreview = ref(false)
const validationErrors = ref([]) // { templateId, paramKey, message }[]
const pathWarnings = ref({}) // { "templateId.paramKey": "message" }

// 编辑中的助手列表（本地副本）
const editingHelpers = ref([])

// 当面板打开/应用变化时，加载配置
watch(() => [props.open, props.appName], ([isOpen]) => {
  if (isOpen && props.appName) {
    saveError.value = ''
    const stored = getAppHelpers(props.appName)
    // 为每个模板创建一个条目
    editingHelpers.value = templates.value.map(tmpl => {
      const existing = stored.find(h => h.templateId === tmpl.id)
      return {
        templateId: tmpl.id,
        enabled: existing?.enabled || false,
        params: { ...(existing?.params || {}) },
      }
    })
  }
}, { immediate: true })

function isEnabled(templateId) {
  const h = editingHelpers.value.find(h => h.templateId === templateId)
  return h?.enabled || false
}

function getTemplate(templateId) {
  return templates.value.find(t => t.id === templateId)
}

function toggleHelper(templateId) {
  const h = editingHelpers.value.find(h => h.templateId === templateId)
  if (h) h.enabled = !h.enabled
}

function moveHelper(index, direction) {
  const target = index + direction
  if (target < 0 || target >= editingHelpers.value.length) return
  const arr = [...editingHelpers.value]
  ;[arr[index], arr[target]] = [arr[target], arr[index]]
  editingHelpers.value = arr
}

function getParamValue(templateId, paramKey) {
  const h = editingHelpers.value.find(h => h.templateId === templateId)
  return h?.params?.[paramKey] || getGlobalPath(templateId, paramKey)
}

function setParamValue(templateId, paramKey, value) {
  const h = editingHelpers.value.find(h => h.templateId === templateId)
  if (h) {
    h.params = { ...h.params, [paramKey]: value }
    // 仅当全局路径为空时才自动回写，避免 app 级修改污染其他应用
    if (paramKey === 'path' && value && !getGlobalPath(templateId, paramKey)) {
      setGlobalPath(templateId, paramKey, value)
      const tmpl = getTemplate(templateId)
      notify.snack((lhText.value.globalSaved || '').replace('{name}', tmpl?.name || templateId), { timeout: 3000 })
    }
    // 清除该字段的验证错误
    validationErrors.value = validationErrors.value.filter(
      e => !(e.templateId === templateId && e.paramKey === paramKey)
    )
    // 异步校验路径是否存在
    if (paramKey === 'path' && value) {
      checkPathExists(templateId, paramKey, value)
    } else if (paramKey === 'path') {
      delete pathWarnings.value[`${templateId}.${paramKey}`]
    }
  }
}

let _pathCheckTimer = null
async function checkPathExists(templateId, paramKey, filePath) {
  clearTimeout(_pathCheckTimer)
  const key = `${templateId}.${paramKey}`
  _pathCheckTimer = setTimeout(async () => {
    if (!hasTauri.value) return
    try {
      const { exists } = await import('@tauri-apps/plugin-fs')
      const found = await exists(filePath)
      if (!found) {
        pathWarnings.value = { ...pathWarnings.value, [key]: lhText.value.fileNotFound }
      } else {
        const { [key]: _, ...rest } = pathWarnings.value
        pathWarnings.value = rest
      }
    } catch {
      // fs plugin 不可用，忽略
    }
  }, 500) // 延迟 500ms 防抖
}

function getGlobalHint(templateId, paramKey) {
  const h = editingHelpers.value.find(h => h.templateId === templateId)
  const localVal = h?.params?.[paramKey]
  const globalVal = getGlobalPath(templateId, paramKey)
  // 只在有全局默认且本地没有覆盖时显示
  if (globalVal && !localVal) return globalVal
  return ''
}

/** 字段下方的说明：路径警告 > 全局默认提示；错误由 MField 的 error 显示。 */
function fieldNote(templateId, paramKey) {
  const warn = pathWarnings.value[`${templateId}.${paramKey}`]
  if (warn) return warn
  const hint = getGlobalHint(templateId, paramKey)
  return hint ? `${lhText.value.globalDefault}${hint}` : ''
}

function hasFieldError(templateId, paramKey) {
  return validationErrors.value.some(e => e.templateId === templateId && e.paramKey === paramKey)
}

function getFieldError(templateId, paramKey) {
  return validationErrors.value.find(e => e.templateId === templateId && e.paramKey === paramKey)?.message || ''
}

async function browseFile(templateId, paramKey) {
  try {
    const { open } = await import('@tauri-apps/plugin-dialog')
    const path = await open({
      filters: [
        { name: lhText.value.executableFiles || 'Executables', extensions: ['exe', 'bat', 'cmd', 'lnk', 'com', 'scr'] },
        { name: lhText.value.allFiles || 'All Files', extensions: ['*'] },
      ],
    })
    if (path) {
      setParamValue(templateId, paramKey, path)
    }
  } catch (e) {
    console.warn('File dialog not available:', e)
  }
}

const hasAnyEnabled = computed(() => editingHelpers.value.some(h => h.enabled))

const previewCommands = computed(() => {
  return buildPreviewCommands(editingHelpers.value)
})

async function handleSave() {
  // 验证必填字段
  const errors = validateHelpers(editingHelpers.value)
  validationErrors.value = errors
  if (errors.length > 0) {
    return
  }

  saving.value = true
  saveError.value = ''
  try {
    // 1. 保存到本地
    setAppHelpers(props.appName, editingHelpers.value)

    // 2. 生成 prep-cmd 并保存到服务器
    const { prepCmds, wrapCmd } = generateAppCommands(props.appName)

    // 获取当前所有应用数据
    const resp = await fetch(`${props.proxyUrl}/api/apps`)
    if (!resp.ok) throw new Error('Failed to fetch apps')
    const data = await resp.json()
    const allApps = data.apps || data || []

    const appIndex = allApps.findIndex(a => a.name === props.appName)
    if (appIndex === -1) throw new Error('App not found')

    const app = { ...allApps[appIndex] }
    const MARKER = '& REM launch-helper'

    // 移除旧的 helper prep-cmd
    app['prep-cmd'] = (app['prep-cmd'] || []).filter(
      c => !c.do?.includes('REM launch-helper') && !c.undo?.includes('REM launch-helper')
    )

    // 添加新的 helper prep-cmd
    const helperPrepCmds = prepCmds.map(c => ({
      do: `${c.do} ${MARKER}`,
      undo: c.undo ? `${c.undo} ${MARKER}` : '',
      elevated: c.elevated ? 'true' : 'false',
    }))
    app['prep-cmd'] = [...helperPrepCmds, ...app['prep-cmd']]

    // 应用命令包装器
    if (wrapCmd) {
      const originalCmd = app._originalCmd || app.cmd
      app.cmd = wrapCmd(originalCmd)
    }

    // 删除内部字段，不发给服务器
    delete app._originalCmd

    // 按 Sunshine API 格式保存：{ apps: [...], editApp: { ...app, index } }
    const editApp = { ...app, index: appIndex }
    const saveResp = await fetch(`${props.proxyUrl}/api/apps`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ apps: allApps, editApp }),
    })

    if (!saveResp.ok) throw new Error('Failed to save app')

    emit('saved')
    emit('close')
  } catch (e) {
    console.error('Failed to save launch helpers:', e)
    saveError.value = `${lhText.value.saveFailed || 'Save failed: '}${e.message}`
  } finally {
    saving.value = false
  }
}

onMounted(async () => {
  hasTauri.value = await isTauriRuntime()
})
</script>
<style lang="less" scoped>
.helper-sheet {
  color: var(--md-sys-color-on-surface);

  // 标题栏的字色：顶层 <dialog> 不继承窗口的文字色，深色下会掉成暗蓝
  :deep(.m-sheet__head h2) {
    color: var(--md-sys-color-on-surface);
  }
}

.panel-desc {
  margin: 0 0 16px;
  font: 400 17px/26px var(--md-ref-typeface-brand, system-ui);
  color: var(--md-sys-color-on-surface-variant);

  .panel-hint {
    display: block;
    font-size: 14px;
    line-height: 20px;
    opacity: 0.85;
  }
}

.helper-card {
  margin-top: 14px;
  padding: 16px 20px;
  border-radius: 28px;
  background: var(--md-sys-color-surface-container);
  transition: background-color var(--m-spring-default-effects-duration, 200ms) var(--m-spring-default-effects, ease);

  &.active {
    background: var(--md-sys-color-primary-container);
    color: var(--md-sys-color-on-primary-container);
  }
}

.helper-header {
  display: flex;
  align-items: center;
  gap: 14px;
}

.order-buttons {
  display: flex;
  flex-direction: column;
  flex: none;

  &.is-off {
    visibility: hidden;
  }
}

.helper-icon {
  flex: none;
  display: grid;
  place-items: center;
  width: 52px;
  height: 52px;
  border-radius: 18px;
  background: var(--md-sys-color-surface-container-highest);
  cursor: pointer;
}

.helper-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
  cursor: pointer;

  .helper-name {
    display: flex;
    align-items: center;
    gap: 10px;
    font: 600 19px/26px var(--md-ref-typeface-brand, system-ui);
  }

  .helper-desc {
    font: 400 14px/20px var(--md-ref-typeface-brand, system-ui);
    opacity: 0.8;
  }
}

.helper-config {
  display: flex;
  flex-direction: column;
  gap: 14px;
  margin-top: 16px;
}

.field-input-row {
  display: flex;
  align-items: flex-start;
  gap: 8px;

  .field-input {
    flex: 1;
  }
}

.preview-section {
  margin-top: 20px;
}

.preview-content {
  display: flex;
  flex-direction: column;
  gap: 10px;
  margin-top: 8px;
}

.preview-cmd {
  padding: 12px 16px;
  border-radius: 20px;
  background: var(--md-sys-color-surface-container-high);

  .cmd-label {
    font: 500 13px/18px var(--md-ref-typeface-brand, system-ui);
    color: var(--md-sys-color-on-surface-variant);
  }

  .cmd-code {
    display: block;
    margin-top: 4px;
    font: 400 14px/20px ui-monospace, 'Cascadia Mono', monospace;
    color: var(--md-sys-color-on-surface);
    word-break: break-all;
    white-space: pre-wrap;
  }
}
</style>
