import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

import type {
  CompanyProfile,
  EnvironmentSnapshot,
  InstallRequest,
  InstallResult,
  LogEntry,
  PersistedSettings,
  RuntimeStatus,
  UserProfile,
} from '@/types'

export const detectEnvironment = () =>
  invoke<EnvironmentSnapshot>('detect_environment')

export const installOpenClaw = (request: InstallRequest) =>
  invoke<InstallResult>('install_openclaw', { request })

export const openManualInstall = () => invoke<void>('open_manual_install')

export const saveProfile = (
  companyProfile: CompanyProfile,
  userProfile: UserProfile,
  token: string,
  sshPassword: string,
) =>
  invoke<PersistedSettings>('save_profile', {
    companyProfile,
    userProfile,
    token,
    sshPassword,
  })

export const startRuntime = () => invoke<RuntimeStatus>('start_runtime')

export const stopRuntime = () => invoke<RuntimeStatus>('stop_runtime')

export const getRuntimeStatus = () =>
  invoke<RuntimeStatus>('get_runtime_status')

export const streamLogs = () => invoke<LogEntry[]>('stream_logs')

export const onRuntimeLog = (handler: (entry: LogEntry) => void) =>
  listen<LogEntry>('runtime-log', (event) => handler(event.payload))

export const onRuntimeStatus = (handler: (status: RuntimeStatus) => void): Promise<UnlistenFn> =>
  listen<RuntimeStatus>('runtime-status', (event) => handler(event.payload))
