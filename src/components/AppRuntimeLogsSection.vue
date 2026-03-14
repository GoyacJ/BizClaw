<script setup lang="ts">
import { computed } from 'vue'

import { formatAppTime, translate } from '@/lib/i18n'
import type { LogEntry } from '@/types'

const props = defineProps<{
  logs: LogEntry[]
}>()

const logPreview = computed(() => [...props.logs].reverse().slice(0, 16))
</script>

<template>
  <section class="page-stack">
    <article class="surface-card section-card">
      <div class="section-header">
        <div>
          <p class="eyebrow">{{ translate('runtimeLog.eyebrow') }}</p>
          <h3>{{ translate('runtimeLog.title') }}</h3>
        </div>
        <span class="status-chip" data-tone="neutral">{{ translate('runtimeLog.count', { count: props.logs.length }) }}</span>
      </div>
      <ol class="log-list">
        <li v-for="entry in logPreview" :key="`${entry.timestampMs}-${entry.message}`">
          <time>{{ formatAppTime(entry.timestampMs) }}</time>
          <strong>[{{ entry.source }}]</strong>
          <span>{{ entry.message }}</span>
        </li>
      </ol>
    </article>
  </section>
</template>
