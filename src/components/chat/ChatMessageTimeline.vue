<script setup lang="ts">
import { formatAppTime, translate } from '@/lib/i18n'
import type { ChatMessage } from '@/types'

defineProps<{
  messages: ChatMessage[]
}>()

const emit = defineEmits<{
  retry: [messageId: string]
}>()
</script>

<template>
  <div v-if="messages.length === 0" class="empty-state">
    {{ translate('chat.timelineEmpty') }}
  </div>
  <div v-else class="chat-timeline">
    <article
      v-for="message in messages"
      :key="message.id"
      class="chat-message"
      :data-role="message.role"
      :data-status="message.status"
    >
      <header class="chat-message-header">
        <strong>
          {{
            message.role === 'assistant'
              ? translate('chat.role.assistant')
              : message.role === 'user'
                ? translate('chat.role.user')
                : translate('chat.role.system')
          }}
        </strong>
        <span class="card-label">{{ formatAppTime(message.createdAt) }}</span>
      </header>
      <p>{{ message.content }}</p>
      <div v-if="message.status === 'error'" class="button-row">
        <button class="ghost-button" type="button" @click="emit('retry', message.id)">
          {{ translate('chat.retry') }}
        </button>
      </div>
    </article>
  </div>
</template>
