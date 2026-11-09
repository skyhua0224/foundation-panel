<script setup>
import { computed } from 'vue'
import { MPage, MGroup, MItem, MSection } from 'foundation-ui/vue'
import { useI18n } from '../../desktop/i18n/index.js'
import { pageItems } from '../../composables/panelNav.js'

// 串流 of a narrow window: the same page as 设置 (the title, one group of rows with plain icons, the wallpaper behind). It only lists pages: 添加应用 is not here, it is the button of
// the page 串流应用 itself (a list of pages has no action of the app list). The rail's list page (a different surface: tonal icon blobs) does not exist on a phone;
// Back from a page of 串流 is this page.
const emit = defineEmits(['open'])
const props = defineProps({ runtime: { type: Object, default: () => ({}) } })
const { t } = useI18n()
const items = computed(() => pageItems(t.value, 'stream', props.runtime))
</script>

<template>
  <MPage :title="t.m3e.nav.stream">
    <MSection>
      <MGroup>
        <MItem v-for="item in items" :key="item.id" action :icon="item.icon" :title="item.label" :text="item.text" @click="emit('open', item)" />
      </MGroup>
    </MSection>
  </MPage>
</template>
