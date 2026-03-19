import { createApp, nextTick, ref } from 'vue'
import { afterEach, describe, expect, it, vi } from 'vitest'

import AppChatSection from './AppChatSection.vue'

describe('AppChatSection', () => {
  let host: HTMLDivElement | null = null
  let app: ReturnType<typeof createApp> | null = null

  afterEach(() => {
    app?.unmount()
    host?.remove()
    app = null
    host = null
  })

  it('renders a three-panel workbench layout with context card', async () => {
    host = document.createElement('div')
    document.body.appendChild(host)

    app = createApp(AppChatSection, {
      state: createChatState(),
    })
    app.mount(host)
    await nextTick()

    expect(host.querySelector('.chat-workbench')).not.toBeNull()
    expect(host.querySelector('.chat-workbench__sessions')).not.toBeNull()
    expect(host.querySelector('.chat-workbench__timeline')).not.toBeNull()
    expect(host.querySelector('.chat-workbench__context')).not.toBeNull()
    expect(host.textContent).toContain('当前会话')
    expect(host.textContent).toContain('活跃 Agent')
    expect(host.textContent).toContain('快捷技能')
  })

  it('wires new-session and retry interactions to chat state actions', async () => {
    const state = createChatState()
    host = document.createElement('div')
    document.body.appendChild(host)

    app = createApp(AppChatSection, { state })
    app.mount(host)
    await nextTick()

    const createButton = Array.from(host.querySelectorAll<HTMLButtonElement>('button'))
      .find((button) => button.textContent?.includes('新建会话'))
    createButton?.click()
    await nextTick()

    const retryButton = Array.from(host.querySelectorAll<HTMLButtonElement>('button'))
      .find((button) => button.textContent?.includes('重试'))
    retryButton?.click()
    await nextTick()

    expect(state.createSession).toHaveBeenCalledTimes(1)
    expect(state.retryMessage).toHaveBeenCalledWith('failed-user-1')
  })
})

function createChatState() {
  return {
    sessions: ref([
      {
        id: 'session-1',
        title: '默认会话',
        updatedAt: Date.now(),
        preview: '上次提问：如何优化日志面板？',
      },
    ]),
    selectedSessionId: ref('session-1'),
    messages: ref([
      {
        id: 'assistant-1',
        role: 'assistant' as const,
        content: '可以先把日志输出按模块分组。',
        createdAt: Date.now() - 1000,
        status: 'done' as const,
      },
      {
        id: 'failed-user-1',
        role: 'user' as const,
        content: '把分组规则也给我',
        createdAt: Date.now(),
        status: 'error' as const,
      },
    ]),
    loading: ref(false),
    sending: ref(false),
    error: ref<string | null>(null),
    createSession: vi.fn(),
    selectSession: vi.fn(),
    sendMessage: vi.fn(),
    retryMessage: vi.fn(),
  }
}
