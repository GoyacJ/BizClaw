<script setup lang="ts">
import { translate } from '@/lib/i18n'
import { phaseLabel } from '@/lib/runtime-view'
import type { RuntimePhase } from '@/types'

const props = defineProps<{
  sectionTitle: string
  runtimePhase: RuntimePhase
  busyLabel: string
}>()

const emit = defineEmits<{
  refresh: []
}>()
</script>

<template>
  <header class="workspace-header surface-card">
    <div>
      <p class="eyebrow">{{ translate('workspace.eyebrow') }}</p>
      <h2>{{ props.sectionTitle }}</h2>
    </div>
    <div class="workspace-actions">
      <span
        class="status-chip"
        :data-tone="props.runtimePhase === 'running' ? 'success' : props.runtimePhase === 'error' ? 'error' : 'neutral'"
      >
        {{ phaseLabel(props.runtimePhase) }}
      </span>
      <span v-if="props.busyLabel" class="status-chip" data-tone="active">{{ props.busyLabel }}</span>
      <button class="secondary-button" @click="emit('refresh')">{{ translate('workspace.refresh') }}</button>
    </div>
  </header>
</template>
