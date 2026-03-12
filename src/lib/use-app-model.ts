import { computed, onBeforeUnmount, onMounted, reactive, ref } from 'vue'

import {
  detectEnvironment,
  installOpenClaw,
  onRuntimeLog,
  onRuntimeStatus,
  openManualInstall,
  saveProfile,
  startRuntime,
  stopRuntime,
  streamLogs,
} from '@/lib/api'
import {
  createEmptyCompanyProfileDraft,
  draftFromCompanyProfile,
  isCompanyProfileDraftComplete,
  normalizeCompanyProfileDraft,
} from '@/lib/profile-form'
import {
  buildInstallSummary,
  buildWorkflow,
  shouldShowInstallConsole,
  startRuntimeDisabledReason,
  tokenStatusLabel,
  tokenStatusTone,
} from '@/lib/runtime-view'
import type {
  CompanyProfileDraft,
  EnvironmentSnapshot,
  InstallResult,
  LogEntry,
  RuntimeStatus,
  UserProfile,
} from '@/types'

const defaultUserProfile = (): UserProfile => ({
  displayName: '',
  autoConnect: true,
  runInBackground: true,
})

export function useAppModel() {
  const environment = ref<EnvironmentSnapshot | null>(null)
  const installResult = ref<InstallResult | null>(null)
  const logs = ref<LogEntry[]>([])
  const lastError = ref<string | null>(null)
  const lastErrorAction = ref<string | null>(null)
  const busyAction = ref<string | null>(null)
  const advancedOpen = ref(false)
  const sshPasswordInput = ref('')
  const tokenInput = ref('')
  const hydratedFromSaved = ref(false)
  const companyProfile = reactive<CompanyProfileDraft>(createEmptyCompanyProfileDraft())
  const userProfile = reactive(defaultUserProfile())
  const unlistenCallbacks: Array<() => void> = []

  const workflow = computed(() => buildWorkflow(environment.value))
  const companyProfileComplete = computed(() => isCompanyProfileDraftComplete(companyProfile))
  const displayNameReady = computed(() => userProfile.displayName.trim().length > 0)
  const tokenInputReady = computed(() => tokenInput.value.trim().length > 0)
  const tokenStored = computed(() => environment.value?.tokenStatus === 'saved')
  const tokenReady = computed(() => tokenInputReady.value || tokenStored.value)
  const canSaveProfile = computed(() => (
    companyProfileComplete.value
    && displayNameReady.value
    && tokenReady.value
    && !busyAction.value
  ))
  const canSaveAndConnect = computed(() => (
    canSaveProfile.value && Boolean(environment.value?.openclawInstalled)
  ))
  const connectDisabledReason = computed(() => (
    startRuntimeDisabledReason(environment.value, busyAction.value)
  ))
  const canStartHostedRuntime = computed(() => !connectDisabledReason.value)
  const tokenStateLabel = computed(() => tokenStatusLabel(environment.value))
  const tokenStateToneValue = computed(() => tokenStatusTone(environment.value))
  const installSummary = computed(() => buildInstallSummary(
    environment.value,
    installResult.value,
    busyAction.value,
  ))
  const showInstallConsole = computed(() => shouldShowInstallConsole(installResult.value))
  const runtimeStatus = computed<RuntimeStatus>(() => environment.value?.runtimeStatus ?? {
    phase: 'checking',
    sshConnected: false,
    nodeConnected: false,
    lastError: null,
  })
  const profileError = computed(() => {
    if (lastErrorAction.value === 'save' || lastErrorAction.value === 'connect') {
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

  async function refreshEnvironment() {
    const snapshot = await detectEnvironment()
    environment.value = snapshot
    if (!hydratedFromSaved.value && snapshot.savedSettings) {
      Object.assign(
        companyProfile,
        draftFromCompanyProfile(snapshot.savedSettings.companyProfile),
      )
      Object.assign(userProfile, snapshot.savedSettings.userProfile)
      hydratedFromSaved.value = true
    }
    logs.value = await streamLogs()
  }

  async function withBusy<T>(label: string, action: () => Promise<T>): Promise<T | null> {
    busyAction.value = label
    lastError.value = null
    lastErrorAction.value = null

    try {
      const result = await action()
      lastErrorAction.value = null
      return result
    } catch (error) {
      lastError.value = error instanceof Error ? error.message : String(error)
      lastErrorAction.value = label
      return null
    } finally {
      busyAction.value = null
    }
  }

  async function installCli() {
    const result = await withBusy('install', () =>
      installOpenClaw({
        preferOfficial: true,
        allowElevation: true,
      }),
    )

    if (!result) {
      return
    }

    installResult.value = result
    await refreshEnvironment()
  }

  async function launchManualInstall() {
    await withBusy('manual-install', () => openManualInstall())
  }

  async function saveOnly() {
    const result = await withBusy('save', async () => {
      const normalizedCompanyProfile = normalizeCompanyProfileDraft(companyProfile)
      return saveProfile(
        normalizedCompanyProfile,
        { ...userProfile },
        tokenInput.value,
        sshPasswordInput.value,
      )
    })

    if (!result) {
      return
    }

    tokenInput.value = ''
    sshPasswordInput.value = ''
    await refreshEnvironment()
  }

  async function saveAndConnect() {
    const result = await withBusy('connect', async () => {
      const normalizedCompanyProfile = normalizeCompanyProfileDraft(companyProfile)
      await saveProfile(
        normalizedCompanyProfile,
        { ...userProfile },
        tokenInput.value,
        sshPasswordInput.value,
      )

      tokenInput.value = ''
      sshPasswordInput.value = ''
      return startHostedRuntimeCore()
    })

    if (!result) {
      return
    }

    await refreshEnvironment()
  }

  async function startHostedRuntime() {
    const result = await withBusy('start', async () => startHostedRuntimeCore())

    if (!result) {
      return
    }

    await refreshEnvironment()
  }

  async function startHostedRuntimeCore() {
    const previousEnvironment = environment.value ?? await detectEnvironment()
    environment.value = {
      ...previousEnvironment,
      runtimeStatus: {
        ...runtimeStatus.value,
        phase: 'connecting',
      },
    }

    try {
      environment.value.runtimeStatus = await startRuntime()
      logs.value = await streamLogs()
      return environment.value.runtimeStatus
    } catch (error) {
      environment.value = previousEnvironment
      throw error
    }
  }

  async function stopHostedRuntime() {
    const result = await withBusy('stop', async () => {
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
    await refreshEnvironment()
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
  })

  onBeforeUnmount(() => {
    while (unlistenCallbacks.length > 0) {
      unlistenCallbacks.pop()?.()
    }
  })

  return {
    advancedOpen,
    busyAction,
    canSaveAndConnect,
    canSaveProfile,
    canStartHostedRuntime,
    companyProfile,
    connectDisabledReason,
    environment,
    installCli,
    installResult,
    installSummary,
    lastError,
    launchManualInstall,
    logs,
    profileError,
    refreshEnvironment,
    runtimeError,
    runtimeStatus,
    saveAndConnect,
    saveOnly,
    showInstallConsole,
    startHostedRuntime,
    stopHostedRuntime,
    sshPasswordInput,
    tokenInput,
    tokenStateLabel,
    tokenStateToneValue,
    userProfile,
    workflow,
  }
}
