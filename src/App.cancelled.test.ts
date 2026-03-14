import { createApp, nextTick, reactive, ref } from 'vue'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { EnvironmentSnapshot, OperationTaskSnapshot } from '@/types'

vi.mock('@/lib/runtime-view', () => ({
  operationStepLabel: (step: string) => step,
  phaseLabel: (phase: string) => phase,
}))

vi.mock('@/lib/use-app-model', () => {
  const environment = ref<EnvironmentSnapshot | null>({
    os: 'macos',
    runtimeTarget: 'macNative',
    hostSshInstalled: true,
    targetSshInstalled: true,
    openclawInstalled: true,
    openclawVersion: 'OpenClaw 2026.3.8',
    latestOpenclawVersion: '2026.3.9',
    updateAvailable: true,
    hasSavedProfile: true,
    tokenStatus: 'saved',
    tokenStatusMessage: null,
    savedSettings: null,
    runtimeStatus: {
      phase: 'configured',
      sshConnected: false,
      nodeConnected: false,
      gatewayConnected: false,
      lastError: null,
    },
    installRecommendation: 'macOS',
    wslStatus: null,
  })
  const operationTask = ref<OperationTaskSnapshot>({
    phase: 'cancelled',
    kind: 'install',
    step: 'installOpenClaw',
    canStop: false,
    lastResult: {
      kind: 'install',
      strategy: 'official',
      success: false,
      step: 'installOpenClaw',
      stdout: '',
      stderr: '',
      needsElevation: false,
      manualUrl: 'https://docs.openclaw.ai/install',
      followUp: '安装已停止。',
    },
    startedAt: Date.now() - 1_000,
    endedAt: Date.now(),
  })

  return {
    useAppModel: () => ({
      activeSection: ref<'overview' | 'install' | 'connection' | 'runtime'>('install'),
      advancedOpen: ref(false),
      bizclawUpdate: ref({
        phase: 'idle',
        currentVersion: '0.1.8',
        latestVersion: null,
        releaseNotes: null,
        publishedAt: null,
        downloadedBytes: 0,
        totalBytes: null,
        errorMessage: null,
      }),
      bizclawUpdateActionLabel: ref('已加载当前版本'),
      bizclawUpdateBlockedReason: ref<string | null>(null),
      bizclawUpdateDetail: ref('当前 BizClaw 版本为 0.1.8。'),
      bizclawUpdatePrimaryAction: ref('立即更新'),
      bizclawUpdateTone: ref('neutral'),
      checkBizClawUpdates: vi.fn(),
      canSaveProfile: ref(true),
      canStartHostedRuntime: ref(false),
      canTestConnection: ref(false),
      checkForUpdates: vi.fn(),
      closeConnectionTestModal: vi.fn(),
      companyProfile: reactive({
        sshHost: 'gateway.example.com',
        sshUser: 'bizclaw',
        localPort: '18889',
        remoteBindHost: '127.0.0.1',
        remoteBindPort: '18789',
      }),
      connectDisabledReason: ref(''),
      canStopOperation: ref(false),
      connectionTestBusy: ref(false),
      connectionTestCloseDisabled: ref(false),
      connectionTestDisabledReason: ref<string | null>(null),
      connectionTestModal: ref({
        open: false,
        phase: 'idle',
        summary: '',
        result: null,
        steps: [],
      }),
      deferBizClawRestart: vi.fn(),
      environment,
      installBusyAction: ref<string | null>(null),
      installCli: vi.fn(),
      installBizClawUpdate: vi.fn(),
      launchManualInstall: vi.fn(),
      logs: ref([]),
      manualInstallBusy: ref(false),
      operationError: ref<string | null>(null),
      operationEvents: ref([
        {
          kind: 'install',
          step: 'installOpenClaw',
          status: 'cancelled',
          source: 'system',
          message: 'official 已停止',
          timestampMs: Date.now(),
        },
      ]),
      operationHeadline: ref('official 已停止'),
      operationResult: ref(operationTask.value.lastResult),
      operationTask,
      operationTaskPhaseLabel: (phase: string) => phase,
      openclawStateLabel: ref('OpenClaw 2026.3.8'),
      openclawStateTone: ref('success'),
      operationsSummary: ref({
        title: '任务已停止',
        detail: '安装已停止。',
        tone: 'active',
      }),
      overviewCards: ref([]),
      platformLabel: ref('macOS 本机'),
      profileError: ref<string | null>(null),
      refreshEnvironment: vi.fn(),
      runtimeError: ref<string | null>(null),
      runtimeStartBusy: ref(false),
      runtimeStatus: ref({
        phase: 'configured',
        sshConnected: false,
        nodeConnected: false,
        gatewayConnected: false,
        lastError: null,
      }),
      runtimeStopBusy: ref(false),
      restartBizClaw: vi.fn(),
      saveAndTest: vi.fn(),
      saveBusy: ref(false),
      saveOnly: vi.fn(),
      sshPasswordInput: ref(''),
      sshStateLabel: ref('已就绪'),
      startHostedRuntime: vi.fn(),
      statusItems: ref([
        { label: 'Token', value: 'Token 已保存', tone: 'success' },
        { label: 'OpenClaw', value: 'OpenClaw 2026.3.8', tone: 'success' },
        { label: 'SSH', value: '已就绪', tone: 'success' },
        { label: 'Gateway', value: '未连接', tone: 'neutral' },
      ]),
      stopHostedRuntime: vi.fn(),
      stopOperation: vi.fn(),
      targetProfile: reactive({
        wslDistro: 'Ubuntu',
      }),
      tokenInput: ref('saved-token'),
      gatewayStateLabel: ref('未连接'),
      gatewayStateTone: ref('neutral'),
      tokenStateLabel: ref('Token 已保存'),
      tokenStateToneValue: ref('success'),
      updateCli: vi.fn(),
      userProfile: reactive({
        displayName: 'BizClaw',
        autoConnect: true,
        runInBackground: true,
      }),
    }),
  }
})

describe('App cancelled operation state', () => {
  let host: HTMLDivElement | null = null
  let app: ReturnType<typeof createApp> | null = null

  afterEach(() => {
    app?.unmount()
    host?.remove()
    host = null
    app = null
  })

  it('renders a stopped task without failure messaging', async () => {
    const { default: App } = await import('./App.vue')

    host = document.createElement('div')
    document.body.appendChild(host)

    app = createApp(App)
    app.mount(host)
    await nextTick()

    expect(host.textContent).toContain('任务已停止')
    expect(host.textContent).toContain('安装已停止。')
    expect(host.textContent).not.toContain('操作未完成')
  })
})
