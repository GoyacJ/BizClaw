import { createApp, nextTick } from 'vue'
import { afterEach, describe, expect, it } from 'vitest'

import ChatMessageTimeline from './ChatMessageTimeline.vue'

describe('ChatMessageTimeline', () => {
  let host: HTMLDivElement | null = null
  let app: ReturnType<typeof createApp> | null = null
  const retries: string[] = []

  afterEach(() => {
    app?.unmount()
    host?.remove()
    retries.length = 0
    app = null
    host = null
  })

  it('shows retry action only for failed messages and emits message id', async () => {
    host = document.createElement('div')
    document.body.appendChild(host)

    app = createApp(ChatMessageTimeline, {
      messages: [
        {
          id: 'assistant-1',
          role: 'assistant',
          content: '先确认运行时状态。',
          createdAt: Date.now() - 1000,
          status: 'done',
        },
        {
          id: 'user-1',
          role: 'user',
          content: '请继续',
          createdAt: Date.now(),
          status: 'error',
        },
      ],
      onRetry: (messageId: string) => retries.push(messageId),
    })
    app.mount(host)
    await nextTick()

    const retryButton = Array.from(host.querySelectorAll<HTMLButtonElement>('button'))
      .find((button) => button.textContent?.includes('重试'))
    retryButton?.click()
    await nextTick()

    expect(retries).toEqual(['user-1'])
  })
})
