import { computed, nextTick, onBeforeUnmount, onMounted, reactive, ref } from 'vue'

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
  saveProfile,
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
import type {
  CompanyProfileDraft,
  ConnectionTestEvent,
  ConnectionTestModalState,
  ConnectionTestModalStep,
  ConnectionTestResult,
  EnvironmentSnapshot,
  LogEntry,
  OperationEvent,
  OperationTaskSnapshot,
  RuntimeStatus,
  TargetProfile,
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

function createConnectionTestSteps(): ConnectionTestModalStep[] {
  return [
    {
      step: 'save',
      label: '配置已保存',
      status: 'pending',
      message: '',
    },
    {
      step: 'sshTunnel',
      label: 'SSH 隧道',
      status: 'pending',
      message: '',
    },
    {
      step: 'gatewayProbe',
      label: 'Gateway 鉴权',
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

export function useAppModel() {
  const activeSection = ref<'overview' | 'install' | 'connection' | 'runtime'>('overview')
  const environment = ref<EnvironmentSnapshot | null>(null)
  const operationTask = ref<OperationTaskSnapshot>(createIdleOperationTask())
  const operationEvents = ref<OperationEvent[]>([])
  const logs = ref<LogEntry[]>([])
  const lastError = ref<string | null>(null)
  const lastErrorAction = ref<string | null>(null)
  const installBusyAction = ref<'check-update' | null>(null)
  const manualInstallBusy = ref(false)
  const saveBusy = ref(false)
  const connectionTestBusy = ref(false)
  const runtimeStartBusy = ref(false)
  const runtimeStopBusy = ref(false)
  const advancedOpen = ref(false)
  const sshPasswordInput = ref('')
  const tokenInput = ref('')
  const hydratedFromSaved = ref(false)
  const companyProfile = reactive<CompanyProfileDraft>(createEmptyCompanyProfileDraft())
  const userProfile = reactive(defaultUserProfile())
  const targetProfile = reactive(defaultTargetProfile())
  const connectionTestModal = reactive<ConnectionTestModalState>(createConnectionTestModalState())
  const unlistenCallbacks: Array<() => void> = []

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
  const connectionTestDisabledReason = computed(() => {
    if (connectionActionBusy.value || installActionBusy.value) {
      return '当前操作进行中，请稍候。'
    }

    if (!environment.value) {
      return '正在检测当前环境。'
    }

    if (environment.value.runtimeTarget === 'windowsWsl' && !environment.value.wslStatus?.ready) {
      return '请先完成 WSL / Ubuntu 初始化。'
    }

    if (!environment.value.targetSshInstalled) {
      return '请先补齐目标运行环境中的 OpenSSH。'
    }

    if (!environment.value.openclawInstalled) {
      return '请先安装 OpenClaw CLI。'
    }

    if (runtimeStatus.value.phase === 'connecting' || runtimeStatus.value.phase === 'running') {
      return '托管运行中时不能执行连接测试，请先停止托管。'
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
    if (operationTask.value.phase === 'running' && operationTask.value.kind === 'install') {
      return '安装中'
    }
    if (operationTask.value.phase === 'running' && operationTask.value.kind === 'update') {
      return '更新中'
    }
    if (operationTask.value.phase === 'cancelling') {
      return '停止中'
    }
    if (environment.value?.openclawInstalled) {
      return environment.value.openclawVersion ?? '已就绪'
    }
    return '待安装'
  })
  const openclawStateTone = computed(() => {
    if (operationTask.value.phase === 'running' || operationTask.value.phase === 'cancelling') {
      return 'active'
    }
    return environment.value?.openclawInstalled ? 'success' : 'warning'
  })
  const sshStateLabel = computed(() => {
    if (runtimeStatus.value.phase === 'running') {
      return runtimeStatus.value.sshConnected ? '已连接' : '未连接'
    }
    return environment.value?.targetSshInstalled ? '已就绪' : '待补齐'
  })
  const gatewayStateLabel = computed(() => {
    if (runtimeStatus.value.phase === 'connecting') {
      return '连接中'
    }
    return runtimeStatus.value.gatewayConnected ? '已连接' : '未连接'
  })
  const gatewayStateTone = computed(() => {
    if (runtimeStatus.value.phase === 'connecting') {
      return 'active'
    }
    return runtimeStatus.value.gatewayConnected ? 'success' : 'neutral'
  })
  const statusItems = computed(() => ([
    {
      label: 'Token',
      value: tokenStateLabel.value,
      tone: tokenStateToneValue.value,
    },
    {
      label: 'OpenClaw',
      value: openclawStateLabel.value,
      tone: openclawStateTone.value,
    },
    {
      label: 'SSH',
      value: sshStateLabel.value,
      tone: environment.value?.targetSshInstalled ? 'success' : 'warning',
    },
    {
      label: 'Gateway',
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
  const operationHeadline = computed(() => latestOperationDetail(operationEvents.value))
  const runtimeStatus = computed<RuntimeStatus>(() => environment.value?.runtimeStatus ?? {
    phase: 'checking',
    sshConnected: false,
    nodeConnected: false,
    gatewayConnected: false,
    lastError: null,
  })
  const platformLabel = computed(() => (
    environment.value ? runtimeTargetLabel(environment.value.runtimeTarget) : '检测中'
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
      return operationTask.value.lastResult?.followUp ?? '安装或更新未完成。'
    }

    return null
  })

  const overviewCards = computed(() => {
    const snapshot = environment.value
    if (!snapshot) {
      return []
    }

    return [
      {
        label: '目标运行时',
        value: platformLabel.value,
        detail: snapshot.runtimeTarget === 'windowsWsl'
          ? (snapshot.wslStatus?.ready
            ? `${snapshot.wslStatus.distroName} 已就绪`
            : snapshot.wslStatus?.message ?? '等待 WSL 初始化')
          : 'OpenClaw 与 SSH 在本机运行',
        tone: snapshot.runtimeTarget === 'windowsWsl' && !snapshot.wslStatus?.ready
          ? 'warning'
          : 'success',
      },
      {
        label: 'OpenClaw',
        value: snapshot.openclawInstalled
          ? (snapshot.openclawVersion ?? '已安装')
          : '未安装',
        detail: snapshot.updateAvailable
          ? `可更新到 ${snapshot.latestOpenclawVersion ?? '最新版本'}`
          : snapshot.installRecommendation,
        tone: snapshot.openclawInstalled ? (snapshot.updateAvailable ? 'active' : 'success') : 'warning',
      },
      {
        label: 'OpenSSH',
        value: snapshot.targetSshInstalled ? '已就绪' : '待补齐',
        detail: snapshot.runtimeTarget === 'windowsWsl'
          ? '检测 Ubuntu 内的 ssh'
          : '检测本机 ssh',
        tone: snapshot.targetSshInstalled ? 'success' : 'warning',
      },
      {
        label: '托管状态',
        value: runtimeDetail(runtimeStatus.value),
        detail: `当前阶段：${runtimeStatus.value.phase}`,
        tone: runtimeStatus.value.phase === 'running'
          ? 'success'
          : runtimeStatus.value.phase === 'error'
            ? 'error'
            : 'neutral',
      },
    ]
  })

  async function refreshEnvironment() {
    const snapshot = await detectEnvironment()
    environment.value = snapshot
    if (!hydratedFromSaved.value && snapshot.savedSettings) {
      Object.assign(
        companyProfile,
        draftFromCompanyProfile(snapshot.savedSettings.companyProfile),
      )
      Object.assign(userProfile, snapshot.savedSettings.userProfile)
      Object.assign(targetProfile, snapshot.savedSettings.targetProfile)
      hydratedFromSaved.value = true
    }
    logs.value = await streamLogs()
  }

  async function refreshOperationalState() {
    await refreshEnvironment()
    operationTask.value = await getOperationStatus()
    operationEvents.value = await getOperationEvents()
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

  function openConnectionTestModal() {
    connectionTestModal.open = true
    connectionTestModal.phase = 'running'
    connectionTestModal.summary = '正在保存配置并测试连接，请稍候。'
    connectionTestModal.result = null
    connectionTestModal.steps = createConnectionTestSteps()
    markConnectionTestStep('save', 'success', '配置已保存')
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

  async function installCli() {
    activeSection.value = 'install'
    resetOperationState()
    const result = await withActionError('install', () =>
      installOpenClaw({
        preferOfficial: true,
        allowElevation: false,
      }),
    )

    if (!result) {
      return
    }

    applyOperationTaskSnapshot(result)
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
    resetOperationState()
    const result = await withActionError('update', () =>
      updateOpenClaw({
        preferOfficial: true,
        allowElevation: false,
      }),
    )

    if (!result) {
      return
    }

    applyOperationTaskSnapshot(result)
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

  async function launchManualInstall() {
    manualInstallBusy.value = true
    try {
      await openManualInstall()
    } finally {
      manualInstallBusy.value = false
    }
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

    await refreshOperationalState()
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

    await refreshOperationalState()
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

    await refreshOperationalState()
  }

  onMounted(async () => {
    await refreshOperationalState()
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
        void refreshOperationalState()
      }),
    )
  })

  onBeforeUnmount(() => {
    while (unlistenCallbacks.length > 0) {
      unlistenCallbacks.pop()?.()
    }
  })

  return {
    activeSection,
    advancedOpen,
    canSaveProfile,
    canStartHostedRuntime,
    canTestConnection,
    checkForUpdates,
    closeConnectionTestModal,
    companyProfile,
    connectDisabledReason,
    canStopOperation,
    connectionTestBusy,
    connectionTestCloseDisabled,
    connectionTestDisabledReason,
    connectionTestModal,
    environment,
    installBusyAction,
    installCli,
    launchManualInstall,
    logs,
    manualInstallBusy,
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
    saveAndTest,
    saveBusy,
    saveOnly,
    sshPasswordInput,
    startHostedRuntime,
    statusItems,
    stopOperation,
    stopHostedRuntime,
    targetProfile,
    tokenInput,
    tokenStateLabel,
    tokenStateToneValue,
    gatewayStateLabel,
    gatewayStateTone,
    sshStateLabel,
    operationTaskPhaseLabel,
    updateCli,
    userProfile,
  }
}
