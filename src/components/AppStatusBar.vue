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
    <div
      v-for="item in props.statusItems"
      :key="item.label"
      class="status-bar-item"
      :data-tone="item.tone"
      :aria-label="`${item.label}: ${item.value}`"
    >
      <span class="status-bar-item-label">{{ item.label }}</span>
      <span
        class="status-indicator"
        :data-tone="item.tone"
        :title="item.value"
        :aria-hidden="true"
      />
    </div>
  </footer>
</template>
