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
import { buildWorkflow } from '@/lib/runtime-view'
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
  const busyAction = ref<string | null>(null)
  const advancedOpen = ref(false)
  const tokenInput = ref('')
  const hydratedFromSaved = ref(false)
  const companyProfile = reactive<CompanyProfileDraft>(createEmptyCompanyProfileDraft())
  const userProfile = reactive(defaultUserProfile())
  const unlistenCallbacks: Array<() => void> = []

  const workflow = computed(() => buildWorkflow(environment.value))
  const companyProfileComplete = computed(() => isCompanyProfileDraftComplete(companyProfile))
  const displayNameReady = computed(() => userProfile.displayName.trim().length > 0)
  const tokenReady = computed(() => (
    tokenInput.value.trim().length > 0 || Boolean(environment.value?.hasSavedToken)
  ))
  const canSaveProfile = computed(() => (
    companyProfileComplete.value
    && displayNameReady.value
    && tokenReady.value
    && !busyAction.value
  ))
  const canConnect = computed(() => (
    canSaveProfile.value && Boolean(environment.value?.openclawInstalled)
  ))
  const runtimeStatus = computed<RuntimeStatus>(() => environment.value?.runtimeStatus ?? {
    phase: 'checking',
    sshConnected: false,
    nodeConnected: false,
    lastError: null,
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

  async function withBusy<T>(label: string, action: () => Promise<T>) {
    busyAction.value = label
    lastError.value = null
    try {
      return await action()
    } catch (error) {
      lastError.value = error instanceof Error ? error.message : String(error)
      throw error
    } finally {
      busyAction.value = null
    }
  }

  async function installCli() {
    installResult.value = await withBusy('install', () =>
      installOpenClaw({
        preferOfficial: true,
        allowElevation: true,
      }),
    )
    await refreshEnvironment()
  }

  async function launchManualInstall() {
    await withBusy('manual-install', () => openManualInstall())
  }

  async function saveOnly() {
    await withBusy('save', async () => {
      const normalizedCompanyProfile = normalizeCompanyProfileDraft(companyProfile)
      await saveProfile(
        normalizedCompanyProfile,
        { ...userProfile },
        tokenInput.value,
      )
      tokenInput.value = ''
      await refreshEnvironment()
    })
  }

  async function saveAndConnect() {
    await withBusy('connect', async () => {
      const normalizedCompanyProfile = normalizeCompanyProfileDraft(companyProfile)
      await saveProfile(
        normalizedCompanyProfile,
        { ...userProfile },
        tokenInput.value,
      )
      tokenInput.value = ''
      environment.value = {
        ...(environment.value ?? await detectEnvironment()),
        runtimeStatus: {
          ...runtimeStatus.value,
          phase: 'connecting',
        },
      }
      environment.value.runtimeStatus = await startRuntime()
      logs.value = await streamLogs()
      await refreshEnvironment()
    })
  }

  async function stopHostedRuntime() {
    await withBusy('stop', async () => {
      environment.value = environment.value && {
        ...environment.value,
        runtimeStatus: await stopRuntime(),
      }
      logs.value = await streamLogs()
      await refreshEnvironment()
    })
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
    canConnect,
    canSaveProfile,
    companyProfile,
    environment,
    installCli,
    installResult,
    lastError,
    launchManualInstall,
    logs,
    refreshEnvironment,
    runtimeStatus,
    saveAndConnect,
    saveOnly,
    stopHostedRuntime,
    tokenInput,
    userProfile,
    workflow,
  }
}
