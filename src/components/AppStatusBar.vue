<script setup lang="ts">
import { translate } from '@/lib/i18n'

interface StatusBarItem {
  label: string
  value: string
  tone: string
}

const props = defineProps<{
  latestLogSummary: string
  latestLogTitle?: string | null
  statusItems: StatusBarItem[]
}>()
</script>

<template>
  <footer class="status-bar surface-card">
    <div
      class="status-bar-latest-log"
      :title="props.latestLogTitle ?? props.latestLogSummary"
      :aria-label="`${translate('statusBar.latestLog')}: ${props.latestLogSummary}`"
    >
      <span class="status-bar-latest-log-label">{{ translate('statusBar.latestLog') }}</span>
      <strong class="status-bar-latest-log-message">{{ props.latestLogSummary }}</strong>
    </div>

    <div class="status-bar-statuses">
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
    </div>
  </footer>
</template>
