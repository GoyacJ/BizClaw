import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

import type {
  CompanyProfile,
  ConnectionTestEvent,
  ConnectionTestResult,
  EnvironmentSnapshot,
  InstallRequest,
  LogEntry,
  OperationEvent,
  OperationTaskSnapshot,
  PersistedSettings,
  SupportUrlTarget,
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
  targetSshInstalled: false,
  openclawInstalled: false,
  openclawVersion: null,
  latestOpenclawVersion: null,
  updateAvailable: false,
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

export const detectEnvironment = () => (
  canInvokeTauri()
    ? tauriInvoke<EnvironmentSnapshot>('detect_environment')
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
