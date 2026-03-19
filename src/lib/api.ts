import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

import type {
  ChatMessage,
  ChatSessionSummary,
  ClawHubSkillSearchResult,
  CompanyProfile,
  ConnectionTestEvent,
  ConnectionTestResult,
  CreateChatSessionRequest,
  CreateLocalSkillRequest,
  CreateOpenClawAgentRequest,
  EnvironmentSnapshot,
  InstallClawHubSkillRequest,
  InstallRequest,
  LogEntry,
  OpenClawAgentBinding,
  OpenClawAgentSummary,
  OpenClawSkillCheckReport,
  OpenClawSkillInfo,
  OpenClawSkillInventory,
  OperationEvent,
  OperationTaskSnapshot,
  PersistedSettings,
  SearchClawHubSkillsRequest,
  SendChatMessageRequest,
  SendChatMessageResult,
  SupportUrlTarget,
  UpdateOpenClawAgentIdentityRequest,
  UiPreferences,
  TargetProfile,
  RuntimeStatus,
  UserProfile,
} from '@/types'

const browserUiPreferences: UiPreferences = {
  theme: 'light',
  locale: 'zh-CN',
  sidebarCollapsed: false,
}

const browserUiPreferencesStorageKey = 'bizclaw-ui-preferences'

const browserRuntimeStatus: RuntimeStatus = {
  phase: 'checking',
  sshConnected: false,
  nodeConnected: false,
  gatewayConnected: false,
  lastError: null,
}

const browserEnvironmentSnapshot: EnvironmentSnapshot = {
  os: 'browser',
  runtimeTarget: 'macNative',
  hostSshInstalled: false,
  hostOpenclawInstalled: false,
  targetSshInstalled: false,
  openclawInstalled: false,
  openclawVersion: null,
  latestOpenclawVersion: null,
  updateAvailable: false,
  wslOpenclawInstalled: false,
  hasSavedProfile: false,
  tokenStatus: 'missing',
  tokenStatusMessage: null,
  uiPreferences: browserUiPreferences,
  savedSettings: null,
  runtimeStatus: browserRuntimeStatus,
  installRecommendation: '',
  wslStatus: null,
}

const idleOperationTask: OperationTaskSnapshot = {
  phase: 'idle',
  kind: null,
  step: null,
  canStop: false,
  lastResult: null,
  startedAt: null,
  endedAt: null,
}

const browserChatSessions: ChatSessionSummary[] = [
  {
    id: 'local-session',
    title: '默认会话',
    updatedAt: Date.now(),
    preview: '你好，我是 BizClaw 助手。',
  },
]

const browserChatMessagesBySessionId: Record<string, ChatMessage[]> = {
  'local-session': [
    {
      id: 'local-msg-assistant',
      role: 'assistant',
      content: '你好，我是 BizClaw 助手。',
      createdAt: Date.now(),
      status: 'done',
    },
  ],
}

const noopUnlisten: UnlistenFn = () => {}

function readBrowserUiPreferences(): UiPreferences {
  const stored = globalThis.localStorage?.getItem(browserUiPreferencesStorageKey)
  if (!stored) {
    return { ...browserUiPreferences }
  }

  try {
    const parsed = JSON.parse(stored) as Partial<UiPreferences>
    return {
      theme: parsed.theme === 'dark' || parsed.theme === 'system' ? parsed.theme : 'light',
      locale: parsed.locale === 'en-US' ? 'en-US' : 'zh-CN',
      sidebarCollapsed: parsed.sidebarCollapsed === true,
    }
  } catch {
    return { ...browserUiPreferences }
  }
}

function writeBrowserUiPreferences(preferences: UiPreferences) {
  globalThis.localStorage?.setItem(browserUiPreferencesStorageKey, JSON.stringify(preferences))
}

function canInvokeTauri() {
  return typeof invoke === 'function'
    && typeof (globalThis as { __TAURI_INTERNALS__?: { invoke?: unknown } }).__TAURI_INTERNALS__?.invoke === 'function'
}

function canListenTauri() {
  return typeof listen === 'function'
    && typeof (globalThis as { __TAURI_INTERNALS__?: { invoke?: unknown } }).__TAURI_INTERNALS__?.invoke === 'function'
}

function browserOnlyError() {
  return new Error('BizClaw desktop APIs are unavailable in the browser preview.')
}

function tauriInvoke<T>(command: string, args?: Record<string, unknown>) {
  if (!canInvokeTauri()) {
    return Promise.reject(browserOnlyError())
  }
  return invoke<T>(command, args)
}

