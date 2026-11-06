<template>
  <MDialog
    ref="dialogRef"
    class="launch-dialog"
    :model-value="!!state"
    :title="headline"
    :icon="dialogIcon"
    :tone="dialogTone"
    :persistent="persistent"
    @update:model-value="onModel"
  >
    <div v-if="state" class="launch-card" :class="'status-' + state.status">
      <div class="launch-cover">
        <img v-if="state.coverUrl" :src="state.coverUrl" :alt="state.appName" />
        <span v-else class="launch-cover-letter">{{ state.appName?.[0] || '?' }}</span>
        <div v-if="state.status === 'launching'" class="launch-cover-busy"><MLoading size="56px" /></div>
      </div>
      <p class="launch-detail">{{ detail }}</p>
    </div>

    <template #actions>
      <MButton
        v-if="state?.status === 'conflict'"
        variant="danger"
        size="lg"
        icon="stop_circle"
        data-focusable
        data-focus-key="launch-stop"
        @click="emit('stop')"
      >
        {{ t.gameSession.stopRunning }}
      </MButton>
      <MButton
        v-if="state && state.status !== 'launching'"
        variant="tonal"
        size="lg"
        data-focusable
        data-focus-key="launch-dismiss"
        @click="emit('dismiss')"
      >
        {{ t.gameSession.dismiss }}
      </MButton>
    </template>
  </MDialog>
</template>

<script setup>
import { computed, nextTick, ref, watch } from 'vue'
import { MButton, MDialog, MIcon, MLoading } from 'foundation-ui/vue'
import { formatDuration } from '../composables/useGameSession.js'
import { focusElement, useModalFocusScope } from '../composables/useFocusNav.js'
import { useI18n } from '../i18n/index.js'

const { t } = useI18n()

const props = defineProps({
  state: { type: Object, default: null },
})

const emit = defineEmits(['dismiss', 'stop'])

// 这是一层吞点击的全屏遮罩，必须和其他弹层一样压入焦点栈：否则手柄导航会继续在
// 遮罩背后的应用网格里走，而 conflict 态自己的「结束游戏 / 关闭」按钮反而够不到
// —— teleport 出去的节点不在 .desktop-window 里，收集不到。
const dialogRef = ref(null)
// MDialog 的根就是 <dialog>，覆盖层成为顶层模态；手柄焦点栈以它为作用域
const overlayRef = computed(() => dialogRef.value?.$el || null)
useModalFocusScope(overlayRef, () => !!props.state)

// 状态在遮罩存续期间会变（launching -> error / conflict / untracked），而
// useModalFocusScope 只在开合时压栈，不会为中途才出现的按钮重新聚焦。所以这里
// 按状态自己收尾：
//   有按钮而焦点在遮罩外 -> 聚焦第一个。否则焦点还停在 launching 期间被 blur 掉的
//   body 上，第一次按 A 是空的（confirmFocused 对 body 直接返回 false）。
//   没有按钮（launching）-> 把焦点从遮罩背后的卡片上移开，否则 A 会点到被完全挡住
//   的应用卡片。关闭时 popFocusScope 会把焦点还回去。
watch(
  () => props.state?.status,
  async () => {
    await nextTick()
    const root = overlayRef.value
    if (!root) return
    const focusable = root.querySelector('[data-focusable]')
    if (focusable) {
      if (!root.contains(document.activeElement)) focusElement(focusable)
      return
    }
    if (document.activeElement instanceof HTMLElement) document.activeElement.blur()
  },
  { immediate: true }
)

const dialogIcon = computed(() => {
  switch (props.state?.status) {
    case 'started':
      return 'check_circle'
    case 'exited':
      return 'power_settings_new'
    case 'untracked':
    case 'conflict':
      return 'warning'
    case 'launching':
      return ''
    default:
      return 'error'
  }
})

const dialogTone = computed(() => {
  switch (props.state?.status) {
    case 'started':
    case 'exited':
      return 'info'
    case 'untracked':
    case 'conflict':
      return 'warning'
    default:
      return 'error'
  }
})

// launching 是唯一不可取消的状态；其余状态 Esc / 点遮罩 = 关闭
const persistent = computed(() => props.state?.status === 'launching')
function onModel(open) {
  if (!open && props.state && !persistent.value) emit('dismiss')
}

function text(key, replacements = {}) {
  let value = t.value.gameSession?.[key] || ''
  for (const [name, replacement] of Object.entries(replacements)) {
    value = value.replace(`{${name}}`, replacement)
  }
  return value
}

const headline = computed(() => {
  const name = props.state?.appName || ''
  switch (props.state?.status) {
    case 'launching':
      return text('launching', { name })
    case 'started':
      return text('started', { name })
    case 'untracked':
      return text('untracked', { name })
    case 'conflict':
      return text('conflict', { name: props.state.message || '' })
    case 'exited':
      return text('exited', { name })
    default:
      return text('launchFailed', { name })
  }
})

const detail = computed(() => {
  switch (props.state?.status) {
    case 'launching':
      return text('launchingHint')
    case 'started':
      return text('startedHint')
    case 'untracked':
      return props.state.message === 'detached-only'
        ? text('untrackedDetached')
        : text('untrackedNoHandle')
    case 'conflict':
      return text('conflictHint')
    case 'exited':
      return text('exitedHint', { duration: formatDuration(props.state.seconds) })
    default:
      return props.state?.message || ''
  }
})
</script>
<style lang="less" scoped>
.launch-dialog {
  max-width: 640px;
}

.launch-card {
  display: flex;
  align-items: center;
  gap: 24px;
}

.launch-cover {
  position: relative;
  flex: none;
  width: 120px;
  aspect-ratio: 3 / 4;
  border-radius: 24px;
  overflow: hidden;
  background: var(--md-sys-color-primary-container);
  color: var(--md-sys-color-on-primary-container);
  display: grid;
  place-items: center;
  box-shadow: var(--d-shadow-1);

  img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .launch-cover-letter {
    font: 800 56px/1 var(--md-ref-typeface-brand, system-ui);
  }

  .launch-cover-busy {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    background: color-mix(in srgb, var(--md-sys-color-scrim, #000) 45%, transparent);
  }
}

.launch-detail {
  margin: 0;
  font: 400 18px/28px var(--md-ref-typeface-brand, system-ui);
  color: var(--md-sys-color-on-surface-variant);
}
</style>
