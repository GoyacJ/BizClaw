import { createApp, defineComponent, h, nextTick } from 'vue'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { translate } from '@/lib/i18n'
import { useAppModel } from './use-app-model'
import type { EnvironmentSnapshot, LogEntry, OperationEvent, OperationTaskSnapshot } from '@/types'

const apiMocks = vi.hoisted(() => ({
  addOpenClawAgentBindings: vi.fn(),
  checkOpenClawUpdate: vi.fn(),
  checkOpenClawSkills: vi.fn(),
  createLocalOpenClawSkill: vi.fn(),
  createOpenClawAgent: vi.fn(),
  deleteLocalOpenClawSkill: vi.fn(),
  deleteOpenClawAgent: vi.fn(),
  detectEnvironment: vi.fn(),
  getOpenClawSkillInfo: vi.fn(),
  installClawHubSkill: vi.fn(),
  streamLogs: vi.fn(),
  getOperationStatus: vi.fn(),
  getOperationEvents: vi.fn(),
  installOpenClaw: vi.fn(),
  listChatSessions: vi.fn(),
  listChatMessages: vi.fn(),
  listOpenClawAgentBindings: vi.fn(),
  listOpenClawAgents: vi.fn(),
  listOpenClawSkills: vi.fn(),
  openManualInstall: vi.fn(),
  onEnvironmentSnapshot: vi.fn(),
  onRuntimeLog: vi.fn(),
  onRuntimeStatus: vi.fn(),
  onOperationStatus: vi.fn(),
  onOperationEvent: vi.fn(),
  onConnectionTestEvent: vi.fn(),
  onRefreshRequested: vi.fn(),
  openSupportUrl: vi.fn(),
  saveProfile: vi.fn(),
  saveAndTestConnection: vi.fn(),
  saveUiPreferences: vi.fn(),
  searchClawHubSkills: vi.fn(),
  startRuntime: vi.fn(),
  stopOpenClawOperation: vi.fn(),
  stopRuntime: vi.fn(),
  testConnection: vi.fn(),
  sendChatMessage: vi.fn(),
  createChatSession: vi.fn(),
  removeOpenClawAgentBindings: vi.fn(),
  updateOpenClaw: vi.fn(),
  updateOpenClawAgentIdentity: vi.fn(),
}))

const bizclawUpdaterMocks = vi.hoisted(() => ({
  checkForBizClawUpdate: vi.fn(),
  describeBizClawUpdaterError: vi.fn((error: unknown) => (
    error instanceof Error ? error.message : String(error)
  )),
  getCurrentBizClawVersion: vi.fn(),
  relaunchBizClaw: vi.fn(),
}))

vi.mock('@/lib/api', () => ({
  addOpenClawAgentBindings: apiMocks.addOpenClawAgentBindings,
  checkOpenClawUpdate: apiMocks.checkOpenClawUpdate,
  checkOpenClawSkills: apiMocks.checkOpenClawSkills,
  createLocalOpenClawSkill: apiMocks.createLocalOpenClawSkill,
  createOpenClawAgent: apiMocks.createOpenClawAgent,
  detectEnvironment: apiMocks.detectEnvironment,
  deleteLocalOpenClawSkill: apiMocks.deleteLocalOpenClawSkill,
  deleteOpenClawAgent: apiMocks.deleteOpenClawAgent,
  getOperationEvents: apiMocks.getOperationEvents,
  getOperationStatus: apiMocks.getOperationStatus,
  getOpenClawSkillInfo: apiMocks.getOpenClawSkillInfo,
  installClawHubSkill: apiMocks.installClawHubSkill,
  installOpenClaw: apiMocks.installOpenClaw,
  listChatSessions: apiMocks.listChatSessions,
  listChatMessages: apiMocks.listChatMessages,
  listOpenClawAgentBindings: apiMocks.listOpenClawAgentBindings,
  listOpenClawAgents: apiMocks.listOpenClawAgents,
  listOpenClawSkills: apiMocks.listOpenClawSkills,
  onConnectionTestEvent: apiMocks.onConnectionTestEvent,
  onEnvironmentSnapshot: apiMocks.onEnvironmentSnapshot,
  onOperationEvent: apiMocks.onOperationEvent,
  onOperationStatus: apiMocks.onOperationStatus,
  onRefreshRequested: apiMocks.onRefreshRequested,
  onRuntimeLog: apiMocks.onRuntimeLog,
  onRuntimeStatus: apiMocks.onRuntimeStatus,
  openManualInstall: apiMocks.openManualInstall,
  openSupportUrl: apiMocks.openSupportUrl,
  saveProfile: apiMocks.saveProfile,
  saveAndTestConnection: apiMocks.saveAndTestConnection,
  saveUiPreferences: apiMocks.saveUiPreferences,
  searchClawHubSkills: apiMocks.searchClawHubSkills,
  startRuntime: apiMocks.startRuntime,
  stopOpenClawOperation: apiMocks.stopOpenClawOperation,
  stopRuntime: apiMocks.stopRuntime,
  streamLogs: apiMocks.streamLogs,
  testConnection: apiMocks.testConnection,
  sendChatMessage: apiMocks.sendChatMessage,
  createChatSession: apiMocks.createChatSession,
  removeOpenClawAgentBindings: apiMocks.removeOpenClawAgentBindings,
  updateOpenClaw: apiMocks.updateOpenClaw,
  updateOpenClawAgentIdentity: apiMocks.updateOpenClawAgentIdentity,
}))

vi.mock('@/lib/bizclaw-updater', () => ({
  checkForBizClawUpdate: bizclawUpdaterMocks.checkForBizClawUpdate,
  describeBizClawUpdaterError: bizclawUpdaterMocks.describeBizClawUpdaterError,
  getCurrentBizClawVersion: bizclawUpdaterMocks.getCurrentBizClawVersion,
  relaunchBizClaw: bizclawUpdaterMocks.relaunchBizClaw,
}))

