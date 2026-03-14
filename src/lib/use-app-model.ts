import { computed, nextTick, onBeforeUnmount, onMounted, reactive, ref, shallowRef } from 'vue'

import {
  checkForBizClawUpdate,
  describeBizClawUpdaterError,
  getCurrentBizClawVersion,
  relaunchBizClaw,
  type BizClawUpdateDownloadEvent,
  type PendingBizClawUpdate,
} from '@/lib/bizclaw-updater'
import {
  checkOpenClawUpdate,
  detectEnvironment,
  getOperationEvents,
  getOperationStatus,
  installOpenClaw,
  onConnectionTestEvent,
  onOperationEvent,
  onOperationStatus,
  onRefreshRequested,
  onRuntimeLog,
  onRuntimeStatus,
  openManualInstall,
  openSupportUrl,
  saveProfile,
  saveUiPreferences,
  startRuntime,
  stopOpenClawOperation,
  stopRuntime,
  streamLogs,
  testConnection,
  updateOpenClaw,
} from '@/lib/api'
import {
  createEmptyCompanyProfileDraft,
  draftFromCompanyProfile,
  isCompanyProfileDraftComplete,
  normalizeCompanyProfileDraft,
} from '@/lib/profile-form'
import {
  buildOperationsSummary,
  latestOperationDetail,
  operationTaskPhaseLabel,
  runtimeDetail,
  runtimeTargetLabel,
  startRuntimeDisabledReason,
  tokenStatusLabel,
  tokenStatusTone,
} from '@/lib/runtime-view'
import { appLocaleRef, setAppLocale, translate } from '@/lib/i18n'
import type {
  BizClawUpdateState,
  CompanyProfileDraft,
  ConnectionTestEvent,
  InstallRemediationModalState,
  ConnectionTestModalState,
  ConnectionTestModalStep,
  ConnectionTestResult,
  EnvironmentSnapshot,
  InstallRequest,
  LogEntry,
  OperationEvent,
  OperationKind,
  OperationResult,
  OperationTaskSnapshot,
  RuntimeStatus,
  SupportUrlTarget,
  TargetProfile,
  ThemePreference,
  LocalePreference,
  UiPreferences,
  UserProfile,
} from '@/types'

const defaultUserProfile = (): UserProfile => ({
  displayName: '',
  autoConnect: true,
  runInBackground: true,
})

const defaultTargetProfile = (): TargetProfile => ({
  wslDistro: 'Ubuntu',
})

const defaultUiPreferences = (): UiPreferences => ({
  theme: 'light',
  locale: 'zh-CN',
  sidebarCollapsed: false,
})

type AppliedTheme = Exclude<ThemePreference, 'system'>

const systemThemeMediaQuery = '(prefers-color-scheme: dark)'

function resolveAppliedTheme(theme: ThemePreference): AppliedTheme {
  if (theme !== 'system') {
    return theme
  }

  if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') {
    return 'light'
  }

  return window.matchMedia(systemThemeMediaQuery).matches ? 'dark' : 'light'
}

function applyThemePreference(theme: ThemePreference) {
  const appliedTheme = resolveAppliedTheme(theme)
  document.documentElement.dataset.theme = appliedTheme
  document.documentElement.style.colorScheme = appliedTheme
}

function observeSystemTheme(handler: () => void): (() => void) | null {
  if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') {
    return null
  }

  const mediaQuery = window.matchMedia(systemThemeMediaQuery)
  if (typeof mediaQuery.addEventListener === 'function') {
    mediaQuery.addEventListener('change', handler)
    return () => mediaQuery.removeEventListener('change', handler)
  }

  mediaQuery.addListener(handler)
  return () => mediaQuery.removeListener(handler)
}

function connectionTestStepLabel(step: ConnectionTestEvent['step']) {
  switch (step) {
    case 'save':
      return translate('connectionTest.save')
    case 'sshTunnel':
      return translate('connectionTest.sshTunnel')
    case 'gatewayProbe':
      return translate('connectionTest.gatewayProbe')
  }
}

function createConnectionTestSteps(): ConnectionTestModalStep[] {
  return [
    {
      step: 'save',
      label: connectionTestStepLabel('save'),
      status: 'pending',
      message: '',
    },
    {
      step: 'sshTunnel',
      label: connectionTestStepLabel('sshTunnel'),
      status: 'pending',
      message: '',
    },
    {
      step: 'gatewayProbe',
      label: connectionTestStepLabel('gatewayProbe'),
      status: 'pending',
      message: '',
    },
  ]
}

function createConnectionTestModalState(): ConnectionTestModalState {
  return {
    open: false,
    phase: 'idle',
    summary: '',
    result: null,
    steps: createConnectionTestSteps(),
  }
}

