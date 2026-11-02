<template>
  <MComponentStatus :status="status" />
</template>

<script setup>
import { MComponentStatus, registeredComponentStatus } from 'foundation-ui/vue'

// 组件状态 of the control panel: the controller is the one the shell registered (SidebarMenu); the home cards read the same one.
const status = registeredComponentStatus()
</script>