function tauriListen<T>(
  eventName: string,
  handler: (payload: T) => void,
): Promise<UnlistenFn> {
  if (!canListenTauri()) {
    return Promise.resolve(noopUnlisten)
  }
  return listen<T>(eventName, (event) => handler(event.payload))
}

export const detectEnvironment = (force = false) => (
  canInvokeTauri()
    ? tauriInvoke<EnvironmentSnapshot>('detect_environment', force ? { force } : undefined)
    : Promise.resolve({
      ...browserEnvironmentSnapshot,
      uiPreferences: readBrowserUiPreferences(),
      runtimeStatus: { ...browserRuntimeStatus },
    })
)

export const installOpenClaw = (request: InstallRequest) =>
  tauriInvoke<OperationTaskSnapshot>('install_openclaw', { request })

export const checkOpenClawUpdate = () => (
  canInvokeTauri()
    ? tauriInvoke<OperationTaskSnapshot>('check_openclaw_update')
    : Promise.resolve<OperationTaskSnapshot>({
      ...idleOperationTask,
      phase: 'success',
      kind: 'checkUpdate',
      step: 'checkUpdate',
      startedAt: Date.now(),
      endedAt: Date.now(),
    })
)

export const updateOpenClaw = (request: InstallRequest) =>
  tauriInvoke<OperationTaskSnapshot>('update_openclaw', { request })

export const getOperationStatus = () => (
  canInvokeTauri()
    ? tauriInvoke<OperationTaskSnapshot>('get_operation_status')
    : Promise.resolve({ ...idleOperationTask })
)

export const getOperationEvents = () => (
  canInvokeTauri()
    ? tauriInvoke<OperationEvent[]>('get_operation_events')
    : Promise.resolve([])
)

export const stopOpenClawOperation = () =>
  tauriInvoke<OperationTaskSnapshot>('stop_openclaw_operation')

export const openSupportUrl = (target: SupportUrlTarget) =>
  tauriInvoke<void>('open_support_url', { target })

export const openManualInstall = () => openSupportUrl('openclawManual')

export const saveProfile = (
  companyProfile: CompanyProfile,
  userProfile: UserProfile,
  targetProfile: TargetProfile,
  token: string,
  sshPassword: string,
) =>
  tauriInvoke<PersistedSettings>('save_profile', {
    companyProfile,
    userProfile,
    targetProfile,
    token,
    sshPassword,
  })

export const listChatSessions = () => (
  canInvokeTauri()
    ? tauriInvoke<ChatSessionSummary[]>('list_chat_sessions')
    : Promise.resolve([...browserChatSessions])
)

export const createChatSession = (request: CreateChatSessionRequest = {}) => (
  canInvokeTauri()
    ? tauriInvoke<ChatSessionSummary>('create_chat_session', { request })
    : (() => {
        const session: ChatSessionSummary = {
          id: `local-session-${Date.now()}`,
          title: request.title?.trim() || '新会话',
          updatedAt: Date.now(),
          preview: '',
        }
        browserChatSessions.unshift(session)
        browserChatMessagesBySessionId[session.id] = []
        return Promise.resolve(session)
      })()
)

export const listChatMessages = (sessionId: string) => (
  canInvokeTauri()
    ? tauriInvoke<ChatMessage[]>('list_chat_messages', { sessionId })
    : Promise.resolve([...(browserChatMessagesBySessionId[sessionId] ?? [])])
)

export const sendChatMessage = (request: SendChatMessageRequest) => (
  canInvokeTauri()
    ? tauriInvoke<SendChatMessageResult>('send_chat_message', { request })
    : (() => {
        const now = Date.now()
        const userMessage: ChatMessage = {
          id: `local-user-${now}`,
          role: 'user',
          content: request.content,
          createdAt: now,
          status: 'done',
        }
        const assistantMessage: ChatMessage = {
          id: `local-assistant-${now + 1}`,
          role: 'assistant',
          content: `已收到：${request.content}`,
          createdAt: now + 1,
          status: 'done',
        }
        if (!browserChatMessagesBySessionId[request.sessionId]) {
          browserChatMessagesBySessionId[request.sessionId] = []
        }
        browserChatMessagesBySessionId[request.sessionId].push(userMessage, assistantMessage)
        const session = browserChatSessions.find((item) => item.id === request.sessionId)
        if (session) {
          session.updatedAt = now + 1
          session.preview = assistantMessage.content
        }
        return Promise.resolve({ userMessage, assistantMessage })
      })()
)

export const listOpenClawAgents = () =>
  tauriInvoke<OpenClawAgentSummary[]>('list_openclaw_agents')