function createInstallRemediationModalState(): InstallRemediationModalState {
  return {
    open: false,
    kind: null,
    actionKind: null,
    title: '',
    detail: '',
    confirmLabel: '',
    supportLabel: '',
    supportUrlTarget: null,
  }
}

function createIdleOperationTask(): OperationTaskSnapshot {
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

function createBizClawUpdateState(): BizClawUpdateState {
  return {
    phase: 'idle',
    currentVersion: null,
    latestVersion: null,
    releaseNotes: null,
    publishedAt: null,
    downloadedBytes: 0,
    totalBytes: null,
    errorMessage: null,
  }
}

export function useAppModel() {
  const activeSection = ref<'overview' | 'install' | 'connection' | 'runtime' | 'settings'>('overview')
  const environment = ref<EnvironmentSnapshot | null>(null)
  const operationTask = ref<OperationTaskSnapshot>(createIdleOperationTask())
  const operationEvents = ref<OperationEvent[]>([])
  const logs = ref<LogEntry[]>([])
  const lastError = ref<string | null>(null)
  const lastErrorAction = ref<string | null>(null)
  const installBusyAction = ref<'check-update' | null>(null)
  const bizclawUpdate = ref<BizClawUpdateState>(createBizClawUpdateState())
  const manualInstallBusy = ref(false)
  const saveBusy = ref(false)
  const connectionTestBusy = ref(false)
  const runtimeStartBusy = ref(false)
  const runtimeStopBusy = ref(false)
  const advancedOpen = ref(false)
  const sshPasswordInput = ref('')
  const tokenInput = ref('')
  const hydratedFromSaved = ref(false)
  const uiPreferences = ref<UiPreferences>(defaultUiPreferences())
  const companyProfile = reactive<CompanyProfileDraft>(createEmptyCompanyProfileDraft())
  const userProfile = reactive(defaultUserProfile())
  const targetProfile = reactive(defaultTargetProfile())
  const connectionTestModal = reactive<ConnectionTestModalState>(createConnectionTestModalState())
  const installRemediationModal = reactive<InstallRemediationModalState>(createInstallRemediationModalState())
  const unlistenCallbacks: Array<() => void> = []
  const pendingBizClawUpdate = shallowRef<PendingBizClawUpdate | null>(null)
  let stopObservingSystemTheme: (() => void) | null = null

  const companyProfileComplete = computed(() => isCompanyProfileDraftComplete(companyProfile))
  const displayNameReady = computed(() => userProfile.displayName.trim().length > 0)
  const tokenStored = computed(() => environment.value?.tokenStatus === 'saved')
  const tokenReady = computed(() => tokenInput.value.trim().length > 0 || tokenStored.value)
  const activeOperationBusy = computed(() => (
    operationTask.value.phase === 'running' || operationTask.value.phase === 'cancelling'
  ))
  const connectionActionBusy = computed(() => (
    saveBusy.value
    || connectionTestBusy.value
    || runtimeStartBusy.value
    || runtimeStopBusy.value
  ))
  const canSaveProfile = computed(() => (
    companyProfileComplete.value
    && displayNameReady.value
    && tokenReady.value
    && !connectionActionBusy.value
  ))
  const installActionBusy = computed(() => installBusyAction.value !== null || activeOperationBusy.value)
  const bizclawUpdateBusy = computed(() => (
    bizclawUpdate.value.phase === 'checking'
    || bizclawUpdate.value.phase === 'downloading'
    || bizclawUpdate.value.phase === 'installing'
  ))
  const connectionTestDisabledReason = computed(() => {
    void appLocaleRef.value
    if (connectionActionBusy.value || installActionBusy.value) {
      return translate('runtime.startDisabled.blocking')
    }

    if (!environment.value) {
      return translate('runtime.startDisabled.checking')
    }

    if (environment.value.runtimeTarget === 'windowsWsl' && !environment.value.wslStatus?.ready) {
      return translate('runtime.startDisabled.wsl')
    }

    if (!environment.value.targetSshInstalled) {
      return translate('runtime.startDisabled.ssh')
    }

    if (!environment.value.openclawInstalled) {
      return translate('runtime.startDisabled.openclaw')
    }

    if (runtimeStatus.value.phase === 'connecting' || runtimeStatus.value.phase === 'running') {
      return translate('bizclaw.blockedRuntime')
    }

    return null
  })
  const canTestConnection = computed(() => canSaveProfile.value && !connectionTestDisabledReason.value)
  const connectDisabledReason = computed(() => (
    startRuntimeDisabledReason(
      environment.value,
      connectionActionBusy.value || installActionBusy.value,
    )
  ))
  const canStartHostedRuntime = computed(() => !connectDisabledReason.value)
  const canStopOperation = computed(() => operationTask.value.canStop)
  const tokenStateLabel = computed(() => tokenStatusLabel(environment.value))
  const tokenStateToneValue = computed(() => tokenStatusTone(environment.value))
  const openclawStateLabel = computed(() => {
    void appLocaleRef.value
    if (operationTask.value.phase === 'running' && operationTask.value.kind === 'install') {
      return translate('busy.installing')
    }
    if (operationTask.value.phase === 'running' && operationTask.value.kind === 'update') {
      return translate('busy.updating')
    }
    if (operationTask.value.phase === 'cancelling') {
      return translate('busy.stopping')
    }
    if (environment.value?.openclawInstalled) {
      return environment.value.openclawVersion ?? translate('state.openclawReady')
    }
    return translate('state.openclawInstallPending')
  })
  const openclawStateTone = computed(() => {
    if (operationTask.value.phase === 'running' || operationTask.value.phase === 'cancelling') {
      return 'active'
    }
    return environment.value?.openclawInstalled ? 'success' : 'warning'
  })
  const sshStateLabel = computed(() => {
    void appLocaleRef.value
    if (runtimeStatus.value.phase === 'running') {
      return runtimeStatus.value.sshConnected
        ? translate('common.connected')
        : translate('common.notConnected')
    }
    return environment.value?.targetSshInstalled
      ? translate('common.ready')
      : translate('state.sshPending')
  })
  const gatewayStateLabel = computed(() => {
    void appLocaleRef.value
    if (runtimeStatus.value.phase === 'connecting') {
      return translate('state.gatewayConnecting')
    }
    return runtimeStatus.value.gatewayConnected
      ? translate('common.connected')
      : translate('common.notConnected')
  })
  const gatewayStateTone = computed(() => {
    if (runtimeStatus.value.phase === 'connecting') {
      return 'active'
    }
    return runtimeStatus.value.gatewayConnected ? 'success' : 'neutral'
  })
  const statusItems = computed(() => ([
    {
      label: translate('common.token'),
      value: tokenStateLabel.value,
      tone: tokenStateToneValue.value,
    },
    {
      label: translate('common.openclaw'),
      value: openclawStateLabel.value,
      tone: openclawStateTone.value,
    },
    {
      label: translate('common.ssh'),
      value: sshStateLabel.value,
      tone: environment.value?.targetSshInstalled ? 'success' : 'warning',
    },
    {
      label: translate('common.gateway'),
      value: gatewayStateLabel.value,
      tone: gatewayStateTone.value,
    },
  ]))
  const operationResult = computed(() => operationTask.value.lastResult)
  const operationsSummary = computed(() => buildOperationsSummary(
    environment.value,
    operationTask.value,
    installBusyAction.value,
  ))
  const bizclawUpdateBlockedReason = computed(() => {
    void appLocaleRef.value
    if (activeOperationBusy.value) {
      return translate('bizclaw.blockedInstalling')
    }

    if (runtimeStatus.value.phase === 'connecting' || runtimeStatus.value.phase === 'running') {
      return translate('bizclaw.blockedRuntime')
    }

    return null
  })
  const bizclawUpdateTone = computed<'neutral' | 'active' | 'success' | 'error'>(() => {
    switch (bizclawUpdate.value.phase) {
      case 'checking':
      case 'downloading':
      case 'installing':
      case 'available':
      case 'readyToRestart':
        return 'active'
      case 'upToDate':
        return 'success'
      case 'error':
        return 'error'
      default:
        return 'neutral'
    }
  })
  const bizclawUpdateActionLabel = computed(() => {
    void appLocaleRef.value
    switch (bizclawUpdate.value.phase) {
      case 'checking':
        return translate('bizclaw.action.checking')
      case 'upToDate':
        return translate('bizclaw.action.upToDate')
      case 'available':
        return translate('bizclaw.action.available')
      case 'downloading':
        return translate('bizclaw.action.downloading')
      case 'installing':
        return translate('bizclaw.action.installing')
      case 'readyToRestart':
        return translate('bizclaw.action.readyToRestart')
      case 'error':
        return translate('bizclaw.action.error')
      default:
        return bizclawUpdate.value.currentVersion
          ? translate('bizclaw.action.loaded')
          : translate('bizclaw.action.loading')
    }
  })
  const bizclawUpdatePrimaryAction = computed(() => (
    void appLocaleRef.value,
    bizclawUpdate.value.phase === 'readyToRestart'
      ? translate('install.restartNow')
      : translate('install.installNow')
  ))
  const bizclawUpdateDetail = computed(() => {
    void appLocaleRef.value
    switch (bizclawUpdate.value.phase) {
      case 'checking':
        return translate('bizclaw.detail.checking')
      case 'upToDate':
        return translate('bizclaw.detail.upToDate', {
          version: bizclawUpdate.value.currentVersion ?? translate('common.unknown'),
        })
      case 'available':
        return translate('bizclaw.detail.available', {
          current: bizclawUpdate.value.currentVersion ?? translate('common.unknown'),
          latest: bizclawUpdate.value.latestVersion ?? translate('common.latest'),
        })
      case 'downloading':
        return translate('bizclaw.detail.downloading')
      case 'installing':
        return translate('bizclaw.detail.installing')
      case 'readyToRestart':
        return translate('bizclaw.detail.readyToRestart', {
          version: bizclawUpdate.value.latestVersion ?? translate('common.latest'),
        })
      case 'error':
        return bizclawUpdate.value.errorMessage ?? translate('bizclaw.detail.failed')
      default:
        return bizclawUpdate.value.currentVersion
          ? translate('bizclaw.detail.current', { version: bizclawUpdate.value.currentVersion })
          : translate('bizclaw.detail.loading')
    }
  })
  const operationHeadline = computed(() => latestOperationDetail(operationEvents.value))
  const runtimeStatus = computed<RuntimeStatus>(() => environment.value?.runtimeStatus ?? {
    phase: 'checking',
    sshConnected: false,
    nodeConnected: false,
    gatewayConnected: false,
    lastError: null,
  })
  const platformLabel = computed(() => (
    environment.value ? runtimeTargetLabel(environment.value.runtimeTarget) : translate('common.checkPending')
  ))
  const connectionTestCloseDisabled = computed(() => (
    connectionTestBusy.value && connectionTestModal.phase === 'running'
  ))
  const profileError = computed(() => {
    if (lastErrorAction.value === 'save') {
      return lastError.value
    }

    if (environment.value?.tokenStatus === 'error') {
      return environment.value.tokenStatusMessage
    }

    return null
  })
  const runtimeError = computed(() => {
    if (lastErrorAction.value === 'start' || lastErrorAction.value === 'stop') {
      return lastError.value
    }

    return runtimeStatus.value.lastError ?? null
  })
  const operationError = computed(() => {
    if (lastErrorAction.value === 'install'
      || lastErrorAction.value === 'check-update'
      || lastErrorAction.value === 'update'
      || lastErrorAction.value === 'stop-operation') {
      return lastError.value
    }

    if (operationTask.value.phase === 'error') {
      return operationTask.value.lastResult?.followUp ?? translate('runtime.operationsSummary.failedDetail')
    }

    return null
  })

  const overviewCards = computed(() => {
    void appLocaleRef.value
    const snapshot = environment.value
    if (!snapshot) {
      return []
    }

    return [
      {
        label: translate('overview.targetRuntime'),
        value: platformLabel.value,
        detail: snapshot.runtimeTarget === 'windowsWsl'
          ? (snapshot.wslStatus?.ready
            ? translate('overview.ubuntuReady', { name: snapshot.wslStatus.distroName })
            : snapshot.wslStatus?.message ?? translate('overview.waitWsl'))
          : translate('overview.localRuntimeDetail'),
        tone: snapshot.runtimeTarget === 'windowsWsl' && !snapshot.wslStatus?.ready
          ? 'warning'
          : 'success',
      },
      {
        label: translate('overview.openclaw'),
        value: snapshot.openclawInstalled
          ? (snapshot.openclawVersion ?? translate('overview.installed'))
          : translate('overview.notInstalled'),
        detail: snapshot.updateAvailable
          ? translate('overview.updateAvailable', {
            version: snapshot.latestOpenclawVersion ?? translate('common.latest'),
          })
          : snapshot.installRecommendation,
        tone: snapshot.openclawInstalled ? (snapshot.updateAvailable ? 'active' : 'success') : 'warning',
      },
      {
        label: translate('overview.openssh'),
        value: snapshot.targetSshInstalled ? translate('common.ready') : translate('state.sshPending'),
        detail: snapshot.runtimeTarget === 'windowsWsl'
          ? translate('overview.detectUbuntuSsh')
          : translate('overview.detectLocalSsh'),
        tone: snapshot.targetSshInstalled ? 'success' : 'warning',
      },
      {
        label: translate('overview.runtimeStatus'),
        value: runtimeDetail(runtimeStatus.value),
        detail: translate('overview.currentPhase', { phase: runtimeStatus.value.phase }),
        tone: runtimeStatus.value.phase === 'running'
          ? 'success'
          : runtimeStatus.value.phase === 'error'
            ? 'error'
            : 'neutral',
      },
    ]
  })

  function syncUiPreferences(preferences: UiPreferences) {
    stopObservingSystemTheme?.()
    stopObservingSystemTheme = null
    uiPreferences.value = { ...preferences }
    applyThemePreference(preferences.theme)
    if (preferences.theme === 'system') {
      stopObservingSystemTheme = observeSystemTheme(() => {
        if (uiPreferences.value.theme === 'system') {
          applyThemePreference('system')
        }
      })
    }
    setAppLocale(preferences.locale)
    for (const step of connectionTestModal.steps) {
      step.label = connectionTestStepLabel(step.step)
    }
  }

  async function refreshEnvironment() {
    const snapshot = await detectEnvironment()
    environment.value = snapshot
    syncUiPreferences(snapshot.uiPreferences)
    if (!hydratedFromSaved.value && snapshot.savedSettings) {
      Object.assign(
        companyProfile,
        draftFromCompanyProfile(snapshot.savedSettings.companyProfile),
      )
      Object.assign(userProfile, snapshot.savedSettings.userProfile)
      Object.assign(targetProfile, snapshot.savedSettings.targetProfile)
      hydratedFromSaved.value = true
    }
  }

  async function refreshLogs() {
    logs.value = await streamLogs()
  }

  async function refreshOperationalState(options: { includeLogs?: boolean } = {}) {
    await refreshEnvironment()
    if (options.includeLogs) {
      await refreshLogs()
    }
    operationTask.value = await getOperationStatus()
    operationEvents.value = await getOperationEvents()
  }

  async function syncBizClawCurrentVersion() {
    try {
      const version = await getCurrentBizClawVersion()
      bizclawUpdate.value = {
        ...bizclawUpdate.value,
        currentVersion: version,
        errorMessage: null,
      }
    } catch (error) {
      bizclawUpdate.value = {
        ...bizclawUpdate.value,
        phase: 'error',
        errorMessage: describeBizClawUpdaterError(error),
      }
    }
  }

  function clearActionError(action: string) {
    if (lastErrorAction.value === action) {
      lastError.value = null
      lastErrorAction.value = null
    }
  }

  async function withBooleanBusy<T>(
    busyRef: { value: boolean },
    action: string,
    handler: () => Promise<T>,
  ): Promise<T | null> {
    busyRef.value = true
    lastError.value = null
    lastErrorAction.value = null

    try {
      return await handler()
    } catch (error) {
      lastError.value = error instanceof Error ? error.message : String(error)
      lastErrorAction.value = action
      return null
    } finally {
      busyRef.value = false
    }
  }

  async function withActionError<T>(
    action: string,
    handler: () => Promise<T>,
  ): Promise<T | null> {
    lastError.value = null
    lastErrorAction.value = null

    try {
      return await handler()
    } catch (error) {
      lastError.value = error instanceof Error ? error.message : String(error)
      lastErrorAction.value = action
      return null
    }
  }

  async function withInstallBusy<T>(
    action: 'check-update',
    handler: () => Promise<T>,
  ): Promise<T | null> {
    installBusyAction.value = action
    lastError.value = null
    lastErrorAction.value = null

    try {
      return await handler()
    } catch (error) {
      lastError.value = error instanceof Error ? error.message : String(error)
      lastErrorAction.value = action
      return null
    } finally {
      installBusyAction.value = null
    }
  }

  function resetOperationState() {
    operationEvents.value = []
    if (!activeOperationBusy.value) {
      operationTask.value = createIdleOperationTask()
    }
  }

  function resetConnectionTestModal() {
    connectionTestModal.open = false
    connectionTestModal.phase = 'idle'
    connectionTestModal.summary = ''
    connectionTestModal.result = null
    connectionTestModal.steps = createConnectionTestSteps()
  }

  function resetInstallRemediationModal() {
    Object.assign(installRemediationModal, createInstallRemediationModalState())
  }

  function openInstallRemediationModal(
    actionKind: Extract<OperationKind, 'install' | 'update'>,
    result: OperationResult,
  ) {
    const remediation = result.remediation
    if (!remediation) {
      resetInstallRemediationModal()
      return
    }

    installRemediationModal.open = true
    installRemediationModal.kind = remediation.kind
    installRemediationModal.actionKind = actionKind
    installRemediationModal.supportUrlTarget = remediation.urlTarget

    if (remediation.kind === 'requestElevation') {
      installRemediationModal.title = translate('install.remediation.elevationTitle')
      installRemediationModal.detail = actionKind === 'install'
        ? translate('install.remediation.elevationInstallDetail')
        : translate('install.remediation.elevationUpdateDetail')
      installRemediationModal.confirmLabel = translate('install.remediation.elevationConfirm')
      installRemediationModal.supportLabel = ''
      return
    }

    installRemediationModal.title = translate('install.remediation.homebrewTitle')
    installRemediationModal.detail = actionKind === 'install'
      ? translate('install.remediation.homebrewInstallDetail')
      : translate('install.remediation.homebrewUpdateDetail')
    installRemediationModal.confirmLabel = translate('install.remediation.homebrewConfirm')
    installRemediationModal.supportLabel = translate('install.remediation.homebrewSupport')
  }

  function openConnectionTestModal() {
    connectionTestModal.open = true
    connectionTestModal.phase = 'running'
    connectionTestModal.summary = translate('connectionTest.runningSummary')
    connectionTestModal.result = null
    connectionTestModal.steps = createConnectionTestSteps()
    markConnectionTestStep('save', 'success', translate('connectionTest.saved'))
  }

  function closeConnectionTestModal() {
    if (connectionTestCloseDisabled.value) {
      return
    }

    resetConnectionTestModal()
  }

  function markConnectionTestStep(
    step: ConnectionTestEvent['step'],
    status: ConnectionTestModalStep['status'],
    message: string,
  ) {
    const target = connectionTestModal.steps.find((entry) => entry.step === step)
    if (!target) {
      return
    }
    target.status = status
    target.message = message
  }

  function applyConnectionTestEvent(entry: ConnectionTestEvent) {
    if (!connectionTestModal.open) {
      return
    }

    markConnectionTestStep(entry.step, entry.status, entry.message)
  }

  function applyConnectionTestResult(result: ConnectionTestResult) {
    connectionTestModal.result = result
    connectionTestModal.phase = result.success ? 'success' : 'error'
    connectionTestModal.summary = result.summary

    if (result.success) {
      markConnectionTestStep('gatewayProbe', 'success', result.summary)
      return
    }

    const failedStep = connectionTestModal.steps.find((step) => step.step === result.step)
    if (failedStep) {
      failedStep.status = 'error'
      failedStep.message = result.summary
    }
  }

  function applyOperationTaskSnapshot(snapshot: OperationTaskSnapshot) {
    const previousPhase = operationTask.value.phase
    operationTask.value = snapshot

    const finished = (
      snapshot.phase === 'success'
      || snapshot.phase === 'error'
      || snapshot.phase === 'cancelled'
    )
    const wasFinished = (
      previousPhase === 'success'
      || previousPhase === 'error'
      || previousPhase === 'cancelled'
    )

    if (finished && !wasFinished) {
      void refreshEnvironment()
    }
  }

  function applyBizClawDownloadEvent(entry: BizClawUpdateDownloadEvent) {
    if (entry.event === 'Started') {
      bizclawUpdate.value = {
        ...bizclawUpdate.value,
        phase: 'downloading',
        downloadedBytes: 0,
        totalBytes: entry.data.contentLength ?? null,
      }
      return
    }

    if (entry.event === 'Progress') {
      bizclawUpdate.value = {
        ...bizclawUpdate.value,
        phase: 'downloading',
        downloadedBytes: bizclawUpdate.value.downloadedBytes + (entry.data.chunkLength ?? 0),
      }
      return
    }

    bizclawUpdate.value = {
      ...bizclawUpdate.value,
      phase: 'installing',
    }
  }

  async function runOpenClawOperation(
    kind: Extract<OperationKind, 'install' | 'update'>,
    request: InstallRequest,
  ) {
    resetOperationState()
    const action = kind === 'install' ? 'install' : 'update'
    const runner = kind === 'install' ? installOpenClaw : updateOpenClaw
    const result = await withActionError(action, () => runner(request))

    if (!result) {
      return null
    }

    applyOperationTaskSnapshot(result)
    const lastResult = result.lastResult
    const remediation = lastResult?.remediation
    if (result.phase === 'error' && lastResult && remediation) {
      openInstallRemediationModal(kind, lastResult)
    } else {
      resetInstallRemediationModal()
    }
    return result
  }

  async function installCli() {
    activeSection.value = 'install'
    await runOpenClawOperation('install', {
      preferOfficial: true,
      allowElevation: false,
    })
  }

  async function checkForUpdates() {
    activeSection.value = 'install'
    const result = await withInstallBusy('check-update', () => checkOpenClawUpdate())
    if (!result) {
      return
    }

    environment.value = result
  }

  async function updateCli() {
    activeSection.value = 'install'
    await runOpenClawOperation('update', {
      preferOfficial: true,
      allowElevation: false,
    })
  }

  async function stopOperation() {
    const result = await withActionError('stop-operation', async () => (
      stopOpenClawOperation()
    ))
    if (!result) {
      return
    }

    applyOperationTaskSnapshot(result)
  }

  async function checkBizClawUpdates() {
    bizclawUpdate.value = {
      ...bizclawUpdate.value,
      phase: 'checking',
      errorMessage: null,
      downloadedBytes: 0,
      totalBytes: null,
    }
    pendingBizClawUpdate.value = null

    try {
      const update = await checkForBizClawUpdate()
      if (!update) {
        bizclawUpdate.value = {
          ...bizclawUpdate.value,
          phase: 'upToDate',
          latestVersion: bizclawUpdate.value.currentVersion,
          releaseNotes: null,
          publishedAt: null,
        }
        return
      }

      pendingBizClawUpdate.value = update
      bizclawUpdate.value = {
        ...bizclawUpdate.value,
        phase: 'available',
        latestVersion: update.version,
        releaseNotes: update.body,
        publishedAt: update.date,
      }
    } catch (error) {
      bizclawUpdate.value = {
        ...bizclawUpdate.value,
        phase: 'error',
        errorMessage: describeBizClawUpdaterError(error),
      }
    }
  }

  async function installBizClawUpdate() {
    if (!pendingBizClawUpdate.value || bizclawUpdateBlockedReason.value) {
      return
    }

    bizclawUpdate.value = {
      ...bizclawUpdate.value,
      phase: 'downloading',
      errorMessage: null,
      downloadedBytes: 0,
      totalBytes: null,
    }

    try {
      await pendingBizClawUpdate.value.downloadAndInstall((entry) => {
        applyBizClawDownloadEvent(entry)
      })
      bizclawUpdate.value = {
        ...bizclawUpdate.value,
        phase: 'readyToRestart',
      }
    } catch (error) {
      bizclawUpdate.value = {
        ...bizclawUpdate.value,
        phase: 'error',
        errorMessage: describeBizClawUpdaterError(error),
      }
    }
  }

  function deferBizClawRestart() {
    if (bizclawUpdate.value.phase !== 'readyToRestart') {
      return
    }
  }

  async function restartBizClaw() {
    await relaunchBizClaw()
  }

  async function launchManualInstall() {
    manualInstallBusy.value = true
    try {
      await openManualInstall()
    } finally {
      manualInstallBusy.value = false
    }
  }

  function closeInstallRemediationModal() {
    resetInstallRemediationModal()
  }

  async function openInstallRemediationSupportUrl() {
    if (!installRemediationModal.supportUrlTarget) {
      return
    }

    manualInstallBusy.value = true
    try {
      await openSupportUrl(installRemediationModal.supportUrlTarget)
    } finally {
      manualInstallBusy.value = false
    }
  }

  async function confirmInstallRemediation() {
    const { actionKind, kind } = installRemediationModal
    if (!actionKind || !kind) {
      return
    }

    const request: InstallRequest = {
      preferOfficial: true,
      allowElevation: kind === 'requestElevation',
    }
    resetInstallRemediationModal()
    await runOpenClawOperation(actionKind, request)
  }

  async function persistProfile() {
    const normalizedCompanyProfile = normalizeCompanyProfileDraft(companyProfile)
    const result = await saveProfile(
      normalizedCompanyProfile,
      { ...userProfile },
      { ...targetProfile },
      tokenInput.value,
      sshPasswordInput.value,
    )

    tokenInput.value = ''
    sshPasswordInput.value = ''
    return result
  }

  async function saveOnly() {
    const result = await withBooleanBusy(saveBusy, 'save', async () => persistProfile())
    if (!result) {
      return
    }

    await refreshEnvironment()
  }

  async function saveAndTest() {
    activeSection.value = 'connection'
    const saved = await withBooleanBusy(saveBusy, 'save', async () => persistProfile())
    if (!saved) {
      return
    }

    clearActionError('save')
    openConnectionTestModal()
    await nextTick()
    void refreshEnvironment()

    connectionTestBusy.value = true
    lastError.value = null
    lastErrorAction.value = null

    try {
      const result = await testConnection()
      applyConnectionTestResult(result)
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      lastError.value = message
      lastErrorAction.value = 'test'
      connectionTestModal.phase = 'error'
      connectionTestModal.summary = message
      if (connectionTestModal.steps.every((step) => step.status !== 'error')) {
        markConnectionTestStep('gatewayProbe', 'error', message)
      }
    } finally {
      connectionTestBusy.value = false
    }

    await refreshEnvironment()
  }

  async function startHostedRuntime() {
    activeSection.value = 'connection'
    runtimeStartBusy.value = true
    lastError.value = null
    lastErrorAction.value = null
    logs.value = []

    const previousEnvironment = environment.value ?? await detectEnvironment()
    environment.value = {
      ...previousEnvironment,
      runtimeStatus: {
        phase: 'connecting',
        sshConnected: false,
        nodeConnected: false,
        gatewayConnected: false,
        lastError: null,
      },
    }

    try {
      const status = await startRuntime()
      if (environment.value) {
        environment.value = {
          ...environment.value,
          runtimeStatus: status,
        }
      }
    } catch (error) {
      lastError.value = error instanceof Error ? error.message : String(error)
      lastErrorAction.value = 'start'
      environment.value = previousEnvironment
    } finally {
      runtimeStartBusy.value = false
    }
  }

  async function stopHostedRuntime() {
    const result = await withBooleanBusy(runtimeStopBusy, 'stop', async () => {
      environment.value = environment.value && {
        ...environment.value,
        runtimeStatus: await stopRuntime(),
      }
      logs.value = await streamLogs()
      return environment.value?.runtimeStatus ?? null
    })

    if (!result) {
      return
    }

    await refreshEnvironment()
  }

  onMounted(async () => {
    await refreshOperationalState({ includeLogs: true })
    await syncBizClawCurrentVersion()
    unlistenCallbacks.push(
      await onRuntimeLog((entry) => {
        logs.value = [...logs.value, entry].slice(-400)
      }),
    )
    unlistenCallbacks.push(
      await onRuntimeStatus((status) => {
        if (!environment.value) {
          return
        }

        environment.value = {
          ...environment.value,
          runtimeStatus: status,
        }
      }),
    )
    unlistenCallbacks.push(
      await onOperationStatus((snapshot) => {
        applyOperationTaskSnapshot(snapshot)
      }),
    )
    unlistenCallbacks.push(
      await onOperationEvent((entry) => {
        operationEvents.value = [...operationEvents.value, entry].slice(-200)
      }),
    )
    unlistenCallbacks.push(
      await onConnectionTestEvent((entry) => {
        applyConnectionTestEvent(entry)
      }),
    )
    unlistenCallbacks.push(
      await onRefreshRequested(() => {
        void refreshEnvironment()
      }),
    )
  })

  onBeforeUnmount(() => {
    stopObservingSystemTheme?.()
    stopObservingSystemTheme = null
    while (unlistenCallbacks.length > 0) {
      unlistenCallbacks.pop()?.()
    }
  })

  async function updateUiPreferences(patch: Partial<UiPreferences>) {
    const previous = { ...uiPreferences.value }
    const next = { ...uiPreferences.value, ...patch }
    syncUiPreferences(next)
    const saved = await withActionError('ui-preferences', () => saveUiPreferences(next))
    if (!saved) {
      syncUiPreferences(previous)
      return
    }

    syncUiPreferences(saved)
    if (patch.locale && patch.locale !== previous.locale) {
      await refreshEnvironment()
    }
  }

  async function setTheme(theme: ThemePreference) {
    if (uiPreferences.value.theme === theme) {
      return
    }

    await updateUiPreferences({ theme })
  }

  async function setLocale(locale: LocalePreference) {
    if (uiPreferences.value.locale === locale) {
      return
    }

    await updateUiPreferences({ locale })
  }

  async function setSidebarCollapsed(sidebarCollapsed: boolean) {
    if (uiPreferences.value.sidebarCollapsed === sidebarCollapsed) {
      return
    }

    await updateUiPreferences({ sidebarCollapsed })
  }

  return {
    activeSection,
    advancedOpen,
    bizclawUpdate,
    bizclawUpdateActionLabel,
    bizclawUpdateBlockedReason,
    bizclawUpdateDetail,
    bizclawUpdatePrimaryAction,
    bizclawUpdateTone,
    checkBizClawUpdates,
    canSaveProfile,
    canStartHostedRuntime,
    canTestConnection,
    checkForUpdates,
    closeConnectionTestModal,
    closeInstallRemediationModal,
    companyProfile,
    confirmInstallRemediation,
    connectDisabledReason,
    canStopOperation,
    connectionTestBusy,
    connectionTestCloseDisabled,
    connectionTestDisabledReason,
    connectionTestModal,
    deferBizClawRestart,
    environment,
    installBusyAction,
    installCli,
    installBizClawUpdate,
    launchManualInstall,
    logs,
    installRemediationModal,
    manualInstallBusy,
    openInstallRemediationSupportUrl,
    operationError,
    operationEvents,
    operationHeadline,
    operationResult,
    operationTask,
    operationsSummary,
    openclawStateLabel,
    openclawStateTone,
    overviewCards,
    platformLabel,
    profileError,
    refreshEnvironment,
    runtimeError,
    runtimeStartBusy,
    runtimeStatus,
    runtimeStopBusy,
    restartBizClaw,
    saveAndTest,
    saveBusy,
    saveOnly,
    setSidebarCollapsed,
    sshPasswordInput,
    startHostedRuntime,
    statusItems,
    stopOperation,
    stopHostedRuntime,
    setLocale,
    setTheme,
    targetProfile,
    tokenInput,
    tokenStateLabel,
    tokenStateToneValue,
    gatewayStateLabel,
    gatewayStateTone,
    sshStateLabel,
    uiPreferences,
    operationTaskPhaseLabel,
    updateCli,
    userProfile,
  }
}
