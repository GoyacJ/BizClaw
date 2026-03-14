<script setup lang="ts">
interface StatusBarItem {
  label: string
  value: string
  tone: string
}

const props = defineProps<{
  statusItems: StatusBarItem[]
}>()
</script>

<template>
  <footer class="status-bar surface-card">
    <div v-for="item in props.statusItems" :key="item.label" class="status-bar-item">
      <span>{{ item.label }}</span>
      <strong :data-tone="item.tone">{{ item.value }}</strong>
    </div>
  </footer>
</template>