describe('useAppModel', () => {
  let host: HTMLDivElement | null = null
  let app: ReturnType<typeof createApp> | null = null
  const originalMatchMedia = window.matchMedia

  afterEach(() => {
    app?.unmount()
    host?.remove()
    host = null
    app = null
    document.documentElement.dataset.theme = ''
    document.documentElement.style.colorScheme = ''
    window.matchMedia = originalMatchMedia
    vi.clearAllMocks()
  })

  it('hydrates operation status and recent events on mount in order', async () => {
    const snapshot = createSnapshot()
    const task: OperationTaskSnapshot = {
      phase: 'cancelled',
      kind: 'install',
      step: 'installOpenClaw',
      canStop: false,
      lastResult: {
        kind: 'install',
        strategy: 'official',
        success: false,
        step: 'installOpenClaw',
        stdout: 'partial output',
        stderr: '',
        needsElevation: false,
        manualUrl: 'https://docs.openclaw.ai/install',
        followUp: '安装已停止。',
      },
      startedAt: 1,
      endedAt: 2,
    }
    const events: OperationEvent[] = [
      {
        kind: 'install',
        step: 'installOpenClaw',
        status: 'cancelled',
        source: 'system',
        message: 'official 已停止',
        timestampMs: 2,
      },
    ]

    apiMocks.detectEnvironment.mockResolvedValue(snapshot)
    apiMocks.streamLogs.mockResolvedValue([])
    apiMocks.getOperationStatus.mockResolvedValue(task)
    apiMocks.getOperationEvents.mockResolvedValue(events)
    bizclawUpdaterMocks.getCurrentBizClawVersion.mockResolvedValue('0.1.8')
    apiMocks.onRuntimeLog.mockResolvedValue(() => {})
    apiMocks.onRuntimeStatus.mockResolvedValue(() => {})
    apiMocks.onEnvironmentSnapshot.mockResolvedValue(() => {})
    apiMocks.onOperationStatus.mockResolvedValue(() => {})
    apiMocks.onOperationEvent.mockResolvedValue(() => {})
    apiMocks.onConnectionTestEvent.mockResolvedValue(() => {})
    apiMocks.onRefreshRequested.mockResolvedValue(() => {})

    let model!: ReturnType<typeof useAppModel>
    const TestHarness = defineComponent({
      setup() {
        model = useAppModel()
        return () => h('div')
      },
    })

    host = document.createElement('div')
    document.body.appendChild(host)

    app = createApp(TestHarness)
    app.mount(host)

    await flushPromises()

    expect(model.operationTask.value).toEqual(task)
    expect(model.operationEvents.value).toEqual(events)
    expect(apiMocks.detectEnvironment).toHaveBeenCalledTimes(1)
    expect(apiMocks.streamLogs).toHaveBeenCalledTimes(1)
    expect(apiMocks.getOperationStatus).toHaveBeenCalledTimes(1)
    expect(apiMocks.getOperationEvents).toHaveBeenCalledTimes(1)
    expect(apiMocks.detectEnvironment.mock.invocationCallOrder[0]).toBeLessThan(apiMocks.streamLogs.mock.invocationCallOrder[0])
    expect(apiMocks.streamLogs.mock.invocationCallOrder[0]).toBeLessThan(apiMocks.getOperationStatus.mock.invocationCallOrder[0])
    expect(apiMocks.getOperationStatus.mock.invocationCallOrder[0]).toBeLessThan(apiMocks.getOperationEvents.mock.invocationCallOrder[0])
  })

  it('loads the current BizClaw version on mount without checking remote updates', async () => {
    apiMocks.detectEnvironment.mockResolvedValue(createSnapshot())
    apiMocks.streamLogs.mockResolvedValue([])
    apiMocks.getOperationStatus.mockResolvedValue(createIdleTask())
    apiMocks.getOperationEvents.mockResolvedValue([])
    bizclawUpdaterMocks.getCurrentBizClawVersion.mockResolvedValue('0.1.8')
    bizclawUpdaterMocks.checkForBizClawUpdate.mockResolvedValue(null)
    apiMocks.onRuntimeLog.mockResolvedValue(() => {})
    apiMocks.onRuntimeStatus.mockResolvedValue(() => {})
    apiMocks.onEnvironmentSnapshot.mockResolvedValue(() => {})
    apiMocks.onOperationStatus.mockResolvedValue(() => {})
    apiMocks.onOperationEvent.mockResolvedValue(() => {})
    apiMocks.onConnectionTestEvent.mockResolvedValue(() => {})
    apiMocks.onRefreshRequested.mockResolvedValue(() => {})

    let model!: ReturnType<typeof useAppModel>
    const TestHarness = defineComponent({
      setup() {
        model = useAppModel()
        return () => h('div')
      },
    })

    host = document.createElement('div')
    document.body.appendChild(host)
    app = createApp(TestHarness)
    app.mount(host)

    await flushPromises()

    expect(bizclawUpdaterMocks.getCurrentBizClawVersion).toHaveBeenCalledTimes(1)
    expect(bizclawUpdaterMocks.checkForBizClawUpdate).not.toHaveBeenCalled()
    expect(model.bizclawUpdate.value.phase).toBe('idle')
    expect(model.bizclawUpdate.value.currentVersion).toBe('0.1.8')
    expect(model.bizclawUpdateActionLabel.value).toBe('已加载当前版本')
  })

  it('loads agent and skill management state lazily when each section is opened', async () => {
    apiMocks.detectEnvironment.mockResolvedValue(createSnapshot())
    apiMocks.streamLogs.mockResolvedValue([])
    apiMocks.getOperationStatus.mockResolvedValue(createIdleTask())
    apiMocks.getOperationEvents.mockResolvedValue([])
    apiMocks.listOpenClawAgents.mockResolvedValue([
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
        routes: ['default (no explicit rules)'],
      },
    ])
    apiMocks.listOpenClawAgentBindings.mockResolvedValue([
      {
        agentId: 'main',
        channel: 'telegram',
        accountId: null,
        description: 'telegram (default account)',
      },
    ])
    apiMocks.listOpenClawSkills.mockResolvedValue({
      workspaceDir: '/Users/goya/.openclaw/workspace',
      managedSkillsDir: '/Users/goya/.openclaw/skills',
      skills: [
        {
          name: 'sonoscli',
          description: 'Control Sonos speakers.',
          eligible: true,
          disabled: false,
          blockedByAllowlist: false,
          source: 'openclaw-workspace',
          bundled: false,
          locationKind: 'workspaceLocal',
          canDelete: true,
          missing: {
            bins: [],
            anyBins: [],
            env: [],
            config: [],
            os: [],
          },
        },
      ],
    })
    bizclawUpdaterMocks.getCurrentBizClawVersion.mockResolvedValue('0.1.8')
    apiMocks.onRuntimeLog.mockResolvedValue(() => {})
    apiMocks.onRuntimeStatus.mockResolvedValue(() => {})
    apiMocks.onEnvironmentSnapshot.mockResolvedValue(() => {})
    apiMocks.onOperationStatus.mockResolvedValue(() => {})
    apiMocks.onOperationEvent.mockResolvedValue(() => {})
    apiMocks.onConnectionTestEvent.mockResolvedValue(() => {})
    apiMocks.onRefreshRequested.mockResolvedValue(() => {})

    let model!: ReturnType<typeof useAppModel>
    const TestHarness = defineComponent({
      setup() {
        model = useAppModel()
        return () => h('div')
      },
    })

    host = document.createElement('div')
    document.body.appendChild(host)
    app = createApp(TestHarness)
    app.mount(host)

    await flushPromises()

    expect(apiMocks.listOpenClawAgents).not.toHaveBeenCalled()
    expect(apiMocks.listOpenClawAgentBindings).not.toHaveBeenCalled()
    expect(apiMocks.listOpenClawSkills).not.toHaveBeenCalled()
    expect(apiMocks.checkOpenClawSkills).not.toHaveBeenCalled()

    model.activeSection.value = 'agent'
    await flushPromises()

    expect(apiMocks.listOpenClawAgents).toHaveBeenCalledTimes(1)
    expect(apiMocks.listOpenClawAgentBindings).toHaveBeenCalledTimes(1)
    expect(apiMocks.listOpenClawAgentBindings).toHaveBeenCalledWith('main')
    expect(model.agentsState.agents.value[0]?.id).toBe('main')
    expect(model.agentsState.bindings.value[0]?.description).toContain('telegram')

    model.activeSection.value = 'skill'
    await flushPromises()

    expect(apiMocks.listOpenClawSkills).toHaveBeenCalledTimes(1)
    expect(model.skillsState.inventory.value.skills[0]?.name).toBe('sonoscli')
    expect(model.skillsState.checkReport.value.summary.total).toBe(1)

    model.activeSection.value = 'agent'
    await flushPromises()

    expect(apiMocks.listOpenClawAgents).toHaveBeenCalledTimes(1)
    expect(apiMocks.listOpenClawSkills).toHaveBeenCalledTimes(1)
  })

  it('loads chat sessions lazily and sends a message in the selected session', async () => {
    apiMocks.detectEnvironment.mockResolvedValue(createSnapshot())
    apiMocks.streamLogs.mockResolvedValue([])
    apiMocks.getOperationStatus.mockResolvedValue(createIdleTask())
    apiMocks.getOperationEvents.mockResolvedValue([])
    apiMocks.listChatSessions.mockResolvedValue([
      {
        id: 'session-1',
        title: '默认会话',
        updatedAt: Date.now(),
      },
    ])
    apiMocks.listChatMessages.mockResolvedValue([
      {
        id: 'msg-1',
        role: 'assistant',
        content: '你好，我是 BizClaw。',
        createdAt: Date.now(),
        status: 'done',
      },
    ])
    apiMocks.sendChatMessage.mockResolvedValue({
      userMessage: {
        id: 'msg-2',
        role: 'user',
        content: '你好',
        createdAt: Date.now(),
        status: 'done',
      },
      assistantMessage: {
        id: 'msg-3',
        role: 'assistant',
        content: '你好，有什么可以帮你？',
        createdAt: Date.now(),
        status: 'done',
      },
    })
    bizclawUpdaterMocks.getCurrentBizClawVersion.mockResolvedValue('0.1.8')
    apiMocks.onRuntimeLog.mockResolvedValue(() => {})
    apiMocks.onRuntimeStatus.mockResolvedValue(() => {})
    apiMocks.onEnvironmentSnapshot.mockResolvedValue(() => {})
    apiMocks.onOperationStatus.mockResolvedValue(() => {})
    apiMocks.onOperationEvent.mockResolvedValue(() => {})
    apiMocks.onConnectionTestEvent.mockResolvedValue(() => {})
    apiMocks.onRefreshRequested.mockResolvedValue(() => {})

    let model!: ReturnType<typeof useAppModel>
    const TestHarness = defineComponent({
      setup() {
        model = useAppModel()
        return () => h('div')
      },
    })

    host = document.createElement('div')
    document.body.appendChild(host)
    app = createApp(TestHarness)
    app.mount(host)

    await flushPromises()
    expect(apiMocks.listChatSessions).not.toHaveBeenCalled()

    model.activeSection.value = 'chat'
    await flushPromises()

    expect(apiMocks.listChatSessions).toHaveBeenCalledTimes(1)
    expect(apiMocks.listChatMessages).toHaveBeenCalledWith('session-1')
    expect(model.chatState.sessions.value[0]?.title).toBe('默认会话')

    await model.chatState.sendMessage('你好')
    await flushPromises()

    expect(apiMocks.sendChatMessage).toHaveBeenCalledWith({
      sessionId: 'session-1',
      content: '你好',
    })
    expect(model.chatState.messages.value[model.chatState.messages.value.length - 1]?.role).toBe('assistant')
  })

  it('marks the correct optimistic message as failed when multiple sends overlap', async () => {
    apiMocks.detectEnvironment.mockResolvedValue(createSnapshot())
    apiMocks.streamLogs.mockResolvedValue([])
    apiMocks.getOperationStatus.mockResolvedValue(createIdleTask())
    apiMocks.getOperationEvents.mockResolvedValue([])
    apiMocks.listChatSessions.mockResolvedValue([
      {
        id: 'session-1',
        title: '默认会话',
        updatedAt: Date.now(),
      },
    ])
    apiMocks.listChatMessages.mockResolvedValue([])
    let resolveFirstSend!: (value: {
      userMessage: {
        id: string
        role: 'user'
        content: string
        createdAt: number
        status: 'done'
      }
      assistantMessage: {
        id: string
        role: 'assistant'
        content: string
        createdAt: number
        status: 'done'
      }
    }) => void
    apiMocks.sendChatMessage
      .mockImplementationOnce(() => new Promise((resolve) => {
        resolveFirstSend = resolve
      }))
      .mockRejectedValueOnce(new Error('network lost'))
    bizclawUpdaterMocks.getCurrentBizClawVersion.mockResolvedValue('0.1.8')
    apiMocks.onRuntimeLog.mockResolvedValue(() => {})
    apiMocks.onRuntimeStatus.mockResolvedValue(() => {})
    apiMocks.onEnvironmentSnapshot.mockResolvedValue(() => {})
    apiMocks.onOperationStatus.mockResolvedValue(() => {})
    apiMocks.onOperationEvent.mockResolvedValue(() => {})
    apiMocks.onConnectionTestEvent.mockResolvedValue(() => {})
    apiMocks.onRefreshRequested.mockResolvedValue(() => {})

    let model!: ReturnType<typeof useAppModel>
    const TestHarness = defineComponent({
      setup() {
        model = useAppModel()
        return () => h('div')
      },
    })

    host = document.createElement('div')
    document.body.appendChild(host)
    app = createApp(TestHarness)
    app.mount(host)

    await flushPromises()
    model.activeSection.value = 'chat'
    await flushPromises()

    const firstSend = model.chatState.sendMessage('first message')
    await nextTick()
    const secondSend = model.chatState.sendMessage('second message')
    await flushPromises()

    resolveFirstSend({
      userMessage: {
        id: 'msg-user-1',
        role: 'user',
        content: 'first message',
        createdAt: Date.now(),
        status: 'done',
      },
      assistantMessage: {
        id: 'msg-assistant-1',
        role: 'assistant',
        content: 'done',
        createdAt: Date.now(),
        status: 'done',
      },
    })
    await Promise.allSettled([firstSend, secondSend])
    await flushPromises()

    const failedSecond = model.chatState.messages.value.find((item) => item.content === 'second message')
    expect(failedSecond?.status).toBe('error')
    expect(model.chatState.messages.value.some((item) => item.content === 'first message' && item.status === 'done')).toBe(true)
  })

  it('ignores stale message-list response when switching sessions quickly', async () => {
    apiMocks.detectEnvironment.mockResolvedValue(createSnapshot())
    apiMocks.streamLogs.mockResolvedValue([])
    apiMocks.getOperationStatus.mockResolvedValue(createIdleTask())
    apiMocks.getOperationEvents.mockResolvedValue([])
    apiMocks.listChatSessions.mockResolvedValue([
      {
        id: 'session-1',
        title: '会话1',
        updatedAt: Date.now() - 1000,
      },
      {
        id: 'session-2',
        title: '会话2',
        updatedAt: Date.now(),
      },
    ])
    let resolveSession1!: (messages: Array<{
      id: string
      role: 'assistant'
      content: string
      createdAt: number
      status: 'done'
    }>) => void
    apiMocks.listChatMessages
      .mockImplementationOnce(() => new Promise((resolve) => {
        resolveSession1 = resolve
      }))
      .mockResolvedValueOnce([
        {
          id: 'session-2-message',
          role: 'assistant',
          content: '来自会话2',
          createdAt: Date.now(),
          status: 'done',
        },
      ])
    bizclawUpdaterMocks.getCurrentBizClawVersion.mockResolvedValue('0.1.8')
    apiMocks.onRuntimeLog.mockResolvedValue(() => {})
    apiMocks.onRuntimeStatus.mockResolvedValue(() => {})
    apiMocks.onEnvironmentSnapshot.mockResolvedValue(() => {})
    apiMocks.onOperationStatus.mockResolvedValue(() => {})
    apiMocks.onOperationEvent.mockResolvedValue(() => {})
    apiMocks.onConnectionTestEvent.mockResolvedValue(() => {})
    apiMocks.onRefreshRequested.mockResolvedValue(() => {})

    let model!: ReturnType<typeof useAppModel>
    const TestHarness = defineComponent({
      setup() {
        model = useAppModel()
        return () => h('div')
      },
    })

    host = document.createElement('div')
    document.body.appendChild(host)
    app = createApp(TestHarness)
    app.mount(host)

    await flushPromises()
    model.activeSection.value = 'chat'
    await nextTick()
    await model.chatState.selectSession('session-2')
    await flushPromises()

    resolveSession1([
      {
        id: 'session-1-message',
        role: 'assistant',
        content: '来自会话1',
        createdAt: Date.now(),
        status: 'done',
      },
    ])
    await flushPromises()

    expect(model.chatState.selectedSessionId.value).toBe('session-2')
    expect(model.chatState.messages.value).toHaveLength(1)
    expect(model.chatState.messages.value[0]?.id).toBe('session-2-message')
  })

  it('refreshes agent state after creating a new agent', async () => {
    apiMocks.detectEnvironment.mockResolvedValue(createSnapshot())
    apiMocks.streamLogs.mockResolvedValue([])
    apiMocks.getOperationStatus.mockResolvedValue(createIdleTask())
    apiMocks.getOperationEvents.mockResolvedValue([])
    apiMocks.listOpenClawAgents
      .mockResolvedValueOnce([
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
          routes: ['default (no explicit rules)'],
        },
      ])
      .mockResolvedValueOnce([
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
          routes: ['default (no explicit rules)'],
        },
        {
          id: 'ops',
          name: 'Ops',
          identityName: '值班',
          identityEmoji: '🛠',
          identitySource: 'config',
          workspace: '/tmp/openclaw-ops',
          agentDir: '/Users/goya/.openclaw/agents/ops/agent',
          model: 'minimax-cn/MiniMax-M2.5',
          bindings: 0,
          isDefault: false,
          routes: [],
        },
      ])
    apiMocks.listOpenClawAgentBindings.mockResolvedValue([])
    apiMocks.listOpenClawSkills.mockResolvedValue({
      workspaceDir: '/Users/goya/.openclaw/workspace',
      managedSkillsDir: '/Users/goya/.openclaw/skills',
      skills: [],
    })
    apiMocks.createOpenClawAgent.mockResolvedValue({
      agentId: 'ops',
    })
    bizclawUpdaterMocks.getCurrentBizClawVersion.mockResolvedValue('0.1.8')
    apiMocks.onRuntimeLog.mockResolvedValue(() => {})
    apiMocks.onRuntimeStatus.mockResolvedValue(() => {})
    apiMocks.onOperationStatus.mockResolvedValue(() => {})
    apiMocks.onOperationEvent.mockResolvedValue(() => {})
    apiMocks.onConnectionTestEvent.mockResolvedValue(() => {})
    apiMocks.onRefreshRequested.mockResolvedValue(() => {})

    let model!: ReturnType<typeof useAppModel>
    const TestHarness = defineComponent({
      setup() {
        model = useAppModel()
        return () => h('div')
      },
    })

    host = document.createElement('div')
    document.body.appendChild(host)
    app = createApp(TestHarness)
    app.mount(host)

    await flushPromises()
    model.activeSection.value = 'agent'
    await flushPromises()

    await model.agentsState.createAgent({
      name: 'Ops',
      workspace: '/tmp/openclaw-ops',
      model: 'minimax-cn/MiniMax-M2.5',
      bindings: [],
    })
    await flushPromises()

    expect(apiMocks.createOpenClawAgent).toHaveBeenCalledWith({
      name: 'Ops',
      workspace: '/tmp/openclaw-ops',
      model: 'minimax-cn/MiniMax-M2.5',
      bindings: [],
    })
    expect(apiMocks.listOpenClawAgents).toHaveBeenCalledTimes(2)
    expect(apiMocks.listOpenClawAgentBindings).toHaveBeenNthCalledWith(1, 'main')
    expect(apiMocks.listOpenClawAgentBindings).toHaveBeenNthCalledWith(2, 'ops')
    expect(model.agentsState.agents.value).toHaveLength(2)
  })

  it('keeps agent mutations responsive while the revalidation refresh continues in the background', async () => {
    apiMocks.detectEnvironment.mockResolvedValue(createSnapshot())
    apiMocks.streamLogs.mockResolvedValue([])
    apiMocks.getOperationStatus.mockResolvedValue(createIdleTask())
    apiMocks.getOperationEvents.mockResolvedValue([])
    let resolveRevalidation!: (agents: Array<{
      id: string
      name: string
      identityName: string
      identityEmoji: string
      identitySource: 'identity' | 'config'
      workspace: string
      agentDir: string
      model: string
      bindings: number
      isDefault: boolean
      routes: string[]
    }>) => void
    const pendingRevalidation = new Promise<Array<{
      id: string
      name: string
      identityName: string
      identityEmoji: string
      identitySource: 'identity' | 'config'
      workspace: string
      agentDir: string
      model: string
      bindings: number
      isDefault: boolean
      routes: string[]
    }>>((resolve) => {
      resolveRevalidation = resolve
    })
    apiMocks.listOpenClawAgents
      .mockResolvedValueOnce([
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
          routes: ['default (no explicit rules)'],
        },
      ])
      .mockReturnValueOnce(pendingRevalidation)
    apiMocks.listOpenClawAgentBindings.mockResolvedValue([])
    apiMocks.listOpenClawSkills.mockResolvedValue({
      workspaceDir: '/Users/goya/.openclaw/workspace',
      managedSkillsDir: '/Users/goya/.openclaw/skills',
      skills: [],
    })
    apiMocks.createOpenClawAgent.mockResolvedValue({
      agentId: 'ops',
    })
    bizclawUpdaterMocks.getCurrentBizClawVersion.mockResolvedValue('0.1.8')
    apiMocks.onRuntimeLog.mockResolvedValue(() => {})
    apiMocks.onRuntimeStatus.mockResolvedValue(() => {})
    apiMocks.onOperationStatus.mockResolvedValue(() => {})
    apiMocks.onOperationEvent.mockResolvedValue(() => {})
    apiMocks.onConnectionTestEvent.mockResolvedValue(() => {})
    apiMocks.onRefreshRequested.mockResolvedValue(() => {})

    let model!: ReturnType<typeof useAppModel>
    const TestHarness = defineComponent({
      setup() {
        model = useAppModel()
        return () => h('div')
      },
    })

    host = document.createElement('div')
    document.body.appendChild(host)
    app = createApp(TestHarness)
    app.mount(host)

    await flushPromises()
    model.activeSection.value = 'agent'
    await flushPromises()

    let settled = false
    const createPromise = model.agentsState.createAgent({
      name: 'Ops',
      workspace: '/tmp/openclaw-ops',
      model: 'minimax-cn/MiniMax-M2.5',
      bindings: [],
    }).then(() => {
      settled = true
    })

    await Promise.resolve()
    await nextTick()
    await new Promise((resolve) => setTimeout(resolve, 0))

    expect(settled).toBe(true)
    expect(apiMocks.listOpenClawAgents).toHaveBeenCalledTimes(2)

    const completeAgentRevalidation = resolveRevalidation
    completeAgentRevalidation([
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
        routes: ['default (no explicit rules)'],
      },
      {
        id: 'ops',
        name: 'Ops',
        identityName: '值班',
        identityEmoji: '🛠',
        identitySource: 'config',
        workspace: '/tmp/openclaw-ops',
        agentDir: '/Users/goya/.openclaw/agents/ops/agent',
        model: 'minimax-cn/MiniMax-M2.5',
        bindings: 0,
        isDefault: false,
        routes: [],
      },
    ])
    await createPromise
    await flushPromises()
  })

  it('loads agent bindings lazily and does not refetch the same agent twice', async () => {
    apiMocks.detectEnvironment.mockResolvedValue(createSnapshot())
    apiMocks.streamLogs.mockResolvedValue([])
    apiMocks.getOperationStatus.mockResolvedValue(createIdleTask())
    apiMocks.getOperationEvents.mockResolvedValue([])
    apiMocks.listOpenClawAgents.mockResolvedValue([
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
        routes: ['default (no explicit rules)'],
      },
      {
        id: 'ops',
        name: 'Ops',
        identityName: '值班',
        identityEmoji: '🛠',
        identitySource: 'config',
        workspace: '/tmp/openclaw-ops',
        agentDir: '/Users/goya/.openclaw/agents/ops/agent',
        model: 'minimax-cn/MiniMax-M2.5',
        bindings: 0,
        isDefault: false,
        routes: [],
      },
    ])
    apiMocks.listOpenClawAgentBindings
      .mockResolvedValueOnce([
        {
          agentId: 'main',
          channel: 'telegram',
          accountId: null,
          description: 'telegram (default account)',
        },
      ])
      .mockResolvedValueOnce([])
    apiMocks.listOpenClawSkills.mockResolvedValue({
      workspaceDir: '/Users/goya/.openclaw/workspace',
      managedSkillsDir: '/Users/goya/.openclaw/skills',
      skills: [],
    })
    bizclawUpdaterMocks.getCurrentBizClawVersion.mockResolvedValue('0.1.8')
    apiMocks.onRuntimeLog.mockResolvedValue(() => {})
    apiMocks.onRuntimeStatus.mockResolvedValue(() => {})
    apiMocks.onOperationStatus.mockResolvedValue(() => {})
    apiMocks.onOperationEvent.mockResolvedValue(() => {})
    apiMocks.onConnectionTestEvent.mockResolvedValue(() => {})
    apiMocks.onRefreshRequested.mockResolvedValue(() => {})

    let model!: ReturnType<typeof useAppModel>
    const TestHarness = defineComponent({
      setup() {
        model = useAppModel()
        return () => h('div')
      },
    })

    host = document.createElement('div')
    document.body.appendChild(host)
    app = createApp(TestHarness)
    app.mount(host)

    await flushPromises()
    model.activeSection.value = 'agent'
    await flushPromises()

    await model.agentsState.selectAgent('ops')
    await flushPromises()
    await model.agentsState.selectAgent('ops')
    await flushPromises()

    expect(apiMocks.listOpenClawAgentBindings).toHaveBeenCalledTimes(2)
    expect(apiMocks.listOpenClawAgentBindings).toHaveBeenNthCalledWith(1, 'main')
    expect(apiMocks.listOpenClawAgentBindings).toHaveBeenNthCalledWith(2, 'ops')
    expect(model.agentsState.selectedAgentId.value).toBe('ops')
  })

  it('caches skill detail requests across repeated selections', async () => {
    apiMocks.detectEnvironment.mockResolvedValue(createSnapshot())
    apiMocks.streamLogs.mockResolvedValue([])
    apiMocks.getOperationStatus.mockResolvedValue(createIdleTask())
    apiMocks.getOperationEvents.mockResolvedValue([])
    apiMocks.listOpenClawAgents.mockResolvedValue([
      {
        id: 'main',
        name: 'Main',
        identityName: '霸天',
        identityEmoji: '🐯',
        identitySource: 'identity',
        workspace: '/Users/goya/.openclaw/workspace',
        agentDir: '/Users/goya/.openclaw/agents/main/agent',
        model: 'minimax-cn/MiniMax-M2.5',
        bindings: 0,
        isDefault: true,
        routes: [],
      },
    ])
    apiMocks.listOpenClawAgentBindings.mockResolvedValue([])
    apiMocks.listOpenClawSkills.mockResolvedValue({
      workspaceDir: '/Users/goya/.openclaw/workspace',
      managedSkillsDir: '/Users/goya/.openclaw/skills',
      skills: [
        {
          name: 'sonoscli',
          description: 'Control Sonos speakers.',
          eligible: true,
          disabled: false,
          blockedByAllowlist: false,
          source: 'openclaw-workspace',
          bundled: false,
          locationKind: 'workspaceLocal',
          canDelete: true,
          missing: {
            bins: [],
            anyBins: [],
            env: [],
            config: [],
            os: [],
          },
        },
        {
          name: 'coding-agent',
          description: 'Delegate coding tasks.',
          eligible: true,
          disabled: false,
          blockedByAllowlist: false,
          source: 'openclaw-bundled',
          bundled: true,
          locationKind: 'bundled',
          canDelete: false,
          missing: {
            bins: [],
            anyBins: [],
            env: [],
            config: [],
            os: [],
          },
        },
      ],
    })
    apiMocks.getOpenClawSkillInfo.mockResolvedValue({
      name: 'coding-agent',
      description: 'Delegate coding tasks.',
      source: 'openclaw-bundled',
      bundled: true,
      locationKind: 'bundled',
      canDelete: false,
      filePath: '/Users/goya/.openclaw/skills/coding-agent/SKILL.md',
      baseDir: '/Users/goya/.openclaw/skills/coding-agent',
      skillKey: 'coding-agent',
      homepage: null,
      always: false,
      disabled: false,
      blockedByAllowlist: false,
      eligible: true,
      requirements: {
        bins: [],
        anyBins: [],
        env: [],
        config: [],
        os: [],
      },
      missing: {
        bins: [],
        anyBins: [],
        env: [],
        config: [],
        os: [],
      },
      configChecks: [],
      install: [],
    })
    bizclawUpdaterMocks.getCurrentBizClawVersion.mockResolvedValue('0.1.8')
    apiMocks.onRuntimeLog.mockResolvedValue(() => {})
    apiMocks.onRuntimeStatus.mockResolvedValue(() => {})
    apiMocks.onOperationStatus.mockResolvedValue(() => {})
    apiMocks.onOperationEvent.mockResolvedValue(() => {})
    apiMocks.onConnectionTestEvent.mockResolvedValue(() => {})
    apiMocks.onRefreshRequested.mockResolvedValue(() => {})

    let model!: ReturnType<typeof useAppModel>
    const TestHarness = defineComponent({
      setup() {
        model = useAppModel()
        return () => h('div')
      },
    })

    host = document.createElement('div')
    document.body.appendChild(host)
    app = createApp(TestHarness)
    app.mount(host)

    await flushPromises()
    model.activeSection.value = 'skill'
    await flushPromises()

    await model.skillsState.selectSkill('coding-agent')
    await flushPromises()
    await model.skillsState.selectSkill('coding-agent')
    await flushPromises()

    expect(apiMocks.getOpenClawSkillInfo).toHaveBeenCalledTimes(1)
    expect(apiMocks.getOpenClawSkillInfo).toHaveBeenCalledWith('coding-agent')
    expect(model.skillsState.selectedSkillInfo.value?.name).toBe('coding-agent')
  })

  it('installs a searched skill and selects it after refreshing inventory', async () => {
    apiMocks.detectEnvironment.mockResolvedValue(createSnapshot())
    apiMocks.streamLogs.mockResolvedValue([])
    apiMocks.getOperationStatus.mockResolvedValue(createIdleTask())
    apiMocks.getOperationEvents.mockResolvedValue([])
    apiMocks.listOpenClawAgents.mockResolvedValue([
      {
        id: 'main',
        name: 'Main',
        identityName: '霸天',
        identityEmoji: '🐯',
        identitySource: 'identity',
        workspace: '/Users/goya/.openclaw/workspace',
        agentDir: '/Users/goya/.openclaw/agents/main/agent',
        model: 'minimax-cn/MiniMax-M2.5',
        bindings: 0,
        isDefault: true,
        routes: [],
      },
    ])
    apiMocks.listOpenClawAgentBindings.mockResolvedValue([])
    apiMocks.listOpenClawSkills
      .mockResolvedValueOnce({
        workspaceDir: '/Users/goya/.openclaw/workspace',
        managedSkillsDir: '/Users/goya/.openclaw/skills',
        skills: [],
      })
      .mockResolvedValueOnce({
        workspaceDir: '/Users/goya/.openclaw/workspace',
        managedSkillsDir: '/Users/goya/.openclaw/skills',
        skills: [
          {
            name: 'calendar',
            description: 'Calendar utilities.',
            eligible: true,
            disabled: false,
            blockedByAllowlist: false,
            source: 'openclaw-workspace',
            bundled: false,
            locationKind: 'workspaceLocal',
            canDelete: true,
            missing: {
              bins: [],
              anyBins: [],
              env: [],
              config: [],
              os: [],
            },
          },
        ],
      })
    apiMocks.installClawHubSkill.mockResolvedValue({
      slug: 'calendar',
      location: 'workspaceLocal',
    })
    apiMocks.getOpenClawSkillInfo.mockResolvedValue({
      name: 'calendar',
      description: 'Calendar utilities.',
      source: 'openclaw-workspace',
      bundled: false,
      locationKind: 'workspaceLocal',
      canDelete: true,
      filePath: '/Users/goya/.openclaw/workspace/skills/calendar/SKILL.md',
      baseDir: '/Users/goya/.openclaw/workspace/skills/calendar',
      skillKey: 'calendar',
      homepage: null,
      always: false,
      disabled: false,
      blockedByAllowlist: false,
      eligible: true,
      requirements: {
        bins: [],
        anyBins: [],
        env: [],
        config: [],
        os: [],
      },
      missing: {
        bins: [],
        anyBins: [],
        env: [],
        config: [],
        os: [],
      },
      configChecks: [],
      install: [],
    })
    bizclawUpdaterMocks.getCurrentBizClawVersion.mockResolvedValue('0.1.8')
    apiMocks.onRuntimeLog.mockResolvedValue(() => {})
    apiMocks.onRuntimeStatus.mockResolvedValue(() => {})
    apiMocks.onOperationStatus.mockResolvedValue(() => {})
    apiMocks.onOperationEvent.mockResolvedValue(() => {})
    apiMocks.onConnectionTestEvent.mockResolvedValue(() => {})
    apiMocks.onRefreshRequested.mockResolvedValue(() => {})

    let model!: ReturnType<typeof useAppModel>
    const TestHarness = defineComponent({
      setup() {
        model = useAppModel()
        return () => h('div')
      },
    })

    host = document.createElement('div')
    document.body.appendChild(host)
    app = createApp(TestHarness)
    app.mount(host)

    await flushPromises()
    model.activeSection.value = 'skill'
    await flushPromises()

    await model.skillsState.installSkill({
      slug: 'calendar',
      location: 'workspaceLocal',
    })
    await flushPromises()

    expect(apiMocks.installClawHubSkill).toHaveBeenCalledWith({
      slug: 'calendar',
      location: 'workspaceLocal',
    })
    expect(apiMocks.listOpenClawSkills).toHaveBeenCalledTimes(2)
    expect(model.skillsState.selectedSkillName.value).toBe('calendar')
    expect(model.skillsState.selectedSkillInfo.value?.name).toBe('calendar')
  })

  it('keeps skill install mutations responsive while the inventory refresh continues in the background', async () => {
    apiMocks.detectEnvironment.mockResolvedValue(createSnapshot())
    apiMocks.streamLogs.mockResolvedValue([])
    apiMocks.getOperationStatus.mockResolvedValue(createIdleTask())
    apiMocks.getOperationEvents.mockResolvedValue([])
    let resolveRevalidation!: (inventory: {
      workspaceDir: string
      managedSkillsDir: string
      skills: Array<{
        name: string
        description: string
        eligible: boolean
        disabled: boolean
        blockedByAllowlist: boolean
        source: 'openclaw-workspace' | 'openclaw-bundled' | 'user-managed' | 'other'
        bundled: boolean
        locationKind: 'workspaceLocal' | 'sharedLocal' | 'bundled' | 'external'
        canDelete: boolean
        missing: {
          bins: string[]
          anyBins: string[]
          env: string[]
          config: string[]
          os: string[]
        }
      }>
    }) => void
    const pendingRevalidation = new Promise<{
      workspaceDir: string
      managedSkillsDir: string
      skills: Array<{
        name: string
        description: string
        eligible: boolean
        disabled: boolean
        blockedByAllowlist: boolean
        source: 'openclaw-workspace' | 'openclaw-bundled' | 'user-managed' | 'other'
        bundled: boolean
        locationKind: 'workspaceLocal' | 'sharedLocal' | 'bundled' | 'external'
        canDelete: boolean
        missing: {
          bins: string[]
          anyBins: string[]
          env: string[]
          config: string[]
          os: string[]
        }
      }>
    }>((resolve) => {
      resolveRevalidation = resolve
    })
    apiMocks.listOpenClawAgents.mockResolvedValue([
      {
        id: 'main',
        name: 'Main',
        identityName: '霸天',
        identityEmoji: '🐯',
        identitySource: 'identity',
        workspace: '/Users/goya/.openclaw/workspace',
        agentDir: '/Users/goya/.openclaw/agents/main/agent',
        model: 'minimax-cn/MiniMax-M2.5',
        bindings: 0,
        isDefault: true,
        routes: [],
      },
    ])
    apiMocks.listOpenClawAgentBindings.mockResolvedValue([])
    apiMocks.listOpenClawSkills
      .mockResolvedValueOnce({
        workspaceDir: '/Users/goya/.openclaw/workspace',
        managedSkillsDir: '/Users/goya/.openclaw/skills',
        skills: [],
      })
      .mockReturnValueOnce(pendingRevalidation)
    apiMocks.installClawHubSkill.mockResolvedValue({
      skillName: 'calendar',
      slug: 'calendar',
      location: 'workspaceLocal',
    })
    apiMocks.getOpenClawSkillInfo.mockResolvedValue({
      name: 'calendar',
      description: 'Calendar utilities.',
      source: 'openclaw-workspace',
      bundled: false,
      locationKind: 'workspaceLocal',
      canDelete: true,
      filePath: '/Users/goya/.openclaw/workspace/skills/calendar/SKILL.md',
      baseDir: '/Users/goya/.openclaw/workspace/skills/calendar',
      skillKey: 'calendar',
      homepage: null,
      always: false,
      disabled: false,
      blockedByAllowlist: false,
      eligible: true,
      requirements: {
        bins: [],
        anyBins: [],
        env: [],
        config: [],
        os: [],
      },
      missing: {
        bins: [],
        anyBins: [],
        env: [],
        config: [],
        os: [],
      },
      configChecks: [],
      install: [],
    })
    bizclawUpdaterMocks.getCurrentBizClawVersion.mockResolvedValue('0.1.8')
    apiMocks.onRuntimeLog.mockResolvedValue(() => {})
    apiMocks.onRuntimeStatus.mockResolvedValue(() => {})
    apiMocks.onOperationStatus.mockResolvedValue(() => {})
    apiMocks.onOperationEvent.mockResolvedValue(() => {})
    apiMocks.onConnectionTestEvent.mockResolvedValue(() => {})
    apiMocks.onRefreshRequested.mockResolvedValue(() => {})

    let model!: ReturnType<typeof useAppModel>
    const TestHarness = defineComponent({
      setup() {
        model = useAppModel()
        return () => h('div')
      },
    })

    host = document.createElement('div')
    document.body.appendChild(host)
    app = createApp(TestHarness)
    app.mount(host)

    await flushPromises()
    model.activeSection.value = 'skill'
    await flushPromises()

    let settled = false
    const installPromise = model.skillsState.installSkill({
      slug: 'calendar',
      location: 'workspaceLocal',
    }).then(() => {
      settled = true
    })

    await Promise.resolve()
    await nextTick()
    await new Promise((resolve) => setTimeout(resolve, 0))

    expect(settled).toBe(true)
    expect(apiMocks.listOpenClawSkills).toHaveBeenCalledTimes(2)

    const completeSkillRevalidation = resolveRevalidation
    completeSkillRevalidation({
      workspaceDir: '/Users/goya/.openclaw/workspace',
      managedSkillsDir: '/Users/goya/.openclaw/skills',
      skills: [
        {
          name: 'calendar',
          description: 'Calendar utilities.',
          eligible: true,
          disabled: false,
          blockedByAllowlist: false,
          source: 'openclaw-workspace',
          bundled: false,
          locationKind: 'workspaceLocal',
          canDelete: true,
          missing: {
            bins: [],
            anyBins: [],
            env: [],
            config: [],
            os: [],
          },
        },
      ],
    })
    await installPromise
    await flushPromises()
  })

  it('keeps save-and-test available while environment detection is still pending', async () => {
    apiMocks.detectEnvironment.mockReturnValue(new Promise(() => {}))
    apiMocks.streamLogs.mockResolvedValue([])
    apiMocks.getOperationStatus.mockResolvedValue(createIdleTask())
    apiMocks.getOperationEvents.mockResolvedValue([])
    bizclawUpdaterMocks.getCurrentBizClawVersion.mockResolvedValue('0.1.8')
    apiMocks.onRuntimeLog.mockResolvedValue(() => {})
    apiMocks.onRuntimeStatus.mockResolvedValue(() => {})
    apiMocks.onOperationStatus.mockResolvedValue(() => {})
    apiMocks.onOperationEvent.mockResolvedValue(() => {})
    apiMocks.onConnectionTestEvent.mockResolvedValue(() => {})
    apiMocks.onRefreshRequested.mockResolvedValue(() => {})

    let model!: ReturnType<typeof useAppModel>
    const TestHarness = defineComponent({
      setup() {
        model = useAppModel()
        return () => h('div')
      },
    })

    host = document.createElement('div')
    document.body.appendChild(host)
    app = createApp(TestHarness)
    app.mount(host)

    await nextTick()

    Object.assign(model.companyProfile, {
      companyName: 'OpenClaw',
      sshHost: 'gateway.example.com',
      sshPort: '22',
      sshUser: 'root',
      remoteBindHost: '127.0.0.1',
      remoteBindPort: '8080',
      localPort: '18080',
    })
    model.userProfile.displayName = 'Tester'
    model.tokenInput.value = 'token'

    expect(model.canSaveProfile.value).toBe(true)
    expect(model.connectionTestDisabledReason.value).toBeNull()
    expect(model.canTestConnection.value).toBe(true)
  })

  it('starts the hosted runtime without waiting for a pending environment refresh', async () => {
    apiMocks.detectEnvironment.mockReturnValue(new Promise(() => {}))
    apiMocks.streamLogs.mockResolvedValue([])
    apiMocks.getOperationStatus.mockResolvedValue(createIdleTask())
    apiMocks.getOperationEvents.mockResolvedValue([])
    apiMocks.startRuntime.mockResolvedValue({
      phase: 'connecting',
      sshConnected: false,
      nodeConnected: false,
      gatewayConnected: false,
      lastError: null,
    })
    bizclawUpdaterMocks.getCurrentBizClawVersion.mockResolvedValue('0.1.8')
    apiMocks.onRuntimeLog.mockResolvedValue(() => {})
    apiMocks.onRuntimeStatus.mockResolvedValue(() => {})
    apiMocks.onOperationStatus.mockResolvedValue(() => {})
    apiMocks.onOperationEvent.mockResolvedValue(() => {})
    apiMocks.onConnectionTestEvent.mockResolvedValue(() => {})
    apiMocks.onRefreshRequested.mockResolvedValue(() => {})

    let model!: ReturnType<typeof useAppModel>
    const TestHarness = defineComponent({
      setup() {
        model = useAppModel()
        return () => h('div')
      },
    })

    host = document.createElement('div')
    document.body.appendChild(host)
    app = createApp(TestHarness)
    app.mount(host)

    await nextTick()
    await model.startHostedRuntime()

    expect(apiMocks.startRuntime).toHaveBeenCalledTimes(1)
  })

  it('hydrates ui preferences from the environment and persists locale changes', async () => {
    apiMocks.detectEnvironment
      .mockResolvedValueOnce(createSnapshot({
        uiPreferences: {
          theme: 'dark',
          locale: 'en-US',
          sidebarCollapsed: false,
        },
      }))
      .mockResolvedValue(createSnapshot({
        uiPreferences: {
          theme: 'dark',
          locale: 'zh-CN',
          sidebarCollapsed: false,
        },
      }))
    apiMocks.streamLogs.mockResolvedValue([])
    apiMocks.getOperationStatus.mockResolvedValue(createIdleTask())
    apiMocks.getOperationEvents.mockResolvedValue([])
    apiMocks.saveUiPreferences.mockResolvedValue({
      theme: 'dark',
      locale: 'zh-CN',
      sidebarCollapsed: false,
    })
    bizclawUpdaterMocks.getCurrentBizClawVersion.mockResolvedValue('0.1.8')
    apiMocks.onRuntimeLog.mockResolvedValue(() => {})
    apiMocks.onRuntimeStatus.mockResolvedValue(() => {})
    apiMocks.onOperationStatus.mockResolvedValue(() => {})
    apiMocks.onOperationEvent.mockResolvedValue(() => {})
    apiMocks.onConnectionTestEvent.mockResolvedValue(() => {})
    apiMocks.onRefreshRequested.mockResolvedValue(() => {})

    let model!: ReturnType<typeof useAppModel>
    const TestHarness = defineComponent({
      setup() {
        model = useAppModel()
        return () => h('div')
      },
    })

    host = document.createElement('div')
    document.body.appendChild(host)
    app = createApp(TestHarness)
    app.mount(host)

    await flushPromises()

    expect(model.uiPreferences.value).toEqual({
      theme: 'dark',
      locale: 'en-US',
      sidebarCollapsed: false,
    })
    expect(model.tokenStateLabel.value).toBe('Token saved')

    await model.setLocale('zh-CN')
    await flushPromises()

    expect(apiMocks.saveUiPreferences).toHaveBeenCalledWith({
      theme: 'dark',
      locale: 'zh-CN',
      sidebarCollapsed: false,
    })
    expect(model.uiPreferences.value.locale).toBe('zh-CN')
    expect(model.tokenStateLabel.value).toBe('Token 已保存')
  })

  it('applies and tracks the system theme when the preference is set to system', async () => {
    const mediaQuery = createMatchMediaMock(true)
    window.matchMedia = vi.fn().mockImplementation((query: string) => {
      expect(query).toBe('(prefers-color-scheme: dark)')
      return mediaQuery
    })

    apiMocks.detectEnvironment.mockResolvedValue(createSnapshot({
      uiPreferences: {
        theme: 'system',
        locale: 'zh-CN',
        sidebarCollapsed: false,
      },
    }))
    apiMocks.streamLogs.mockResolvedValue([])
    apiMocks.getOperationStatus.mockResolvedValue(createIdleTask())
    apiMocks.getOperationEvents.mockResolvedValue([])
    apiMocks.saveUiPreferences.mockImplementation(async (preferences) => preferences)
    bizclawUpdaterMocks.getCurrentBizClawVersion.mockResolvedValue('0.1.8')
    apiMocks.onRuntimeLog.mockResolvedValue(() => {})
    apiMocks.onRuntimeStatus.mockResolvedValue(() => {})
    apiMocks.onOperationStatus.mockResolvedValue(() => {})
    apiMocks.onOperationEvent.mockResolvedValue(() => {})
    apiMocks.onConnectionTestEvent.mockResolvedValue(() => {})
    apiMocks.onRefreshRequested.mockResolvedValue(() => {})

    let model!: ReturnType<typeof useAppModel>
    const TestHarness = defineComponent({
      setup() {
        model = useAppModel()
        return () => h('div')
      },
    })

    host = document.createElement('div')
    document.body.appendChild(host)
    app = createApp(TestHarness)
    app.mount(host)

    await flushPromises()

    expect(model.uiPreferences.value.theme).toBe('system')
    expect(document.documentElement.dataset.theme).toBe('dark')
    expect(document.documentElement.style.colorScheme).toBe('dark')

    mediaQuery.setMatches(false)
    await flushPromises()

    expect(document.documentElement.dataset.theme).toBe('light')
    expect(document.documentElement.style.colorScheme).toBe('light')

    await model.setTheme('light')
    await flushPromises()

    mediaQuery.setMatches(true)
    await flushPromises()

    expect(apiMocks.saveUiPreferences).toHaveBeenCalledWith({
      theme: 'light',
      locale: 'zh-CN',
      sidebarCollapsed: false,
    })
    expect(document.documentElement.dataset.theme).toBe('light')
  })

  it('keeps english locale after saving localized state without forcing another environment refresh', async () => {
    apiMocks.detectEnvironment
      .mockResolvedValueOnce(createSnapshot({
        uiPreferences: {
          theme: 'light',
          locale: 'zh-CN',
          sidebarCollapsed: false,
        },
      }))
      .mockResolvedValue(createSnapshot({
        uiPreferences: {
          theme: 'light',
          locale: 'en-US',
          sidebarCollapsed: false,
        },
      }))
    apiMocks.streamLogs.mockResolvedValue([])
    apiMocks.getOperationStatus.mockResolvedValue(createIdleTask())
    apiMocks.getOperationEvents.mockResolvedValue([])
    apiMocks.saveUiPreferences.mockResolvedValue({
      theme: 'light',
      locale: 'en-US',
      sidebarCollapsed: false,
    })
    bizclawUpdaterMocks.getCurrentBizClawVersion.mockResolvedValue('0.1.8')
    apiMocks.onRuntimeLog.mockResolvedValue(() => {})
    apiMocks.onRuntimeStatus.mockResolvedValue(() => {})
    apiMocks.onOperationStatus.mockResolvedValue(() => {})
    apiMocks.onOperationEvent.mockResolvedValue(() => {})
    apiMocks.onConnectionTestEvent.mockResolvedValue(() => {})
    apiMocks.onRefreshRequested.mockResolvedValue(() => {})

    let model!: ReturnType<typeof useAppModel>
    const TestHarness = defineComponent({
      setup() {
        model = useAppModel()
        return () => h('div')
      },
    })

    host = document.createElement('div')
    document.body.appendChild(host)
    app = createApp(TestHarness)
    app.mount(host)

    await flushPromises()
    await model.setLocale('en-US')
    await flushPromises()

    expect(model.uiPreferences.value.locale).toBe('en-US')
    expect(model.tokenStateLabel.value).toBe('Token saved')
    expect(apiMocks.detectEnvironment).toHaveBeenCalledTimes(1)
    expect(apiMocks.streamLogs).toHaveBeenCalledTimes(1)
    expect(apiMocks.getOperationStatus).toHaveBeenCalledTimes(1)
    expect(apiMocks.getOperationEvents).toHaveBeenCalledTimes(1)
  })

  it('refreshes saved environment without reloading logs or operation history after save only', async () => {
    apiMocks.detectEnvironment
      .mockResolvedValueOnce(createSnapshot({
        savedSettings: null,
      }))
      .mockResolvedValue(createSnapshot({
        savedSettings: {
          companyProfile: {
            sshHost: 'gateway.example.com',
            sshUser: 'bizclaw',
            localPort: 18889,
            remoteBindHost: '127.0.0.1',
            remoteBindPort: 18789,
          },
          userProfile: {
            displayName: 'BizClaw',
            autoConnect: true,
            runInBackground: true,
          },
          targetProfile: {
            wslDistro: 'Ubuntu',
          },
        },
      }))
    apiMocks.streamLogs.mockResolvedValue([])
    apiMocks.getOperationStatus.mockResolvedValue(createIdleTask())
    apiMocks.getOperationEvents.mockResolvedValue([])
    apiMocks.saveProfile.mockResolvedValue({
      companyProfile: {
        sshHost: 'gateway.example.com',
        sshUser: 'bizclaw',
        localPort: 18889,
        remoteBindHost: '127.0.0.1',
        remoteBindPort: 18789,
      },
      userProfile: {
        displayName: 'BizClaw',
        autoConnect: true,
        runInBackground: true,
      },
      targetProfile: {
        wslDistro: 'Ubuntu',
      },
    })
    bizclawUpdaterMocks.getCurrentBizClawVersion.mockResolvedValue('0.1.8')
    apiMocks.onRuntimeLog.mockResolvedValue(() => {})
    apiMocks.onRuntimeStatus.mockResolvedValue(() => {})
    apiMocks.onOperationStatus.mockResolvedValue(() => {})
    apiMocks.onOperationEvent.mockResolvedValue(() => {})
    apiMocks.onConnectionTestEvent.mockResolvedValue(() => {})
    apiMocks.onRefreshRequested.mockResolvedValue(() => {})

    let model!: ReturnType<typeof useAppModel>
    const TestHarness = defineComponent({
      setup() {
        model = useAppModel()
        return () => h('div')
      },
    })

    host = document.createElement('div')
    document.body.appendChild(host)
    app = createApp(TestHarness)
    app.mount(host)

    await flushPromises()
    await model.saveOnly()
    await flushPromises()
    await new Promise((resolve) => setTimeout(resolve, 0))
    await flushPromises()

    expect(apiMocks.detectEnvironment).toHaveBeenCalledTimes(2)
    expect(apiMocks.streamLogs).toHaveBeenCalledTimes(1)
    expect(apiMocks.getOperationStatus).toHaveBeenCalledTimes(1)
    expect(apiMocks.getOperationEvents).toHaveBeenCalledTimes(1)
  })

  it('refreshes the environment only once after save-and-test finishes', async () => {
    apiMocks.detectEnvironment
      .mockResolvedValueOnce(createSnapshot({
        savedSettings: null,
      }))
      .mockResolvedValue(createSnapshot({
        savedSettings: {
          companyProfile: {
            sshHost: 'gateway.example.com',
            sshUser: 'bizclaw',
            localPort: 18889,
            remoteBindHost: '127.0.0.1',
            remoteBindPort: 18789,
          },
          userProfile: {
            displayName: 'BizClaw',
            autoConnect: true,
            runInBackground: true,
          },
          targetProfile: {
            wslDistro: 'Ubuntu',
          },
        },
      }))
    apiMocks.streamLogs.mockResolvedValue([])
    apiMocks.getOperationStatus.mockResolvedValue(createIdleTask())
    apiMocks.getOperationEvents.mockResolvedValue([])
    apiMocks.saveProfile.mockResolvedValue({
      companyProfile: {
        sshHost: 'gateway.example.com',
        sshUser: 'bizclaw',
        localPort: 18889,
        remoteBindHost: '127.0.0.1',
        remoteBindPort: 18789,
      },
      userProfile: {
        displayName: 'BizClaw',
        autoConnect: true,
        runInBackground: true,
      },
      targetProfile: {
        wslDistro: 'Ubuntu',
      },
    })
    apiMocks.saveAndTestConnection.mockResolvedValue({
      success: true,
      step: 'gatewayProbe',
      summary: 'Connection is ready.',
      stdout: 'ok',
      stderr: '',
    })
    bizclawUpdaterMocks.getCurrentBizClawVersion.mockResolvedValue('0.1.8')
    apiMocks.onRuntimeLog.mockResolvedValue(() => {})
    apiMocks.onRuntimeStatus.mockResolvedValue(() => {})
    apiMocks.onOperationStatus.mockResolvedValue(() => {})
    apiMocks.onOperationEvent.mockResolvedValue(() => {})
    apiMocks.onConnectionTestEvent.mockResolvedValue(() => {})
    apiMocks.onRefreshRequested.mockResolvedValue(() => {})

    let model!: ReturnType<typeof useAppModel>
    const TestHarness = defineComponent({
      setup() {
        model = useAppModel()
        return () => h('div')
      },
    })

    host = document.createElement('div')
    document.body.appendChild(host)
    app = createApp(TestHarness)
    app.mount(host)

    await flushPromises()
    await model.saveAndTest()
    await flushPromises()

    expect(apiMocks.saveAndTestConnection).toHaveBeenCalledTimes(1)
    expect(apiMocks.detectEnvironment).toHaveBeenCalledTimes(2)
  })

  it('allows closing the connection test modal while the test is still running', async () => {
    apiMocks.detectEnvironment.mockResolvedValue(createSnapshot())
    apiMocks.streamLogs.mockResolvedValue([])
    apiMocks.getOperationStatus.mockResolvedValue(createIdleTask())
    apiMocks.getOperationEvents.mockResolvedValue([])
    let resolveTest!: (result: {
      success: boolean
      step: 'save' | 'sshTunnel' | 'gatewayProbe'
      summary: string
      stdout: string
      stderr: string
    }) => void
    apiMocks.saveAndTestConnection.mockImplementation(
      () =>
        new Promise((resolve) => {
          resolveTest = resolve
        }),
    )
    bizclawUpdaterMocks.getCurrentBizClawVersion.mockResolvedValue('0.1.8')
    apiMocks.onRuntimeLog.mockResolvedValue(() => {})
    apiMocks.onRuntimeStatus.mockResolvedValue(() => {})
    apiMocks.onOperationStatus.mockResolvedValue(() => {})
    apiMocks.onOperationEvent.mockResolvedValue(() => {})
    apiMocks.onConnectionTestEvent.mockResolvedValue(() => {})
    apiMocks.onRefreshRequested.mockResolvedValue(() => {})

    let model!: ReturnType<typeof useAppModel>
    const TestHarness = defineComponent({
      setup() {
        model = useAppModel()
        return () => h('div')
      },
    })

    host = document.createElement('div')
    document.body.appendChild(host)
    app = createApp(TestHarness)
    app.mount(host)

    await flushPromises()
    const pending = model.saveAndTest()
    await flushPromises()

    expect(model.connectionTestModal.open).toBe(true)
    expect(model.connectionTestModal.phase).toBe('running')

    model.closeConnectionTestModal()
    expect(model.connectionTestModal.open).toBe(false)
    expect(model.connectionTestInlineVisible.value).toBe(true)
    expect(model.connectionTestInlinePhase.value).toBe('running')

    resolveTest({
      success: true,
      step: 'gatewayProbe',
      summary: 'Connection is ready.',
      stdout: 'ok',
      stderr: '',
    })
    await pending
    await flushPromises()

    expect(model.connectionTestModal.open).toBe(false)
    expect(model.connectionTestInlineVisible.value).toBe(true)
    expect(model.connectionTestInlinePhase.value).toBe('success')
    expect(model.connectionTestInlineSummary.value).toBe('Connection is ready.')
    expect(model.connectionTestInlineResult.value?.stdout).toBe('ok')
  })

  it('allows reopening and closing the connection test modal after closing a previous run', async () => {
    apiMocks.detectEnvironment.mockResolvedValue(createSnapshot())
    apiMocks.streamLogs.mockResolvedValue([])
    apiMocks.getOperationStatus.mockResolvedValue(createIdleTask())
    apiMocks.getOperationEvents.mockResolvedValue([])
    const pendingTests: Array<(result: {
      success: boolean
      step: 'save' | 'sshTunnel' | 'gatewayProbe'
      summary: string
      stdout: string
      stderr: string
    }) => void> = []
    apiMocks.saveAndTestConnection.mockImplementation(
      () =>
        new Promise((resolve) => {
          pendingTests.push(resolve)
        }),
    )
    bizclawUpdaterMocks.getCurrentBizClawVersion.mockResolvedValue('0.1.8')
    apiMocks.onRuntimeLog.mockResolvedValue(() => {})
    apiMocks.onRuntimeStatus.mockResolvedValue(() => {})
    apiMocks.onOperationStatus.mockResolvedValue(() => {})
    apiMocks.onOperationEvent.mockResolvedValue(() => {})
    apiMocks.onConnectionTestEvent.mockResolvedValue(() => {})
    apiMocks.onRefreshRequested.mockResolvedValue(() => {})

    let model!: ReturnType<typeof useAppModel>
    const TestHarness = defineComponent({
      setup() {
        model = useAppModel()
        return () => h('div')
      },
    })

    host = document.createElement('div')
    document.body.appendChild(host)
    app = createApp(TestHarness)
    app.mount(host)

    await flushPromises()

    const firstPending = model.saveAndTest()
    await flushPromises()
    model.closeConnectionTestModal()
    expect(model.connectionTestModal.open).toBe(false)

    pendingTests.shift()?.({
      success: true,
      step: 'gatewayProbe',
      summary: 'First connection is ready.',
      stdout: 'ok',
      stderr: '',
    })
    await firstPending
    await flushPromises()

    expect(model.connectionTestModal.open).toBe(false)
    expect(model.connectionTestModal.phase).toBe('idle')
    expect(model.connectionTestInlinePhase.value).toBe('success')

    const secondPending = model.saveAndTest()
    await flushPromises()

    expect(model.connectionTestModal.open).toBe(true)
    expect(model.connectionTestModal.phase).toBe('running')

    model.closeConnectionTestModal()
    expect(model.connectionTestModal.open).toBe(false)
    expect(model.connectionTestInlinePhase.value).toBe('running')

    pendingTests.shift()?.({
      success: true,
      step: 'gatewayProbe',
      summary: 'Second connection is ready.',
      stdout: 'ok',
      stderr: '',
    })
    await secondPending
    await flushPromises()

    expect(model.connectionTestModal.open).toBe(false)
    expect(model.connectionTestModal.phase).toBe('idle')
    expect(model.connectionTestInlineSummary.value).toBe('Second connection is ready.')
  })

  it('does not wait for environment refresh before finishing a successful connection test', async () => {
    apiMocks.detectEnvironment
      .mockResolvedValueOnce(createSnapshot())
      .mockImplementationOnce(
        () =>
          new Promise(() => {
            // Keep the forced refresh pending to verify saveAndTest resolves first.
          }),
      )
    apiMocks.streamLogs.mockResolvedValue([])
    apiMocks.getOperationStatus.mockResolvedValue(createIdleTask())
    apiMocks.getOperationEvents.mockResolvedValue([])
    apiMocks.saveAndTestConnection.mockResolvedValue({
      success: true,
      step: 'gatewayProbe',
      summary: 'Connection is ready.',
      stdout: 'ok',
      stderr: '',
    })
    bizclawUpdaterMocks.getCurrentBizClawVersion.mockResolvedValue('0.1.8')
    apiMocks.onRuntimeLog.mockResolvedValue(() => {})
    apiMocks.onRuntimeStatus.mockResolvedValue(() => {})
    apiMocks.onOperationStatus.mockResolvedValue(() => {})
    apiMocks.onOperationEvent.mockResolvedValue(() => {})
    apiMocks.onConnectionTestEvent.mockResolvedValue(() => {})
    apiMocks.onRefreshRequested.mockResolvedValue(() => {})

    let model!: ReturnType<typeof useAppModel>
    const TestHarness = defineComponent({
      setup() {
        model = useAppModel()
        return () => h('div')
      },
    })

    host = document.createElement('div')
    document.body.appendChild(host)
    app = createApp(TestHarness)
    app.mount(host)

    await flushPromises()
    await expect(model.saveAndTest()).resolves.toBeUndefined()
    await flushPromises()

    expect(model.connectionTestModal.phase).toBe('success')
    expect(apiMocks.detectEnvironment).toHaveBeenCalledTimes(2)
  })

  it('stops the hosted runtime without forcing a redundant environment refresh', async () => {
    apiMocks.detectEnvironment.mockResolvedValue(createSnapshot({
      runtimeStatus: {
        phase: 'running',
        sshConnected: true,
        nodeConnected: true,
        gatewayConnected: true,
        lastError: null,
      },
    }))
    apiMocks.streamLogs.mockResolvedValue([])
    apiMocks.getOperationStatus.mockResolvedValue(createIdleTask())
    apiMocks.getOperationEvents.mockResolvedValue([])
    apiMocks.stopRuntime.mockResolvedValue({
      phase: 'configured',
      sshConnected: false,
      nodeConnected: false,
      gatewayConnected: false,
      lastError: null,
    })
    bizclawUpdaterMocks.getCurrentBizClawVersion.mockResolvedValue('0.1.8')
    apiMocks.onRuntimeLog.mockResolvedValue(() => {})
    apiMocks.onRuntimeStatus.mockResolvedValue(() => {})
    apiMocks.onOperationStatus.mockResolvedValue(() => {})
    apiMocks.onOperationEvent.mockResolvedValue(() => {})
    apiMocks.onConnectionTestEvent.mockResolvedValue(() => {})
    apiMocks.onRefreshRequested.mockResolvedValue(() => {})

    let model!: ReturnType<typeof useAppModel>
    const TestHarness = defineComponent({
      setup() {
        model = useAppModel()
        return () => h('div')
      },
    })

    host = document.createElement('div')
    document.body.appendChild(host)
    app = createApp(TestHarness)
    app.mount(host)

    await flushPromises()
    await model.stopHostedRuntime()
    await flushPromises()

    expect(apiMocks.stopRuntime).toHaveBeenCalledTimes(1)
    expect(apiMocks.streamLogs).toHaveBeenCalledTimes(2)
    expect(apiMocks.detectEnvironment).toHaveBeenCalledTimes(1)
    expect(model.environment.value?.runtimeStatus.phase).toBe('configured')
  })

  it('checks for BizClaw updates only when requested and exposes the available release', async () => {
    apiMocks.detectEnvironment.mockResolvedValue(createSnapshot())
    apiMocks.streamLogs.mockResolvedValue([])
    apiMocks.getOperationStatus.mockResolvedValue(createIdleTask())
    apiMocks.getOperationEvents.mockResolvedValue([])
    bizclawUpdaterMocks.getCurrentBizClawVersion.mockResolvedValue('0.1.8')
    bizclawUpdaterMocks.checkForBizClawUpdate.mockResolvedValue({
      version: '0.1.9',
      body: 'Bug fixes',
      date: '2026-03-14T00:00:00.000Z',
      downloadAndInstall: vi.fn(),
    })
    apiMocks.onRuntimeLog.mockResolvedValue(() => {})
    apiMocks.onRuntimeStatus.mockResolvedValue(() => {})
    apiMocks.onOperationStatus.mockResolvedValue(() => {})
    apiMocks.onOperationEvent.mockResolvedValue(() => {})
    apiMocks.onConnectionTestEvent.mockResolvedValue(() => {})
    apiMocks.onRefreshRequested.mockResolvedValue(() => {})

    let model!: ReturnType<typeof useAppModel>
    const TestHarness = defineComponent({
      setup() {
        model = useAppModel()
        return () => h('div')
      },
    })

    host = document.createElement('div')
    document.body.appendChild(host)
    app = createApp(TestHarness)
    app.mount(host)

    await flushPromises()
    await model.checkBizClawUpdates()
    await flushPromises()

    expect(bizclawUpdaterMocks.checkForBizClawUpdate).toHaveBeenCalledTimes(1)
    expect(model.bizclawUpdate.value.phase).toBe('available')
    expect(model.bizclawUpdate.value.currentVersion).toBe('0.1.8')
    expect(model.bizclawUpdate.value.latestVersion).toBe('0.1.9')
    expect(model.bizclawUpdate.value.releaseNotes).toBe('Bug fixes')
  })

  it('keeps the current section when checking BizClaw updates from settings', async () => {
    apiMocks.detectEnvironment.mockResolvedValue(createSnapshot())
    apiMocks.streamLogs.mockResolvedValue([])
    apiMocks.getOperationStatus.mockResolvedValue(createIdleTask())
    apiMocks.getOperationEvents.mockResolvedValue([])
    bizclawUpdaterMocks.getCurrentBizClawVersion.mockResolvedValue('0.1.8')
    bizclawUpdaterMocks.checkForBizClawUpdate.mockResolvedValue(null)
    apiMocks.onRuntimeLog.mockResolvedValue(() => {})
    apiMocks.onRuntimeStatus.mockResolvedValue(() => {})
    apiMocks.onOperationStatus.mockResolvedValue(() => {})
    apiMocks.onOperationEvent.mockResolvedValue(() => {})
    apiMocks.onConnectionTestEvent.mockResolvedValue(() => {})
    apiMocks.onRefreshRequested.mockResolvedValue(() => {})

    let model!: ReturnType<typeof useAppModel>
    const TestHarness = defineComponent({
      setup() {
        model = useAppModel()
        return () => h('div')
      },
    })

    host = document.createElement('div')
    document.body.appendChild(host)
    app = createApp(TestHarness)
    app.mount(host)

    await flushPromises()
    model.activeSection.value = 'settings'

    await model.checkBizClawUpdates()
    await flushPromises()

    expect(model.activeSection.value).toBe('settings')
  })

  it('shows an actionable BizClaw updater error when the release manifest is missing', async () => {
    apiMocks.detectEnvironment.mockResolvedValue(createSnapshot())
    apiMocks.streamLogs.mockResolvedValue([])
    apiMocks.getOperationStatus.mockResolvedValue(createIdleTask())
    apiMocks.getOperationEvents.mockResolvedValue([])
    bizclawUpdaterMocks.getCurrentBizClawVersion.mockResolvedValue('0.1.8')
    bizclawUpdaterMocks.describeBizClawUpdaterError.mockReturnValueOnce(
      '无法读取 GitHub Release 的更新清单，请确认最新 Release 已上传 latest.json、签名文件和对应安装包。',
    )
    bizclawUpdaterMocks.checkForBizClawUpdate.mockRejectedValue(
      new Error('Could not fetch a valid release JSON from the remote'),
    )
    apiMocks.onRuntimeLog.mockResolvedValue(() => {})
    apiMocks.onRuntimeStatus.mockResolvedValue(() => {})
    apiMocks.onOperationStatus.mockResolvedValue(() => {})
    apiMocks.onOperationEvent.mockResolvedValue(() => {})
    apiMocks.onConnectionTestEvent.mockResolvedValue(() => {})
    apiMocks.onRefreshRequested.mockResolvedValue(() => {})

    let model!: ReturnType<typeof useAppModel>
    const TestHarness = defineComponent({
      setup() {
        model = useAppModel()
        return () => h('div')
      },
    })

    host = document.createElement('div')
    document.body.appendChild(host)
    app = createApp(TestHarness)
    app.mount(host)

    await flushPromises()
    await model.checkBizClawUpdates()
    await flushPromises()

    expect(model.bizclawUpdate.value.phase).toBe('error')
    expect(model.bizclawUpdate.value.errorMessage).toContain('latest.json')
    expect(model.bizclawUpdate.value.errorMessage).toContain('GitHub Release')
  })

  it('installs an available BizClaw update and keeps the restart prompt visible when deferred', async () => {
    const downloadAndInstall = vi.fn(async (handler?: (event: { event: string, data?: { contentLength?: number, chunkLength?: number } }) => void) => {
      handler?.({ event: 'Started', data: { contentLength: 1024 } })
      handler?.({ event: 'Progress', data: { chunkLength: 256 } })
      handler?.({ event: 'Finished' })
    })

    apiMocks.detectEnvironment.mockResolvedValue(createSnapshot())
    apiMocks.streamLogs.mockResolvedValue([])
    apiMocks.getOperationStatus.mockResolvedValue(createIdleTask())
    apiMocks.getOperationEvents.mockResolvedValue([])
    bizclawUpdaterMocks.getCurrentBizClawVersion.mockResolvedValue('0.1.8')
    bizclawUpdaterMocks.checkForBizClawUpdate.mockResolvedValue({
      version: '0.1.9',
      body: 'Bug fixes',
      date: '2026-03-14T00:00:00.000Z',
      downloadAndInstall,
    })
    apiMocks.onRuntimeLog.mockResolvedValue(() => {})
    apiMocks.onRuntimeStatus.mockResolvedValue(() => {})
    apiMocks.onOperationStatus.mockResolvedValue(() => {})
    apiMocks.onOperationEvent.mockResolvedValue(() => {})
    apiMocks.onConnectionTestEvent.mockResolvedValue(() => {})
    apiMocks.onRefreshRequested.mockResolvedValue(() => {})

    let model!: ReturnType<typeof useAppModel>
    const TestHarness = defineComponent({
      setup() {
        model = useAppModel()
        return () => h('div')
      },
    })

    host = document.createElement('div')
    document.body.appendChild(host)
    app = createApp(TestHarness)
    app.mount(host)

    await flushPromises()
    await model.checkBizClawUpdates()
    await flushPromises()
    await model.installBizClawUpdate()
    await flushPromises()
    model.deferBizClawRestart()

    expect(downloadAndInstall).toHaveBeenCalledTimes(1)
    expect(model.bizclawUpdate.value.phase).toBe('readyToRestart')
    expect(model.bizclawUpdate.value.downloadedBytes).toBe(256)
    expect(model.bizclawUpdate.value.totalBytes).toBe(1024)
    expect(model.bizclawUpdatePrimaryAction.value).toBe('立即重启')
  })

  it('prompts for elevation and retries the install after confirmation', async () => {
    apiMocks.detectEnvironment
      .mockResolvedValueOnce(createSnapshot({
        hostSshInstalled: true,
        targetSshInstalled: true,
      }))
      .mockResolvedValue(createSnapshot({
        hostSshInstalled: true,
        targetSshInstalled: true,
      }))
    apiMocks.streamLogs.mockResolvedValue([])
    apiMocks.getOperationStatus.mockResolvedValue(createIdleTask())
    apiMocks.getOperationEvents.mockResolvedValue([])
    apiMocks.installOpenClaw
      .mockResolvedValueOnce(createTask({
        phase: 'error',
        kind: 'install',
        step: 'installOpenClaw',
        success: false,
        followUp: '需要管理员权限才能继续安装。',
        remediation: {
          kind: 'requestElevation',
          urlTarget: null,
        },
      }))
      .mockResolvedValueOnce(createTask({
        phase: 'success',
        kind: 'install',
        step: 'installOpenClaw',
        success: true,
        followUp: 'OpenClaw 安装完成，请继续保存连接并启动托管；如果当前终端还未识别新命令，请重新打开终端。若 PowerShell 里仍无法直接使用 openclaw，可先改用 openclaw.cmd。',
      }))
    bizclawUpdaterMocks.getCurrentBizClawVersion.mockResolvedValue('0.1.8')
    apiMocks.onRuntimeLog.mockResolvedValue(() => {})
    apiMocks.onRuntimeStatus.mockResolvedValue(() => {})
    apiMocks.onOperationStatus.mockResolvedValue(() => {})
    apiMocks.onOperationEvent.mockResolvedValue(() => {})
    apiMocks.onConnectionTestEvent.mockResolvedValue(() => {})
    apiMocks.onRefreshRequested.mockResolvedValue(() => {})

    let model!: ReturnType<typeof useAppModel>
    const TestHarness = defineComponent({
      setup() {
        model = useAppModel()
        return () => h('div')
      },
    })

    host = document.createElement('div')
    document.body.appendChild(host)
    app = createApp(TestHarness)
    app.mount(host)

    await flushPromises()
    await model.installCli()
    await flushPromises()

    expect(apiMocks.installOpenClaw).toHaveBeenNthCalledWith(1, {
      preferOfficial: true,
      allowElevation: false,
    })
    expect(model.installRemediationModal.open).toBe(true)
    expect(model.installRemediationModal.kind).toBe('requestElevation')

    await model.confirmInstallRemediation()
    await flushPromises()

    expect(apiMocks.installOpenClaw).toHaveBeenNthCalledWith(2, {
      preferOfficial: true,
      allowElevation: true,
    })
    expect(model.installRemediationModal.open).toBe(false)
    expect(model.operationTask.value.phase).toBe('success')
  })

  it('installs to native Windows by default when the user clicks install on Windows', async () => {
    apiMocks.detectEnvironment.mockResolvedValue(createSnapshot({
      os: 'windows',
      runtimeTarget: 'windowsNative',
      openclawInstalled: false,
      openclawVersion: null,
      latestOpenclawVersion: null,
      updateAvailable: false,
      hostOpenclawInstalled: false,
      wslOpenclawInstalled: false,
      wslStatus: {
        available: false,
        distroName: 'Ubuntu',
        distroInstalled: false,
        ready: false,
        needsReboot: false,
        message: '未检测到 WSL，可通过自动安装继续。',
      },
    }))
    apiMocks.streamLogs.mockResolvedValue([])
    apiMocks.getOperationStatus.mockResolvedValue(createIdleTask())
    apiMocks.getOperationEvents.mockResolvedValue([])
    bizclawUpdaterMocks.getCurrentBizClawVersion.mockResolvedValue('0.1.8')
    apiMocks.onRuntimeLog.mockResolvedValue(() => {})
    apiMocks.onRuntimeStatus.mockResolvedValue(() => {})
    apiMocks.onEnvironmentSnapshot.mockResolvedValue(() => {})
    apiMocks.onOperationStatus.mockResolvedValue(() => {})
    apiMocks.onOperationEvent.mockResolvedValue(() => {})
    apiMocks.onConnectionTestEvent.mockResolvedValue(() => {})
    apiMocks.onRefreshRequested.mockResolvedValue(() => {})

    let model!: ReturnType<typeof useAppModel>
    const TestHarness = defineComponent({
      setup() {
        model = useAppModel()
        return () => h('div')
      },
    })

    host = document.createElement('div')
    document.body.appendChild(host)
    app = createApp(TestHarness)
    app.mount(host)

    await flushPromises()
    await model.installCli()
    await flushPromises()

    expect(apiMocks.installOpenClaw).toHaveBeenCalledWith({
      preferOfficial: true,
      allowElevation: false,
      windowsTarget: 'windowsNative',
    })
  })

  it('can install explicitly to WSL on Windows when the user chooses the WSL action', async () => {
    apiMocks.detectEnvironment.mockResolvedValue(createSnapshot({
      os: 'windows',
      runtimeTarget: 'windowsNative',
      openclawInstalled: false,
      openclawVersion: null,
      hostOpenclawInstalled: false,
      wslOpenclawInstalled: false,
      wslStatus: {
        available: true,
        distroName: 'Ubuntu',
        distroInstalled: true,
        ready: true,
        needsReboot: false,
        message: null,
      },
    }))
    apiMocks.streamLogs.mockResolvedValue([])
    apiMocks.getOperationStatus.mockResolvedValue(createIdleTask())
    apiMocks.getOperationEvents.mockResolvedValue([])
    apiMocks.installOpenClaw.mockResolvedValue(createIdleTask())
    bizclawUpdaterMocks.getCurrentBizClawVersion.mockResolvedValue('0.1.21')
    apiMocks.onRuntimeLog.mockResolvedValue(() => {})
    apiMocks.onRuntimeStatus.mockResolvedValue(() => {})
    apiMocks.onEnvironmentSnapshot.mockResolvedValue(() => {})
    apiMocks.onOperationStatus.mockResolvedValue(() => {})
    apiMocks.onOperationEvent.mockResolvedValue(() => {})
    apiMocks.onConnectionTestEvent.mockResolvedValue(() => {})
    apiMocks.onRefreshRequested.mockResolvedValue(() => {})

    let model!: ReturnType<typeof useAppModel>
    const TestHarness = defineComponent({
      setup() {
        model = useAppModel()
        return () => h('div')
      },
    })

    host = document.createElement('div')
    document.body.appendChild(host)
    app = createApp(TestHarness)
    app.mount(host)

    await flushPromises()
    await model.installWslCli()
    await flushPromises()

    expect(apiMocks.installOpenClaw).toHaveBeenCalledWith({
      preferOfficial: true,
      allowElevation: false,
      windowsTarget: 'windowsWsl',
    })
  })

  it('applies pushed environment snapshots without requiring a full refresh', async () => {
    let environmentSnapshotHandler: ((snapshot: EnvironmentSnapshot) => void) | null = null
    const initialSnapshot = createSnapshot({
      os: 'windows',
      runtimeTarget: 'windowsNative',
      openclawInstalled: false,
      openclawVersion: null,
      latestOpenclawVersion: null,
      updateAvailable: false,
    })
    const refreshedSnapshot = createSnapshot({
      os: 'windows',
      runtimeTarget: 'windowsWsl',
      openclawInstalled: true,
      openclawVersion: 'OpenClaw 2026.3.9',
      wslOpenclawInstalled: true,
      wslStatus: {
        available: true,
        distroName: 'Ubuntu',
        distroInstalled: true,
        ready: true,
        needsReboot: false,
        message: null,
      },
    })

    apiMocks.detectEnvironment.mockResolvedValue(initialSnapshot)
    apiMocks.streamLogs.mockResolvedValue([])
    apiMocks.getOperationStatus.mockResolvedValue(createIdleTask())
    apiMocks.getOperationEvents.mockResolvedValue([])
    bizclawUpdaterMocks.getCurrentBizClawVersion.mockResolvedValue('0.1.8')
    apiMocks.onRuntimeLog.mockResolvedValue(() => {})
    apiMocks.onRuntimeStatus.mockResolvedValue(() => {})
    apiMocks.onEnvironmentSnapshot.mockImplementation(async (handler: (snapshot: EnvironmentSnapshot) => void) => {
      environmentSnapshotHandler = handler
      return () => {}
    })
    apiMocks.onOperationStatus.mockResolvedValue(() => {})
    apiMocks.onOperationEvent.mockResolvedValue(() => {})
    apiMocks.onConnectionTestEvent.mockResolvedValue(() => {})
    apiMocks.onRefreshRequested.mockResolvedValue(() => {})

    let model!: ReturnType<typeof useAppModel>
    const TestHarness = defineComponent({
      setup() {
        model = useAppModel()
        return () => h('div')
      },
    })

    host = document.createElement('div')
    document.body.appendChild(host)
    app = createApp(TestHarness)
    app.mount(host)

	    await flushPromises()

	    expect(model.environment.value?.runtimeTarget).toBe('windowsNative')
	    expect(environmentSnapshotHandler).not.toBeNull()
	    if (!environmentSnapshotHandler) {
	      throw new Error('expected environment snapshot handler to be registered')
	    }
	    const applyEnvironmentSnapshot = environmentSnapshotHandler as (snapshot: EnvironmentSnapshot) => void

	    applyEnvironmentSnapshot(refreshedSnapshot)
	    await flushPromises()

    expect(model.environment.value).toEqual(refreshedSnapshot)
  })

  it('batches runtime logs and operation events before updating the UI state', async () => {
    let runtimeLogHandler: ((entry: LogEntry) => void) | null = null
    let operationEventHandler: ((entry: OperationEvent) => void) | null = null

    apiMocks.detectEnvironment.mockResolvedValue(createSnapshot())
    apiMocks.streamLogs.mockResolvedValue([])
    apiMocks.getOperationStatus.mockResolvedValue(createIdleTask())
    apiMocks.getOperationEvents.mockResolvedValue([])
    bizclawUpdaterMocks.getCurrentBizClawVersion.mockResolvedValue('0.1.8')
    apiMocks.onRuntimeLog.mockImplementation(async (handler: (entry: LogEntry) => void) => {
      runtimeLogHandler = handler
      return () => {}
    })
    apiMocks.onRuntimeStatus.mockResolvedValue(() => {})
    apiMocks.onOperationStatus.mockResolvedValue(() => {})
    apiMocks.onOperationEvent.mockImplementation(async (handler: (entry: OperationEvent) => void) => {
      operationEventHandler = handler
      return () => {}
    })
    apiMocks.onConnectionTestEvent.mockResolvedValue(() => {})
    apiMocks.onRefreshRequested.mockResolvedValue(() => {})

    let model!: ReturnType<typeof useAppModel>
    const TestHarness = defineComponent({
      setup() {
        model = useAppModel()
        return () => h('div')
      },
    })

    host = document.createElement('div')
    document.body.appendChild(host)
    app = createApp(TestHarness)
    app.mount(host)

    await flushPromises()

    const emitRuntimeLog: (entry: LogEntry) => void = runtimeLogHandler ?? (() => {
      throw new Error('expected runtime log subscription to be registered')
    })
    const emitOperationEvent: (entry: OperationEvent) => void = operationEventHandler ?? (() => {
      throw new Error('expected operation event subscription to be registered')
    })

    emitRuntimeLog({
      source: 'stdout',
      level: 'info',
      message: 'first log line',
      timestampMs: 1,
    })
    emitOperationEvent({
      kind: 'install',
      step: 'installOpenClaw',
      status: 'log',
      source: 'stdout',
      message: 'first event line',
      timestampMs: 2,
    })

    expect(model.logs.value).toEqual([])
    expect(model.operationEvents.value).toEqual([])

    await flushPromises()

    expect(model.logs.value).toHaveLength(1)
    expect(model.logs.value[0]?.message).toBe('first log line')
    expect(model.operationEvents.value).toHaveLength(1)
    expect(model.operationEvents.value[0]?.message).toBe('first event line')
  })

  it('guides Homebrew installation and retries the update after the user asks to continue', async () => {
    apiMocks.detectEnvironment
      .mockResolvedValueOnce(createSnapshot({
        hostSshInstalled: false,
        targetSshInstalled: false,
      }))
      .mockResolvedValue(createSnapshot({
        hostSshInstalled: true,
        targetSshInstalled: true,
      }))
    apiMocks.streamLogs.mockResolvedValue([])
    apiMocks.getOperationStatus.mockResolvedValue(createIdleTask())
    apiMocks.getOperationEvents.mockResolvedValue([])
    apiMocks.openSupportUrl.mockResolvedValue(undefined)
    apiMocks.updateOpenClaw
      .mockResolvedValueOnce(createTask({
        phase: 'error',
        kind: 'update',
        step: 'ensureSsh',
        success: false,
        followUp: '当前未检测到 Homebrew，请先安装 Homebrew 后重试。',
        remediation: {
          kind: 'installHomebrew',
          urlTarget: 'homebrewInstall',
        },
      }))
      .mockResolvedValueOnce(createTask({
        phase: 'success',
        kind: 'update',
        step: 'updateOpenClaw',
        success: true,
        followUp: 'OpenClaw 更新完成，请重新检测版本或直接启动连接。',
      }))
    bizclawUpdaterMocks.getCurrentBizClawVersion.mockResolvedValue('0.1.8')
    apiMocks.onRuntimeLog.mockResolvedValue(() => {})
    apiMocks.onRuntimeStatus.mockResolvedValue(() => {})
    apiMocks.onOperationStatus.mockResolvedValue(() => {})
    apiMocks.onOperationEvent.mockResolvedValue(() => {})
    apiMocks.onConnectionTestEvent.mockResolvedValue(() => {})
    apiMocks.onRefreshRequested.mockResolvedValue(() => {})

    let model!: ReturnType<typeof useAppModel>
    const TestHarness = defineComponent({
      setup() {
        model = useAppModel()
        return () => h('div')
      },
    })

    host = document.createElement('div')
    document.body.appendChild(host)
    app = createApp(TestHarness)
    app.mount(host)

    await flushPromises()
    await model.updateCli()
    await flushPromises()

    expect(apiMocks.updateOpenClaw).toHaveBeenNthCalledWith(1, {
      preferOfficial: true,
      allowElevation: false,
    })
    expect(model.installRemediationModal.open).toBe(true)
    expect(model.installRemediationModal.kind).toBe('installHomebrew')

    await model.openInstallRemediationSupportUrl()
    expect(apiMocks.openSupportUrl).toHaveBeenCalledWith('homebrewInstall')

    await model.confirmInstallRemediation()
    await flushPromises()

    expect(apiMocks.updateOpenClaw).toHaveBeenNthCalledWith(2, {
      preferOfficial: true,
      allowElevation: false,
    })
    expect(model.installRemediationModal.open).toBe(false)
    expect(model.operationTask.value.phase).toBe('success')
  })

  it('keeps the current environment until an async OpenClaw update check reports completion', async () => {
    const initialSnapshot = createSnapshot({
      openclawVersion: 'OpenClaw 2026.3.8',
      latestOpenclawVersion: null,
      updateAvailable: false,
    })
    const refreshedSnapshot = createSnapshot({
      openclawVersion: 'OpenClaw 2026.3.8',
      latestOpenclawVersion: '2026.3.10',
      updateAvailable: true,
    })
    let operationStatusHandler: ((snapshot: OperationTaskSnapshot) => void) | null = null

    apiMocks.detectEnvironment
      .mockResolvedValueOnce(initialSnapshot)
      .mockResolvedValueOnce(refreshedSnapshot)
    apiMocks.streamLogs.mockResolvedValue([])
    apiMocks.getOperationStatus.mockResolvedValue(createIdleTask())
    apiMocks.getOperationEvents.mockResolvedValue([])
    apiMocks.checkOpenClawUpdate.mockResolvedValue(createTask({
      phase: 'running',
      kind: 'checkUpdate',
      step: 'checkUpdate',
      success: true,
      followUp: '正在检查更新。',
    }))
    bizclawUpdaterMocks.getCurrentBizClawVersion.mockResolvedValue('0.1.8')
    apiMocks.onRuntimeLog.mockResolvedValue(() => {})
    apiMocks.onRuntimeStatus.mockResolvedValue(() => {})
    apiMocks.onOperationStatus.mockImplementation(async (
      handler: (snapshot: OperationTaskSnapshot) => void,
    ) => {
      operationStatusHandler = handler
      return () => {}
    })
    apiMocks.onOperationEvent.mockResolvedValue(() => {})
    apiMocks.onConnectionTestEvent.mockResolvedValue(() => {})
    apiMocks.onRefreshRequested.mockResolvedValue(() => {})

    let model!: ReturnType<typeof useAppModel>
    const TestHarness = defineComponent({
      setup() {
        model = useAppModel()
        return () => h('div')
      },
    })

    host = document.createElement('div')
    document.body.appendChild(host)
    app = createApp(TestHarness)
    app.mount(host)

    await flushPromises()
    await model.checkForUpdates()
    await flushPromises()

    expect(apiMocks.checkOpenClawUpdate).toHaveBeenCalledTimes(1)
    expect(model.operationTask.value.phase).toBe('running')
    expect(model.environment.value).toEqual(initialSnapshot)
    expect(apiMocks.detectEnvironment).toHaveBeenCalledTimes(1)

    expect(operationStatusHandler).not.toBeNull()
    operationStatusHandler!(createTask({
      phase: 'success',
      kind: 'checkUpdate',
      step: 'checkUpdate',
      success: true,
      followUp: '检测到新版本。',
    }))
    await flushPromises()

    expect(apiMocks.detectEnvironment).toHaveBeenCalledTimes(2)
    expect(model.environment.value).toEqual(refreshedSnapshot)
  })

  it('shows the Windows administrator run hint on the overview card after install', async () => {
    apiMocks.detectEnvironment.mockResolvedValue(createSnapshot({
      os: 'windows',
      runtimeTarget: 'windowsNative',
      openclawInstalled: true,
      updateAvailable: false,
      latestOpenclawVersion: null,
    }))
    apiMocks.streamLogs.mockResolvedValue([])
    apiMocks.getOperationStatus.mockResolvedValue(createIdleTask())
    apiMocks.getOperationEvents.mockResolvedValue([])
    bizclawUpdaterMocks.getCurrentBizClawVersion.mockResolvedValue('0.1.8')
    apiMocks.onRuntimeLog.mockResolvedValue(() => {})
    apiMocks.onRuntimeStatus.mockResolvedValue(() => {})
    apiMocks.onOperationStatus.mockResolvedValue(() => {})
    apiMocks.onOperationEvent.mockResolvedValue(() => {})
    apiMocks.onConnectionTestEvent.mockResolvedValue(() => {})
    apiMocks.onRefreshRequested.mockResolvedValue(() => {})

    let model!: ReturnType<typeof useAppModel>
    const TestHarness = defineComponent({
      setup() {
        model = useAppModel()
        return () => h('div')
      },
    })

    host = document.createElement('div')
    document.body.appendChild(host)
    app = createApp(TestHarness)
    app.mount(host)

    await flushPromises()

    const openclawCard = model.overviewCards.value.find((card) => card.label === 'OpenClaw')
    expect(openclawCard?.detail).toContain(translate('overview.windowsAdminRunHint'))
  })
})

