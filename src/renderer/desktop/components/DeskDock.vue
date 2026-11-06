<script setup>
import { computed } from 'vue'
import { useDeskScale } from '../composables/useDeskScale.js'

// The action bar of a page (save, reset), docked under the page area instead of floating over it: the window gives it a row of its own
// (#dsk-dock in DesktopWindow), above the gamepad legend, so it never covers a control. Same sofa scale as the page it belongs to.
const scale = useDeskScale()
const style = computed(() => ({ zoom: scale.value }))
</script>

<template>
  <Teleport to="#dsk-dock" defer>
    <div class="dsk-dock" :style="style"><slot /></div>
  </Teleport>
</template>

<style>
.dsk-dock { display: flex; flex-direction: column; align-items: center; gap: 8px; }
.dsk-dock .m-savebar { position: static; margin: 0; }
</style>
