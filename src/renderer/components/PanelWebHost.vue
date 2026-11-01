<script setup>
import { inject } from 'vue'

// The embedded Web UI frame lives here for the whole session (so it never reloads when the window is resized). Where the
// list is hidden behind the page (single pane) a small bar carries the way back.
const scene = inject('m-scene', null)
// a page of the frame that is on a sub-page has its own back bubble: the way back to the list waits until it is on its first page
defineProps({ sub: { type: Boolean, default: false } })
</script>

<template>
  <div class="panel-web">
    <div v-if="scene?.canBack.value && !sub" class="panel-web__bar">
      <button type="button" class="m-page__back" @click="scene.back()"><svg class="m-icon" viewBox="0 -960 960 960" aria-hidden="true"><path d="M313-440l224 224-57 56-320-320 320-320 57 56-224 224h487v80H313Z" /></svg></button>
    </div>
    <div class="panel-web__frame"><slot /></div>
  </div>
</template>

<style>
.panel-web { display: flex; flex-direction: column; height: 100%; min-height: 0; }
.panel-web__bar { padding: 8px 12px 0; flex: none; }
.panel-web__frame { flex: 1; min-height: 0; }
.panel-web__frame .iframe-container, .panel-web__frame .sunshine-iframe { height: 100%; width: 100%; }
</style>
