<template>
  <MDialog
    ref="dialogRef"
    class="cover-dialog"
    :model-value="open"
    :title="t.coverPicker.title + (appName ? ' — ' + appName : '')"
    icon="image"
    tone="info"
    @update:model-value="onModel"
  >
    <div class="search-bar">
      <MField
        v-model="searchQuery"
        class="search-input"
        :label="t.coverPicker.searchPlaceholder"
        data-focusable
        data-focus-key="cover-search"
        @keyup.enter="doSearch"
      />
      <MButton size="lg" icon="search" :loading="searching" data-focusable data-focus-key="cover-search-btn" @click="doSearch">
        {{ searching ? t.coverPicker.searching : t.coverPicker.search }}
      </MButton>
    </div>

    <MBanner v-if="error" tone="error" icon="error" role="alert">{{ error }}</MBanner>

    <div v-if="searching && candidates.length === 0" class="cover-busy" role="status"><MLoading size="56px" /></div>

    <div v-if="candidates.length > 0" class="candidates-grid">
      <div
        v-for="c in candidates"
        :key="c.steam_id"
        class="candidate-card"
        :class="{ selected: selected?.steam_id === c.steam_id, uploading: uploadingId === c.steam_id }"
        role="button"
        tabindex="0"
        :aria-label="c.name"
        :aria-busy="uploadingId === c.steam_id || undefined"
        data-focusable
        :data-focus-key="'cover-' + c.steam_id"
        @click="selectAndUpload(c)"
        @keydown.enter.prevent="selectAndUpload(c)"
      >
        <img :src="c.header_url" :alt="c.name" class="candidate-img" loading="lazy" />
        <div class="candidate-name">{{ c.name }}</div>
        <div v-if="uploadingId === c.steam_id" class="uploading-overlay"><MLoading size="48px" /></div>
      </div>
    </div>

    <p v-else-if="!searching && !error && searched" class="no-results">{{ t.coverPicker.noResults }}</p>

    <template #actions>
      <MButton variant="text" size="lg" data-focusable data-focus-key="cover-close" @click="$emit('close')">{{ t.windowActions.close }}</MButton>
    </template>
  </MDialog>
</template>

<script setup>
import { computed, ref, watch } from 'vue'
import { MBanner, MButton, MDialog, MField, MLoading } from 'foundation-ui/vue'
import { useI18n } from '../i18n/index.js'

const { t } = useI18n()
import { tauriInvoke } from '../composables/useTauri'
import { useModalFocusScope } from '../composables/useFocusNav.js'

const props = defineProps({
  open: { type: Boolean, required: true },
  appName: { type: String, default: '' },
  proxyUrl: { type: String, default: '' },
})

const emit = defineEmits(['close', 'updated'])

const dialogRef = ref(null)
// MDialog 的根就是 <dialog>：手柄焦点栈以它为作用域
const modalRef = computed(() => dialogRef.value?.$el || null)
useModalFocusScope(modalRef, () => props.open)

// Esc / 点遮罩 → 关闭
function onModel(open) {
  if (!open && props.open) emit('close')
}

const searchQuery = ref('')
const candidates = ref([])
const selected = ref(null)
const searching = ref(false)
const searched = ref(false)
const error = ref('')
const uploadingId = ref(null)

// 打开时自动用 app 名搜索
watch(() => props.open, (v) => {
  if (v && props.appName) {
    searchQuery.value = props.appName
    doSearch()
  } else if (!v) {
    candidates.value = []
    selected.value = null
    error.value = ''
    searched.value = false
    uploadingId.value = null
  }
})

async function doSearch() {
  if (!searchQuery.value.trim()) return
  searching.value = true
  error.value = ''
  candidates.value = []
  searched.value = false
  selected.value = null

  try {
    candidates.value = await tauriInvoke('search_steam_covers', {
      query: searchQuery.value.trim(),
    })
  } catch (e) {
    console.error('Steam cover search failed:', e)
    error.value = t.value.coverPicker.searchFailed
  } finally {
    searching.value = false
    searched.value = true
  }
}

async function selectAndUpload(candidate) {
  if (uploadingId.value) return
  selected.value = candidate
  uploadingId.value = candidate.steam_id

  try {
    await tauriInvoke('upload_steam_cover', {
      headerUrl: candidate.header_url,
      appName: props.appName,
      proxyUrl: props.proxyUrl,
    })
    emit('updated', props.appName)
    emit('close')
  } catch (e) {
    console.error('Steam cover upload failed:', e)
    error.value = t.value.coverPicker.uploadFailed
  } finally {
    uploadingId.value = null
  }
}
</script>
<style lang="less" scoped>
.cover-dialog {
  max-width: min(980px, calc(100vw - 64px));
}

.search-bar {
  display: flex;
  align-items: center;
  gap: 16px;

  .search-input {
    flex: 1;
  }
}

.cover-busy {
  display: grid;
  place-items: center;
  padding: 40px 0;
}

.candidates-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
  gap: 16px;
  padding: 6px;
  margin: -6px;
}

.candidate-card {
  position: relative;
  border-radius: 24px;
  overflow: hidden;
  background: var(--md-sys-color-surface-container-high);
  cursor: pointer;
  outline: none;
  transition: transform var(--m-spring-fast-spatial-duration, 200ms) var(--m-spring-fast-spatial, ease),
    box-shadow var(--m-spring-default-effects-duration, 200ms) var(--m-spring-default-effects, ease);

  &:hover {
    transform: translateY(-3px);
    box-shadow: var(--d-shadow-3);
  }

  &:focus-visible {
    box-shadow: 0 0 0 3px var(--md-sys-color-primary);
  }

  // the chosen cover: the selected container (foundation-ui layout.css --m-sel-fill), a heavier name and a check on the picture; no outline (focus keeps its ring)
  &.selected {
    background: var(--m-sel-fill);
  }

  &.selected .candidate-name {
    font-weight: 700;
  }

  &.selected::before,
  &.selected::after {
    content: '';
    position: absolute;
    top: 10px;
    right: 10px;
    width: 28px;
    height: 28px;
    pointer-events: none;
  }

  &.selected::before {
    border-radius: 50%;
    background: var(--md-sys-color-primary);
  }

  &.selected::after {
    background: var(--md-sys-color-on-primary);
    -webkit-mask: var(--m-check-mask) center / 20px no-repeat;
    mask: var(--m-check-mask) center / 20px no-repeat;
  }

  &.uploading {
    pointer-events: none;
  }

  .candidate-img {
    display: block;
    width: 100%;
    aspect-ratio: 460 / 215;
    object-fit: cover;
  }

  .candidate-name {
    padding: 12px 16px 14px;
    font: 500 15px/20px var(--md-ref-typeface-brand, system-ui);
    color: var(--md-sys-color-on-surface);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .uploading-overlay {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    background: color-mix(in srgb, var(--md-sys-color-scrim, #000) 50%, transparent);
  }
}

.no-results {
  margin: 0;
  padding: 24px 0;
  text-align: center;
  font: 400 18px/28px var(--md-ref-typeface-brand, system-ui);
  color: var(--md-sys-color-on-surface-variant);
}
</style>
