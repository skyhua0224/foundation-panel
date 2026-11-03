<template>
  <MSection :title="T.title">
    <MGroup>
      <MItem action icon="refresh" :title="T.restart" :text="T.restartText" @click="run('restart', () => post('/api/restart'))"><template #trail><MIcon v-if="busy === 'restart'" name="sync" /></template></MItem>
      <MItem action icon="restart_alt" :title="T.resetDisplay" :text="T.resetDisplayText" @click="run('reset', () => post('/api/reset-display-device-persistence'), true)" />
      <MItem action icon="cancel" :title="T.forceClose" :text="T.forceCloseText" @click="run('close', () => post('/api/apps/close'), true)" />
      <MItem action icon="wand_stars" :title="T.wizard" @click="reopenWizard" />
      <MItem action icon="smart_toy" :title="T.ai" :text="T.aiText" @click="emit('open-web-page', '/troubleshooting')"><template #trail><MIcon name="open_in_new" /></template></MItem>
      <MItem action icon="warning" :title="T.boom" :text="T.boomText" @click="boomAsk = true" />
    </MGroup>
    <MDialog v-model="boomAsk" :title="T.boomConfirm" icon="warning" tone="error">
      <p>{{ T.boomConfirmText }}</p>
      <template #actions>
        <MButton variant="text" @click="boomAsk = false">{{ T.cancel }}</MButton>
        <MButton variant="danger" icon="warning" @click="boom">{{ T.boom }}</MButton>
      </template>
    </MDialog>
  </MSection>
</template>

<script setup>
import { computed, ref } from 'vue'
import { ElMessage } from '../../composables/notifyMessage.js'
import { MSection, MGroup, MItem, MIcon, MDialog, MButton } from 'foundation-ui/vue'
import { sunshine } from '../../tauri-adapter.js'
import { useI18n } from '../../desktop/i18n/index.js'

// The host actions of the old 故障诊断 page, done natively with the same host routes: restart, reset the display device persistence, force close
// the running app, reopen the setup wizard, stop Sunshine (with a confirmation). The AI diagnosis of the log lives on the Web page, one row away.
const emit = defineEmits(['open-web-page', 'open-guide'])
const { locale } = useI18n()
const ZH = { title: '主机操作', restart: '重启 Sunshine', restartText: '运行异常时可尝试重启，会终止当前会话', resetDisplay: '重置显示器设置', resetDisplayText: '无法自动恢复显示设备设置时，重置后再手动调整', forceClose: '强制关闭应用', forceCloseText: '客户端提示应用仍在运行时，强制关闭它', wizard: '重新打开新手引导', ai: 'AI 诊断', aiText: '让助手分析日志（在故障排除页）', boom: '停止 Sunshine', boomText: '立即关闭，之后需要手动开启', boomConfirm: '停止 Sunshine？', boomConfirmText: 'Sunshine 会立即关闭，需要手动重新开启。', cancel: '取消', done: '已完成', failed: '操作失败' }
const EN = { title: 'Host actions', restart: 'Restart Sunshine', restartText: 'Try this when it misbehaves; it ends the current session', resetDisplay: 'Reset display settings', resetDisplayText: 'When the display device settings cannot be restored by themselves', forceClose: 'Force close the app', forceCloseText: 'When a client says an app is still running', wizard: 'Reopen the setup guide', ai: 'AI diagnosis', aiText: 'Let the assistant read the log (on the troubleshooting page)', boom: 'Stop Sunshine', boomText: 'Closes it at once; start it again by hand', boomConfirm: 'Stop Sunshine?', boomConfirmText: 'Sunshine closes at once and must be started again by hand.', cancel: 'Cancel', done: 'Done', failed: 'The action failed' }
const T = computed(() => (/^zh/i.test(String(locale.value)) ? ZH : EN))
const busy = ref('')
const boomAsk = ref(false)
const base = async () => String((await sunshine.getProxyUrl()) || '').replace(/\/$/, '')
const post = async (path, body) => {
  const response = await fetch(`${await base()}${path}`, { method: 'POST', headers: body ? { 'Content-Type': 'application/json' } : undefined, body: body ? JSON.stringify(body) : undefined })
  if (!response.ok) throw new Error(`HTTP ${response.status}`)
  return response
}
async function run(id, action, report = false) {
  busy.value = id
  try { await action(); if (report) ElMessage.success(T.value.done) } catch { ElMessage.error(T.value.failed) } finally { busy.value = '' }
}
async function boom() {
  boomAsk.value = false
  await run('boom', async () => { await fetch(`${await base()}/api/boom`) }, true)
}
async function reopenWizard() {
  await run('wizard', async () => {
    const response = await fetch(`${await base()}/api/config`)
    if (!response.ok) throw new Error('config')
    const config = await response.json()
    config.setup_wizard_completed = false
    await post('/api/config', config)
    emit('open-guide')
  })
}
</script>
