export type RuntimePhase =
  | 'checking'
  | 'installNeeded'
  | 'installing'
  | 'manualWait'
  | 'configured'
  | 'connecting'
  | 'running'
  | 'error'

export type RuntimeTarget = 'macNative' | 'windowsNative' | 'windowsWsl'
export type WindowsInstallTarget = Extract<RuntimeTarget, 'windowsNative' | 'windowsWsl'>

export type ThemePreference = 'light' | 'dark' | 'system'
export type LocalePreference = 'zh-CN' | 'en-US'

export interface UiPreferences {
  theme: ThemePreference
  locale: LocalePreference
  sidebarCollapsed: boolean
}

export interface CompanyProfile {
  sshHost: string
  sshUser: string
  localPort: number
  remoteBindHost: string
  remoteBindPort: number
}

export interface TargetProfile {
  wslDistro: string
}

export interface CompanyProfileDraft {
  sshHost: string
  sshUser: string
  localPort: string
  remoteBindHost: string
  remoteBindPort: string
}

export interface UserProfile {
  displayName: string
  autoConnect: boolean
  runInBackground: boolean
}

export interface PersistedSettings {
  companyProfile: CompanyProfile
  userProfile: UserProfile
  targetProfile: TargetProfile
}

export interface RuntimeStatus {
  phase: RuntimePhase
  sshConnected: boolean
  nodeConnected: boolean
  gatewayConnected: boolean
  lastError: string | null
}

export interface EnvironmentSnapshot {
  os: string
  runtimeTarget: RuntimeTarget
  hostSshInstalled: boolean
  hostOpenclawInstalled: boolean
  targetSshInstalled: boolean
  openclawInstalled: boolean
  openclawVersion: string | null
  latestOpenclawVersion: string | null
  updateAvailable: boolean
  wslOpenclawInstalled: boolean
  hasSavedProfile: boolean
  tokenStatus: 'saved' | 'missing' | 'error'
  tokenStatusMessage: string | null
  uiPreferences: UiPreferences
  savedSettings: PersistedSettings | null
  runtimeStatus: RuntimeStatus
  installRecommendation: string
  wslStatus: WslStatus | null
}

export type BizClawUpdatePhase =
  | 'idle'
  | 'checking'
  | 'upToDate'
  | 'available'
  | 'downloading'
  | 'installing'
  | 'readyToRestart'
  | 'error'

export interface BizClawUpdateState {
  phase: BizClawUpdatePhase
  currentVersion: string | null
  latestVersion: string | null
  releaseNotes: string | null
  publishedAt: string | null
  downloadedBytes: number
  totalBytes: number | null
  errorMessage: string | null
}

export interface InstallRequest {
  preferOfficial: boolean
  allowElevation: boolean
  windowsTarget?: WindowsInstallTarget | null
}

export type SupportUrlTarget = 'openclawManual' | 'homebrewInstall'

export interface OperationRemediation {
  kind: 'requestElevation' | 'installHomebrew'
  urlTarget: SupportUrlTarget | null
}

export interface OperationResult {
  kind: OperationKind
  strategy: string
  success: boolean
  step: OperationStep
  stdout: string
  stderr: string
  needsElevation: boolean
  manualUrl: string
  followUp: string
  remediation?: OperationRemediation | null
}

export type OperationTaskPhase =
  | 'idle'
  | 'running'
  | 'cancelling'
  | 'success'
  | 'error'
  | 'cancelled'

export interface OperationTaskSnapshot {
  phase: OperationTaskPhase
  kind: OperationKind | null
  step: OperationStep | null
  canStop: boolean
  lastResult: OperationResult | null
  startedAt: number | null
  endedAt: number | null
}

export interface LogEntry {
  source: string
  level: string
  message: string
  timestampMs: number
}

export interface WslStatus {
  available: boolean
  distroName: string
  distroInstalled: boolean
  ready: boolean
  needsReboot: boolean
  message: string | null
}

export type OperationKind = 'install' | 'checkUpdate' | 'update'

export type OperationStep =
  | 'detect'
  | 'bootstrapWsl'
  | 'ensureSsh'
  | 'installOpenClaw'
  | 'checkUpdate'
  | 'updateOpenClaw'

export type OperationEventStatus = 'running' | 'success' | 'error' | 'log' | 'cancelled'
export type OperationEventSource = 'system' | 'stdout' | 'stderr'

export interface OperationEvent {
  kind: OperationKind
  step: OperationStep
  status: OperationEventStatus
  source: OperationEventSource
  message: string
  timestampMs: number
}

export type ConnectionTestStep = 'save' | 'sshTunnel' | 'gatewayProbe'
export type ConnectionTestEventStatus = 'running' | 'success' | 'error'
export type ConnectionTestStepStatus = 'pending' | ConnectionTestEventStatus

export interface ConnectionTestEvent {
  step: ConnectionTestStep
  status: ConnectionTestEventStatus
  message: string
  timestampMs: number
}

export interface ConnectionTestResult {
  success: boolean
  step: ConnectionTestStep
  summary: string
  stdout: string
  stderr: string
}

export interface ConnectionTestModalStep {
  step: ConnectionTestStep
  label: string
  status: ConnectionTestStepStatus
  message: string
}

export interface ConnectionTestModalState {
  open: boolean
  phase: 'idle' | 'running' | 'success' | 'error'
  summary: string
  result: ConnectionTestResult | null
  steps: ConnectionTestModalStep[]
}

export interface InstallRemediationModalState {
  open: boolean
  kind: OperationRemediation['kind'] | null
  actionKind: Extract<OperationKind, 'install' | 'update'> | null
  title: string
  detail: string
  confirmLabel: string
  supportLabel: string
  supportUrlTarget: SupportUrlTarget | null
}