async function flushPromises() {
  for (let index = 0; index < 6; index += 1) {
    await Promise.resolve()
    await nextTick()
  }
  await new Promise((resolve) => setTimeout(resolve, 80))
  await nextTick()
}

function createSnapshot(overrides: Partial<EnvironmentSnapshot> = {}): EnvironmentSnapshot {
  return {
    os: 'macos',
    runtimeTarget: 'macNative',
    hostSshInstalled: true,
    hostOpenclawInstalled: true,
    targetSshInstalled: true,
    openclawInstalled: true,
    openclawVersion: 'OpenClaw 2026.3.8',
    latestOpenclawVersion: '2026.3.9',
    updateAvailable: true,
    wslOpenclawInstalled: false,
    hasSavedProfile: true,
    tokenStatus: 'saved',
    tokenStatusMessage: null,
    uiPreferences: {
      theme: 'light',
      locale: 'zh-CN',
      sidebarCollapsed: false,
    },
    savedSettings: null,
    runtimeStatus: {
      phase: 'configured',
      sshConnected: false,
      nodeConnected: false,
      gatewayConnected: false,
      lastError: null,
    },
    installRecommendation: 'test',
    wslStatus: null,
    ...overrides,
  }
}

function createIdleTask(): OperationTaskSnapshot {
  return {
    phase: 'idle',
    kind: null,
    step: null,
    canStop: false,
    lastResult: null,
    startedAt: null,
    endedAt: null,
  }
}

