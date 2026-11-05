<script setup>
import { computed } from 'vue'
import { MPage, MGroup, MItem, MSection } from 'foundation-ui/vue'
import { useI18n } from '../../desktop/i18n/index.js'
import { pageItems, railItems } from '../../composables/panelNav.js'

// The one "设置" entry of the bottom bar of a narrow window: every page of the five settings categories at once, in sections named by the category, so any page is two presses away
// (the bar entry, the page). `groups`: the stream settings' rows (the shared model); `runtime.lines`: the second line of every row, what is inside.
const emit = defineEmits(['open'])
const props = defineProps({ groups: { type: Array, default: () => [] }, runtime: { type: Object, default: () => ({}) } })
const { t } = useI18n()
const CATEGORIES = ['config', 'input', 'audio', 'display', 'system']
const sections = computed(() => railItems(t.value).filter((r) => CATEGORIES.includes(r.id)).map((rail) => ({
  id: rail.id,
  label: rail.label,
  items: rail.id === 'config' ? props.groups.flatMap((g) => g.items) : pageItems(t.value, rail.id, props.runtime),
})))
</script>

<template>
  <MPage :title="t.m3e.nav.barSettings">
    <MSection v-for="section in sections" :key="section.id" :title="section.label">
      <MGroup>
        <MItem v-for="item in section.items" :key="item.id" action :icon="item.icon" :title="item.label" :text="item.text" @click="emit('open', item)" />
      </MGroup>
    </MSection>
  </MPage>
</template>
