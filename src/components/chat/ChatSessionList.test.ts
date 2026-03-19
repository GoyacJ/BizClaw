import { createApp, nextTick } from 'vue'
import { afterEach, describe, expect, it } from 'vitest'

import ChatSessionList from './ChatSessionList.vue'

describe('ChatSessionList', () => {
  let host: HTMLDivElement | null = null
  let app: ReturnType<typeof createApp> | null = null
  const selected: string[] = []

  afterEach(() => {
    app?.unmount()
    host?.remove()
    selected.length = 0
    app = null
    host = null
  })

  it('renders list rows and emits selected session id', async () => {
    host = document.createElement('div')
    document.body.appendChild(host)

    app = createApp(ChatSessionList, {
      sessions: [
        {
          id: 'session-1',
          title: '排障会话',
          updatedAt: Date.now() - 2000,
          preview: '上次定位到 SSH 隧道',
        },
        {
          id: 'session-2',
          title: '日报草稿',
          updatedAt: Date.now() - 1000,
          preview: '待补充发布节奏',
        },
      ],
      selectedSessionId: 'session-2',
      onSelect: (sessionId: string) => selected.push(sessionId),
    })
    app.mount(host)
    await nextTick()

    const rows = host.querySelectorAll<HTMLButtonElement>('.management-row')
    rows[0]?.click()
    await nextTick()

    expect(rows).toHaveLength(2)
    expect(rows[1]?.getAttribute('data-active')).toBe('true')
    expect(selected).toEqual(['session-1'])
  })
})
