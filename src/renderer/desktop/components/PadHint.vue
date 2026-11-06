<template>
  <span class="pad-hint">
    <template v-for="(part, index) in parts" :key="index">
      <PadGlyph v-if="part.action" :action="part.action" size="sm" />
      <template v-else>{{ part.text }}</template>
    </template>
  </span>
</template>

<script setup>
import { computed } from 'vue'
import PadGlyph from './PadGlyph.vue'
import { splitHint } from '../composables/padGlyphs.js'

// 一句带按键的话：{confirm} {back} 这样的记号画成当前手柄上的图标，其余是普通文字。
// 屏幕键盘的提示、设置页的说明都这样写，文案里不出现具体的键名。
const props = defineProps({
  text: { type: String, default: '' },
})
const parts = computed(() => splitHint(props.text))
</script>

<style lang="less" scoped>
.pad-hint {
  // 图标与文字同一行，句子的行高不被撑开
  :deep(.pad-glyph) { margin: -0.1em 0.14em; }
  // a key in a sentence is a little smaller than the line, so its disc is never cut by the line box
  :deep(.pad-glyph.size-sm) { --pg: 1.4em; }
}
</style>
