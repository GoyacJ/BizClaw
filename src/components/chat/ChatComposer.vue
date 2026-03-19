<script setup lang="ts">
import { ref } from 'vue'

import { translate } from '@/lib/i18n'

const props = defineProps<{
  sending: boolean
}>()

const emit = defineEmits<{
  send: [content: string]
}>()

const draft = ref('')

function submit() {
  const content = draft.value.trim()
  if (!content || props.sending) {
    return
  }
  emit('send', content)
  draft.value = ''
}
</script>

<template>
  <div class="chat-composer">
    <label class="field field--span">
      <span>{{ translate('chat.composer.label') }}</span>
      <textarea
        v-model="draft"
        rows="3"
        :placeholder="translate('chat.composer.placeholder')"
        @keydown.enter.exact.prevent="submit"
      />
    </label>
    <div class="button-row button-row--end">
      <button class="primary-button" type="button" :disabled="sending" @click="submit">
        {{ sending ? translate('chat.composer.sending') : translate('chat.composer.send') }}
      </button>
    </div>
  </div>
</template>
