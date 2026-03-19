<script setup lang="ts">
import { computed } from 'vue'

import { formatAppTime, translate } from '@/lib/i18n'
import type { ChatMessage, ChatSessionSummary } from '@/types'

const props = defineProps<{
  sessions: ChatSessionSummary[]
  selectedSessionId: string | null
  messages: ChatMessage[]
  loading: boolean
  sending: boolean
  error: string | null
}>()

const activeSession = computed(() => (
  props.sessions.find((session) => session.id === props.selectedSessionId) ?? null
))
const latestMessage = computed(() => props.messages[props.messages.length - 1] ?? null)
const latestMessageTime = computed(() => (
  latestMessage.value ? formatAppTime(latestMessage.value.createdAt) : '-'
))
const runtimeStateLabel = computed(() => {
  if (props.loading) {
    return translate('common.inProgress')
  }
  if (props.sending) {
    return translate('chat.context.sending')
  }
  return translate('common.ready')
})
const runtimeStateTone = computed(() => {
  if (props.error) {
    return 'error'
  }
  if (props.loading || props.sending) {
    return 'active'
  }
  return 'success'
})
</script>

<template>
  <aside class="chat-context">
    <article class="chat-context-card">
      <p class="eyebrow">{{ translate('chat.context.sessionEyebrow') }}</p>
      <h3>{{ translate('chat.context.currentSession') }}</h3>
      <p class="supporting-text">{{ activeSession?.title || translate('chat.context.noSession') }}</p>
      <dl class="chat-context-metadata">
        <div>
          <dt>{{ translate('chat.context.messageCount') }}</dt>
          <dd>{{ messages.length }}</dd>
        </div>
        <div>
          <dt>{{ translate('chat.context.latestTime') }}</dt>
          <dd>{{ latestMessageTime }}</dd>
        </div>
      </dl>
    </article>

    <article class="chat-context-card">
      <p class="eyebrow">{{ translate('chat.context.agentEyebrow') }}</p>
      <h3>{{ translate('chat.context.activeAgent') }}</h3>
      <p class="supporting-text">{{ translate('chat.context.agentHint') }}</p>
      <span class="status-chip" :data-tone="runtimeStateTone">{{ runtimeStateLabel }}</span>
    </article>

    <article class="chat-context-card">
      <p class="eyebrow">{{ translate('chat.context.skillEyebrow') }}</p>
      <h3>{{ translate('chat.context.quickSkills') }}</h3>
      <ul class="chat-context-list">
        <li>{{ translate('chat.context.skillPromptRefine') }}</li>
        <li>{{ translate('chat.context.skillSummary') }}</li>
        <li>{{ translate('chat.context.skillChecklist') }}</li>
      </ul>
    </article>
  </aside>
</template>
