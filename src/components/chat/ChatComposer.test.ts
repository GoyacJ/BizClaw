import { createApp, nextTick } from 'vue'
import { afterEach, describe, expect, it } from 'vitest'

import ChatComposer from './ChatComposer.vue'

describe('ChatComposer', () => {
  let host: HTMLDivElement | null = null
  let app: ReturnType<typeof createApp> | null = null
  const sends: string[] = []

  afterEach(() => {
    app?.unmount()
    host?.remove()
    sends.length = 0
    app = null
    host = null
  })

  it('submits trimmed text on enter and clears draft', async () => {
    host = document.createElement('div')
    document.body.appendChild(host)

    app = createApp(ChatComposer, {
      sending: false,
      onSend: (content: string) => sends.push(content),
    })
    app.mount(host)
    await nextTick()

    const textarea = host.querySelector('textarea')
    textarea!.value = '  你好，生成一份排查清单  '
    textarea!.dispatchEvent(new Event('input'))
    textarea!.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter' }))
    await nextTick()

    expect(sends).toEqual(['你好，生成一份排查清单'])
    expect((textarea as HTMLTextAreaElement).value).toBe('')
  })

  it('does not emit send when sending is true', async () => {
    host = document.createElement('div')
    document.body.appendChild(host)

    app = createApp(ChatComposer, {
      sending: true,
      onSend: (content: string) => sends.push(content),
    })
    app.mount(host)
    await nextTick()

    const textarea = host.querySelector('textarea')
    textarea!.value = '继续'
    textarea!.dispatchEvent(new Event('input'))
    textarea!.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter' }))
    await nextTick()

    expect(sends).toEqual([])
  })
})
