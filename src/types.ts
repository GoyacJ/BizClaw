export type RuntimePhase =
  | 'checking'
  | 'installNeeded'
  | 'installing'
  | 'manualWait'
  | 'configured'
  | 'connecting'
  | 'running'
  | 'error'

export interface CompanyProfile {
  sshHost: string
  sshUser: string
  localPort: number
  remoteBindHost: string
  remoteBindPort: number
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
}

export interface RuntimeStatus {
  phase: RuntimePhase
  sshConnected: boolean
  nodeConnected: boolean
  lastError: string | null
}

export interface EnvironmentSnapshot {
  os: string
  sshInstalled: boolean
  openclawInstalled: boolean
  npmInstalled: boolean
  pnpmInstalled: boolean
  hasSavedProfile: boolean
  hasSavedToken: boolean
  savedSettings: PersistedSettings | null
  runtimeStatus: RuntimeStatus
  installRecommendation: string
}

export interface InstallRequest {
  preferOfficial: boolean
  allowElevation: boolean
}

export interface InstallResult {
  strategy: string
  success: boolean
  stdout: string
  stderr: string
  needsElevation: boolean
  manualUrl: string
  followUp: string
}

export interface LogEntry {
  source: string
  level: string
  message: string
  timestampMs: number
}
