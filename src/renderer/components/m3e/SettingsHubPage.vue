<script setup>
import { MPage, MGroup, MItem } from 'foundation-ui/vue'
import { useI18n } from '../../desktop/i18n/index.js'
import { railItems } from '../../composables/panelNav.js'

// The one "设置" entry of the bottom bar of a narrow window: a list of the four settings categories, each opening its own list.
const emit = defineEmits(['select'])
const { t } = useI18n()
const CATEGORIES = ['config', 'input', 'display', 'system']
const items = () => railItems(t.value).filter((i) => CATEGORIES.includes(i.id))
</script>

<template>
  <MPage :title="t.m3e.nav.barSettings">
    <MGroup>
      <MItem v-for="item in items()" :key="item.id" action :icon="item.icon" :title="item.label" @click="emit('select', item)" />
    </MGroup>
  </MPage>
</template>