function createTask({
  phase,
  kind,
  step,
  success,
  followUp,
  remediation = null,
}: {
  phase: OperationTaskSnapshot['phase']
  kind: 'install' | 'update' | 'checkUpdate'
  step: 'ensureSsh' | 'installOpenClaw' | 'updateOpenClaw' | 'checkUpdate'
  success: boolean
  followUp: string
  remediation?: {
    kind: 'requestElevation' | 'installHomebrew'
    urlTarget: 'homebrewInstall' | null
  } | null
}): OperationTaskSnapshot {
  return {
    phase,
    kind,
    step,
    canStop: false,
    lastResult: {
      kind,
      strategy: success ? 'official' : 'official-failed',
      success,
      step,
      stdout: '',
      stderr: '',
      needsElevation: remediation?.kind === 'requestElevation',
      manualUrl: 'https://docs.openclaw.ai/install',
      followUp,
      remediation,
    } as never,
    startedAt: 1,
    endedAt: 2,
  }
}

function createMatchMediaMock(initialMatches: boolean) {
  let matches = initialMatches
  type MatchMediaListener = EventListenerOrEventListenerObject | ((event: MediaQueryListEvent) => void)
  const listeners = new Set<MatchMediaListener>()

  const notifyListener = (listener: MatchMediaListener, event: MediaQueryListEvent) => {
    if (typeof listener === 'function') {
      listener(event)
      return
    }

    listener.handleEvent(event)
  }

  return {
    get matches() {
      return matches
    },
    media: '(prefers-color-scheme: dark)',
    onchange: null,
    addEventListener: vi.fn((_type: string, listener: EventListenerOrEventListenerObject) => {
      listeners.add(listener)
    }),
    removeEventListener: vi.fn((_type: string, listener: EventListenerOrEventListenerObject) => {
      listeners.delete(listener)
    }),
    addListener: vi.fn((listener: ((event: MediaQueryListEvent) => void) | null) => {
      if (listener) {
        listeners.add(listener)
      }
    }),
    removeListener: vi.fn((listener: ((event: MediaQueryListEvent) => void) | null) => {
      if (listener) {
        listeners.delete(listener)
      }
    }),
    dispatchEvent: vi.fn(() => true),
    setMatches(nextMatches: boolean) {
      matches = nextMatches
      const event = { matches, media: '(prefers-color-scheme: dark)' } as MediaQueryListEvent
      for (const listener of listeners) {
        notifyListener(listener, event)
      }
    },
  } satisfies MediaQueryList & { setMatches: (nextMatches: boolean) => void }
}
