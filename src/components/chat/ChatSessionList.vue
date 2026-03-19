<script setup lang="ts">
import { translate } from '@/lib/i18n'
import type { ChatSessionSummary } from '@/types'

defineProps<{
  sessions: ChatSessionSummary[]
  selectedSessionId: string | null
}>()

const emit = defineEmits<{
  select: [sessionId: string]
}>()
</script>

<template>
  <div v-if="sessions.length === 0" class="empty-state">
    {{ translate('chat.emptySessions') }}
  </div>
  <div v-else class="management-list">
    <button
      v-for="session in sessions"
      :key="session.id"
      class="management-row"
      :data-active="String(session.id === selectedSessionId)"
      type="button"
      @click="emit('select', session.id)"
    >
      <div class="management-row-main">
        <strong>{{ session.title }}</strong>
        <span>{{ session.preview || translate('chat.openSessionHint') }}</span>
      </div>
    </button>
  </div>
</template>
