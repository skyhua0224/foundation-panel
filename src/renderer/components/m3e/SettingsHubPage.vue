<script setup>
import { computed, nextTick, onMounted } from 'vue'
import { MPage, MGroup, MItem, MSection } from 'foundation-ui/vue'
import '@settings-data'
import { labelOf } from 'foundation-ui/settings/index.js'
import { useI18n } from '../../desktop/i18n/index.js'
import { pageItems, railItems } from '../../composables/panelNav.js'

// The one "设置" entry of the bottom bar of a narrow window: every page of the five settings categories and of 个性化 at once, in sections named by the category, so any page is two presses away
// (the bar entry, the page). `groups`: the stream settings' rows (the shared model); `runtime.lines`: the second line of every row, what is inside.
const emit = defineEmits(['open'])
// `focus`: the section to bring into view (更多 › 个性化 opens this page at 个性化)
const props = defineProps({ groups: { type: Array, default: () => [] }, runtime: { type: Object, default: () => ({}) }, focus: { type: String, default: '' } })
const { t, locale } = useI18n()
const CATEGORIES = ['config', 'input', 'audio', 'display', 'system']
const sections = computed(() => railItems(t.value).filter((r) => CATEGORIES.includes(r.id)).map((rail) => {
  const items = rail.id === 'config' ? props.groups.flatMap((g) => g.items) : pageItems(t.value, rail.id, props.runtime)
  // a row marked `separated` (the 高级 of 显示与视频) is a group of its own at the end
  return { id: rail.id, label: rail.label, items: items.filter((i) => !i.separated), apart: items.filter((i) => i.separated) }
}).concat([{ id: 'appearance', label: labelOf(locale.value, 'shell.appearance'), items: pageItems(t.value, 'appearance', props.runtime), apart: [] }]))
onMounted(() => { if (props.focus) void nextTick(() => document.querySelector(`[data-hub-section="${props.focus}"]`)?.scrollIntoView({ block: 'start' })) })
</script>

<template>
  <MPage :title="t.m3e.nav.barSettings">
    <MSection v-for="section in sections" :key="section.id" :title="section.label" :data-hub-section="section.id">
      <MGroup>
        <MItem v-for="item in section.items" :key="item.id" action :icon="item.icon" :title="item.label" :text="item.text" @click="emit('open', item)" />
      </MGroup>
      <MGroup v-if="section.apart.length">
        <MItem v-for="item in section.apart" :key="item.id" action :icon="item.icon" :title="item.label" :text="item.text" @click="emit('open', item)" />
      </MGroup>
    </MSection>
  </MPage>
</template>
