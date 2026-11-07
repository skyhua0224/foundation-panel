<script setup>
import { computed } from 'vue'
import { MPage, MGroup, MItem, MSection, MButton } from 'foundation-ui/vue'
import { useI18n } from '../../desktop/i18n/index.js'
import { pageItems } from '../../composables/panelNav.js'

// 串流 of a narrow window: the same page as 设置 (the title, one group of rows with plain icons, the wallpaper behind); the page's one action (添加应用) is in the title row, where MPage puts
// actions. The rail's list page (a different surface: tonal icon blobs, a centred button) does not exist on a phone; Back from a page of 串流 is this page.
const emit = defineEmits(['open', 'add'])
const props = defineProps({ runtime: { type: Object, default: () => ({}) }, addLabel: { type: String, default: '' } })
const { t } = useI18n()
const items = computed(() => pageItems(t.value, 'stream', props.runtime))
</script>

<template>
  <MPage :title="t.m3e.nav.stream">
    <template #actions><MButton variant="tonal" icon="add" @click="emit('add')">{{ addLabel }}</MButton></template>
    <MSection>
      <MGroup>
        <MItem v-for="item in items" :key="item.id" action :icon="item.icon" :title="item.label" :text="item.text" @click="emit('open', item)" />
      </MGroup>
    </MSection>
  </MPage>
</template>
