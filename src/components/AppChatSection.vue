<script setup lang="ts">
import type { Ref } from 'vue'

import ChatComposer from '@/components/chat/ChatComposer.vue'
import ChatContextPanel from '@/components/chat/ChatContextPanel.vue'
import ChatMessageTimeline from '@/components/chat/ChatMessageTimeline.vue'
import ChatSessionList from '@/components/chat/ChatSessionList.vue'
import { translate } from '@/lib/i18n'
import type { ChatMessage, ChatSessionSummary } from '@/types'

interface ChatSectionState {
  sessions: Ref<ChatSessionSummary[]>
  selectedSessionId: Ref<string | null>
  messages: Ref<ChatMessage[]>
  loading: Ref<boolean>
  sending: Ref<boolean>
  error: Ref<string | null>
  createSession: () => Promise<void> | void
  selectSession: (sessionId: string) => Promise<void> | void
  sendMessage: (content: string) => Promise<void> | void
  retryMessage: (messageId: string) => Promise<void> | void
}

const props = defineProps<{
  state: ChatSectionState
}>()
</script>

<template>
  <section class="page-stack">
    <article class="surface-card section-card chat-workbench-header">
      <div class="section-header">
        <div>
          <p class="eyebrow">{{ translate('chat.eyebrow') }}</p>
          <h3>{{ translate('chat.title') }}</h3>
        </div>
        <span class="status-chip" :data-tone="props.state.loading.value ? 'active' : 'neutral'">
          {{ props.state.sessions.value.length }}
        </span>
      </div>
      <p class="supporting-text">{{ translate('chat.detail') }}</p>
      <div class="button-row">
        <button class="primary-button" type="button" :disabled="props.state.sending.value" @click="props.state.createSession">
          {{ translate('chat.newSession') }}
        </button>
      </div>
    </article>

    <div class="chat-workbench">
      <article class="surface-card section-card chat-workbench__sessions">
        <div class="section-header">
          <div>
            <p class="eyebrow">{{ translate('chat.sessions') }}</p>
            <h3>{{ translate('chat.sessions') }}</h3>
          </div>
        </div>
        <ChatSessionList
          :sessions="props.state.sessions.value"
          :selected-session-id="props.state.selectedSessionId.value"
          @select="props.state.selectSession"
        />
      </article>

      <article class="surface-card section-card chat-workbench__timeline">
        <div class="section-header">
          <div>
            <p class="eyebrow">{{ translate('chat.timeline') }}</p>
            <h3>{{ translate('chat.timeline') }}</h3>
          </div>
        </div>
        <ChatMessageTimeline :messages="props.state.messages.value" @retry="props.state.retryMessage" />
        <ChatComposer :sending="props.state.sending.value" @send="props.state.sendMessage" />
      </article>

      <article class="surface-card section-card chat-workbench__context">
        <ChatContextPanel
          :sessions="props.state.sessions.value"
          :selected-session-id="props.state.selectedSessionId.value"
          :messages="props.state.messages.value"
          :loading="props.state.loading.value"
          :sending="props.state.sending.value"
          :error="props.state.error.value"
        />
      </article>
    </div>
    <p v-if="props.state.error.value" class="error-banner">{{ props.state.error.value }}</p>
  </section>
</template>
