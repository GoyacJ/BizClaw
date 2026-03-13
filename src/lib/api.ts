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
  TargetProfile,
  RuntimeStatus,
  UserProfile,
} from '@/types'

export const detectEnvironment = () =>
  invoke<EnvironmentSnapshot>('detect_environment')

export const installOpenClaw = (request: InstallRequest) =>
  invoke<OperationTaskSnapshot>('install_openclaw', { request })

export const checkOpenClawUpdate = () =>
  invoke<EnvironmentSnapshot>('check_openclaw_update')

export const updateOpenClaw = (request: InstallRequest) =>
  invoke<OperationTaskSnapshot>('update_openclaw', { request })

export const getOperationStatus = () =>
  invoke<OperationTaskSnapshot>('get_operation_status')

export const getOperationEvents = () =>
  invoke<OperationEvent[]>('get_operation_events')

export const stopOpenClawOperation = () =>
  invoke<OperationTaskSnapshot>('stop_openclaw_operation')

export const openManualInstall = () => invoke<void>('open_manual_install')

export const saveProfile = (
  companyProfile: CompanyProfile,
  userProfile: UserProfile,
  targetProfile: TargetProfile,
  token: string,
  sshPassword: string,
) =>
  invoke<PersistedSettings>('save_profile', {
    companyProfile,
    userProfile,
    targetProfile,
    token,
    sshPassword,
  })

export const testConnection = () => invoke<ConnectionTestResult>('test_connection')

export const startRuntime = () => invoke<RuntimeStatus>('start_runtime')

export const stopRuntime = () => invoke<RuntimeStatus>('stop_runtime')

export const getRuntimeStatus = () =>
  invoke<RuntimeStatus>('get_runtime_status')

export const streamLogs = () => invoke<LogEntry[]>('stream_logs')

export const onRuntimeLog = (handler: (entry: LogEntry) => void) =>
  listen<LogEntry>('runtime-log', (event) => handler(event.payload))

export const onRuntimeStatus = (handler: (status: RuntimeStatus) => void): Promise<UnlistenFn> =>
  listen<RuntimeStatus>('runtime-status', (event) => handler(event.payload))

export const onOperationEvent = (
  handler: (entry: OperationEvent) => void,
): Promise<UnlistenFn> =>
  listen<OperationEvent>('operation-event', (event) => handler(event.payload))

export const onOperationStatus = (
  handler: (entry: OperationTaskSnapshot) => void,
): Promise<UnlistenFn> =>
  listen<OperationTaskSnapshot>('operation-status', (event) => handler(event.payload))

export const onConnectionTestEvent = (
  handler: (entry: ConnectionTestEvent) => void,
): Promise<UnlistenFn> =>
  listen<ConnectionTestEvent>('connection-test-event', (event) => handler(event.payload))

export const onRefreshRequested = (
  handler: () => void,
): Promise<UnlistenFn> =>
  listen('app-refresh-requested', () => handler())