export const createOpenClawAgent = (request: CreateOpenClawAgentRequest) =>
  tauriInvoke<Record<string, unknown>>('create_openclaw_agent', { request })

export const updateOpenClawAgentIdentity = (request: UpdateOpenClawAgentIdentityRequest) =>
  tauriInvoke<Record<string, unknown>>('update_openclaw_agent_identity', { request })

export const deleteOpenClawAgent = (agentId: string) =>
  tauriInvoke<Record<string, unknown>>('delete_openclaw_agent', { agentId })

export const listOpenClawAgentBindings = (agentId?: string) =>
  tauriInvoke<OpenClawAgentBinding[]>(
    'list_openclaw_agent_bindings',
    agentId ? { agentId } : undefined,
  )

export const addOpenClawAgentBindings = (agentId: string, bindings: string[]) =>
  tauriInvoke<Record<string, unknown>>('add_openclaw_agent_bindings', { agentId, bindings })

export const removeOpenClawAgentBindings = (agentId: string, bindings?: string[]) =>
  tauriInvoke<Record<string, unknown>>(
    'remove_openclaw_agent_bindings',
    bindings ? { agentId, bindings } : { agentId, removeAll: true },
  )

export const listOpenClawSkills = () =>
  tauriInvoke<OpenClawSkillInventory>('list_openclaw_skills')

export const checkOpenClawSkills = () =>
  tauriInvoke<OpenClawSkillCheckReport>('check_openclaw_skills')

export const getOpenClawSkillInfo = (name: string) =>
  tauriInvoke<OpenClawSkillInfo>('get_openclaw_skill_info', { name })

export const searchClawHubSkills = (request: SearchClawHubSkillsRequest) =>
  tauriInvoke<ClawHubSkillSearchResult[]>('search_clawhub_skills', { request })

export const installClawHubSkill = (request: InstallClawHubSkillRequest) =>
  tauriInvoke<Record<string, unknown>>('install_clawhub_skill', { request })

export const createLocalOpenClawSkill = (request: CreateLocalSkillRequest) =>
  tauriInvoke<Record<string, unknown>>('create_local_openclaw_skill', { request })

export const deleteLocalOpenClawSkill = (name: string) =>
  tauriInvoke<Record<string, unknown>>('delete_local_openclaw_skill', { name })

export const saveUiPreferences = (preferences: UiPreferences) => (
  canInvokeTauri()
    ? tauriInvoke<UiPreferences>('save_ui_preferences', { preferences })
    : (
        writeBrowserUiPreferences(preferences),
        Promise.resolve(preferences)
      )
)

export const testConnection = () => tauriInvoke<ConnectionTestResult>('test_connection')

export const startRuntime = () => tauriInvoke<RuntimeStatus>('start_runtime')

export const stopRuntime = () => tauriInvoke<RuntimeStatus>('stop_runtime')

export const getRuntimeStatus = () => (
  canInvokeTauri()
    ? tauriInvoke<RuntimeStatus>('get_runtime_status')
    : Promise.resolve({ ...browserRuntimeStatus })
)

export const streamLogs = () => (
  canInvokeTauri()
    ? tauriInvoke<LogEntry[]>('stream_logs')
    : Promise.resolve([])
)

export const onRuntimeLog = (handler: (entry: LogEntry) => void) =>
  tauriListen<LogEntry>('runtime-log', handler)

export const onRuntimeStatus = (handler: (status: RuntimeStatus) => void): Promise<UnlistenFn> =>
  tauriListen<RuntimeStatus>('runtime-status', handler)

export const onEnvironmentSnapshot = (
  handler: (snapshot: EnvironmentSnapshot) => void,
): Promise<UnlistenFn> =>
  tauriListen<EnvironmentSnapshot>('environment-snapshot', handler)

export const onOperationEvent = (
  handler: (entry: OperationEvent) => void,
): Promise<UnlistenFn> =>
  tauriListen<OperationEvent>('operation-event', handler)

export const onOperationStatus = (
  handler: (entry: OperationTaskSnapshot) => void,
): Promise<UnlistenFn> =>
  tauriListen<OperationTaskSnapshot>('operation-status', handler)

export const onConnectionTestEvent = (
  handler: (entry: ConnectionTestEvent) => void,
): Promise<UnlistenFn> =>
  tauriListen<ConnectionTestEvent>('connection-test-event', handler)

export const onRefreshRequested = (
  handler: () => void,
): Promise<UnlistenFn> =>
  tauriListen('app-refresh-requested', () => handler())
