import { createApp, nextTick, ref } from 'vue'
import { afterEach, describe, expect, it, vi } from 'vitest'

import AppAgentsSection from './AppAgentsSection.vue'

describe('AppAgentsSection', () => {
  let host: HTMLDivElement | null = null
  let app: ReturnType<typeof createApp> | null = null

  afterEach(() => {
    app?.unmount()
    host?.remove()
    app = null
    host = null
  })

  it('shows dirty-state hint after identity fields are edited', async () => {
    host = document.createElement('div')
    document.body.appendChild(host)

    app = createApp(AppAgentsSection, {
      state: createState(),
    })
    app.mount(host)
    await nextTick()

    const input = host.querySelector<HTMLInputElement>('.form-grid .field input')
    input!.value = '新身份'
    input!.dispatchEvent(new Event('input'))
    await nextTick()

    expect(host.textContent).toContain('有未保存修改')
  })
})

function createState() {
  return {
    agents: ref([
      {
        id: 'main',
        name: 'Main',
        identityName: '霸天',
        identityEmoji: '🐯',
        identitySource: 'identity',
        workspace: '/Users/goya/.openclaw/workspace',
        agentDir: '/Users/goya/.openclaw/agents/main/agent',
        model: 'minimax-cn/MiniMax-M2.5',
        bindings: 1,
        isDefault: true,
      },
    ]),
    bindings: ref([
      {
        agentId: 'main',
        channel: 'telegram',
        accountId: null,
        description: 'telegram (default account)',
      },
    ]),
    bindingsLoading: ref(false),
    selectedAgentId: ref('main'),
    loading: ref(false),
    mutationBusy: ref(false),
    error: ref<string | null>(null),
    refreshAgents: vi.fn(),
    selectAgent: vi.fn(),
    createAgent: vi.fn(),
    updateAgentIdentity: vi.fn(),
    deleteAgent: vi.fn(),
    addAgentBindings: vi.fn(),
    removeAgentBindings: vi.fn(),
    clearAgentBindings: vi.fn(),
  }
}
