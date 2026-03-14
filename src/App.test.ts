import { createApp, nextTick, reactive, ref } from 'vue'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { EnvironmentSnapshot, OperationTaskSnapshot } from '@/types'

const mockUiPreferences = {
  theme: 'light' as const,
  locale: 'zh-CN' as const,
}

vi.mock('@/lib/runtime-view', () => ({
  operationStepLabel: (step: string) => step,
  operationTaskPhaseLabel: (phase: string) => phase,
  phaseLabel: (phase: string) => phase,
}))

vi.mock('@/lib/use-app-model', () => {
  const environment = ref<EnvironmentSnapshot | null>({
    os: 'windows',
    runtimeTarget: 'windowsWsl',
    hostSshInstalled: false,
    targetSshInstalled: true,
    openclawInstalled: true,
    openclawVersion: 'OpenClaw 2026.3.8',
    latestOpenclawVersion: '2026.3.9',
    updateAvailable: true,
    hasSavedProfile: true,
    tokenStatus: 'saved',
    tokenStatusMessage: null,
    uiPreferences: {
      theme: 'light',
      locale: 'zh-CN',
    },
    savedSettings: {
      companyProfile: {
        sshHost: 'gateway.example.com',
        sshUser: 'bizclaw',
        localPort: 18889,
        remoteBindHost: '127.0.0.1',
        remoteBindPort: 18789,
      },
      userProfile: {
        displayName: 'BizClaw',
        autoConnect: true,
        runInBackground: true,
      },
      targetProfile: {
        wslDistro: 'Ubuntu',
      },
    },
    runtimeStatus: {
      phase: 'configured',
      sshConnected: false,
      nodeConnected: false,
      gatewayConnected: false,
      lastError: null,
    },
    installRecommendation: 'Windows 通过 WSL 运行',
    wslStatus: {
      available: true,
      distroName: 'Ubuntu',
      distroInstalled: true,
      ready: true,
      needsReboot: false,
      message: null,
    },
  })
  const operationTask = ref<OperationTaskSnapshot>({
    phase: 'running',
    kind: 'install',
    step: 'installOpenClaw',
    canStop: true,
    lastResult: null,
    startedAt: Date.now(),
    endedAt: null,
  })
  const connectionTestModal = ref({
    open: false,
    phase: 'idle',
    summary: '',
    result: null,
    steps: [
      {
        step: 'save',
        label: '配置已保存',
        status: 'success',
        message: '配置已保存',
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
    ],
  })

  return {
    useAppModel: () => ({
      activeSection: ref<'overview' | 'install' | 'connection' | 'runtime'>('overview'),
      advancedOpen: ref(true),
      busyAction: ref<string | null>(null),
      canSaveProfile: ref(true),
      canStartHostedRuntime: ref(true),
      canTestConnection: ref(true),
      checkForUpdates: vi.fn(),
      closeConnectionTestModal: vi.fn(),
      connectionTestBusy: ref(false),
      connectionTestDisabledReason: ref<string | null>(null),
      connectionTestCloseDisabled: ref(false),
      connectionTestModal,
      companyProfile: reactive({
        sshHost: 'gateway.example.com',
        sshUser: 'bizclaw',
        localPort: '18889',
        remoteBindHost: '127.0.0.1',
        remoteBindPort: '18789',
      }),
      connectDisabledReason: ref(''),
      canStopOperation: ref(true),
      environment,
      installCli: vi.fn(),
      installBusyAction: ref<string | null>(null),
      launchManualInstall: vi.fn(),
      manualInstallBusy: ref(false),
      logs: ref([
        {
          source: 'system',
          level: 'info',
          message: 'ready',
          timestampMs: Date.now(),
        },
      ]),
      operationError: ref<string | null>(null),
      operationEvents: ref([
        {
          kind: 'install',
          step: 'installOpenClaw',
          status: 'log',
          source: 'stdout',
          message: 'installing...',
          timestampMs: Date.now(),
        },
      ]),
      operationHeadline: ref('installing...'),
      operationResult: ref(null),
      operationTask,
      operationTaskPhaseLabel: (phase: string) => phase,
      openclawStateLabel: ref('OpenClaw 2026.3.8'),
      openclawStateTone: ref('success'),
      operationsSummary: ref({
        title: '正在安装 OpenClaw',
        detail: 'BizClaw 正在后台执行官方安装器。',
        tone: 'active',
      }),
      overviewCards: ref([
      {
        label: '目标运行时',
        value: 'Windows WSL',
        detail: 'Ubuntu 已就绪',
        tone: 'success',
      },
    ]),
      bizclawUpdate: ref({
        phase: 'available',
        currentVersion: '0.1.8',
        latestVersion: '0.1.9',
        releaseNotes: 'Bug fixes',
        publishedAt: '2026-03-14T00:00:00.000Z',
        downloadedBytes: 0,
        totalBytes: null,
        errorMessage: null,
      }),
      bizclawUpdateActionLabel: ref('检测到新版本'),
      bizclawUpdateDetail: ref('当前 0.1.8，可更新到 0.1.9'),
      bizclawUpdatePrimaryAction: ref('立即更新'),
      bizclawUpdateTone: ref('active'),
      bizclawUpdateBlockedReason: ref<string | null>(null),
      checkBizClawUpdates: vi.fn(),
      installBizClawUpdate: vi.fn(),
      restartBizClaw: vi.fn(),
      deferBizClawRestart: vi.fn(),
      platformLabel: ref('Windows WSL'),
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
      setLocale: vi.fn(),
      setTheme: vi.fn(),
      targetProfile: reactive({
        wslDistro: 'Ubuntu',
      }),
      tokenInput: ref('saved-token'),
      gatewayStateLabel: ref('未连接'),
      gatewayStateTone: ref('neutral'),
      tokenStateLabel: ref('Token 已保存'),
      tokenStateToneValue: ref('success'),
      uiPreferences: ref({
        theme: mockUiPreferences.theme,
        locale: mockUiPreferences.locale,
      }),
      updateCli: vi.fn(),
      userProfile: reactive({
        displayName: 'BizClaw',
        autoConnect: true,
        runInBackground: true,
      }),
    }),
  }
})

describe('App operations center', () => {
  let host: HTMLDivElement | null = null
  let app: ReturnType<typeof createApp> | null = null

  afterEach(() => {
    app?.unmount()
    host?.remove()
    host = null
    app = null
    mockUiPreferences.theme = 'light'
    mockUiPreferences.locale = 'zh-CN'
  })

  it('renders the compact sidebar navigation shell', async () => {
    const { default: App } = await import('./App.vue')

    host = document.createElement('div')
    document.body.appendChild(host)

    app = createApp(App)
    app.mount(host)
    await nextTick()

    expect(host.querySelector('main.ops-shell')).not.toBeNull()
    const navButtons = Array.from(host.querySelectorAll('.sidebar-nav .nav-button')).map((node) =>
      node.textContent?.trim(),
    )

    expect(navButtons).toEqual(['概览', '安装与更新', '连接与配置', '运行日志', '设置'])
    expect(host.textContent).toContain('BIZCLAW')
    expect(host.textContent).not.toContain('操作中心')
    expect(host.textContent).not.toContain('macOS 本机')
    expect(host.querySelector('.sidebar-summary')).toBeNull()
  })

  it('shows WSL distro and SSH password in advanced connection settings', async () => {
    const { default: App } = await import('./App.vue')

    host = document.createElement('div')
    document.body.appendChild(host)

    app = createApp(App)
    app.mount(host)
    await nextTick()

    const button = Array.from(host.querySelectorAll<HTMLButtonElement>('button'))
      .find((node) => node.textContent?.includes('连接与配置'))
    button?.click()
    await nextTick()

    const labels = Array.from(host.querySelectorAll('.advanced-grid .field span'))
      .map((node) => node.textContent?.trim())

    expect(labels).toContain('WSL Distro')
    expect(labels).toContain('SSH Password')
  })

  it('renders save and test plus hosted runtime controls on the connection page', async () => {
    const { default: App } = await import('./App.vue')

    host = document.createElement('div')
    document.body.appendChild(host)

    app = createApp(App)
    app.mount(host)
    await nextTick()

    const button = Array.from(host.querySelectorAll<HTMLButtonElement>('button'))
      .find((node) => node.textContent?.includes('连接与配置'))
    button?.click()
    await nextTick()

    expect(host.textContent).toContain('托管运行时')
    expect(host.textContent).toContain('保存并测试')
    expect(host.textContent).toContain('启动托管')
    expect(host.textContent).toContain('停止托管')
  })

  it('renders the live install console and stop control on the install page', async () => {
    const { default: App } = await import('./App.vue')

    host = document.createElement('div')
    document.body.appendChild(host)

    app = createApp(App)
    app.mount(host)
    await nextTick()

    const button = Array.from(host.querySelectorAll<HTMLButtonElement>('button'))
      .find((node) => node.textContent?.includes('安装与更新'))
    button?.click()
    await nextTick()

    expect(host.textContent).toContain('安装 / 更新输出')
    expect(host.textContent).toContain('installing...')
    expect(host.textContent).toContain('停止')
  })

  it('keeps BizClaw app updates out of the install page', async () => {
    const { default: App } = await import('./App.vue')

    host = document.createElement('div')
    document.body.appendChild(host)

    app = createApp(App)
    app.mount(host)
    await nextTick()

    const button = Array.from(host.querySelectorAll<HTMLButtonElement>('button'))
      .find((node) => node.textContent?.includes('安装与更新'))
    button?.click()
    await nextTick()

    expect(host.textContent).not.toContain('BizClaw 应用更新')
    expect(host.textContent).not.toContain('当前 BizClaw 版本')
    expect(host.textContent).not.toContain('最新 BizClaw 版本')
    expect(host.textContent).not.toContain('立即更新')
  })

  it('renders theme, locale, and BizClaw version controls on the settings page', async () => {
    const { default: App } = await import('./App.vue')

    host = document.createElement('div')
    document.body.appendChild(host)

    app = createApp(App)
    app.mount(host)
    await nextTick()

    const button = Array.from(host.querySelectorAll<HTMLButtonElement>('button'))
      .find((node) => node.textContent?.includes('设置'))
    button?.click()
    await nextTick()

    expect(host.textContent).toContain('主题')
    expect(host.textContent).toContain('国际化')
    expect(host.textContent).toContain('版本')
    expect(host.textContent).toContain('跟随系统')
    expect(host.textContent).toContain('当前 BizClaw 版本')
    expect(host.textContent).toContain('最新 BizClaw 版本')
    expect(host.textContent).toContain('立即更新')
  })

  it('shows the global status bar instead of a sidebar summary', async () => {
    const { default: App } = await import('./App.vue')

    host = document.createElement('div')
    document.body.appendChild(host)

    app = createApp(App)
    app.mount(host)
    await nextTick()

    const statusBar = host.querySelector('.status-bar')

    expect(statusBar).not.toBeNull()
    expect(statusBar?.textContent).toContain('Token')
    expect(statusBar?.textContent).toContain('OpenClaw')
    expect(statusBar?.textContent).toContain('SSH')
    expect(statusBar?.textContent).toContain('Gateway')
    expect(statusBar?.textContent).toContain('未连接')
  })

  it('keeps the runtime page focused on logs only while install work can continue in background', async () => {
    const { default: App } = await import('./App.vue')

    host = document.createElement('div')
    document.body.appendChild(host)

    app = createApp(App)
    app.mount(host)
    await nextTick()

    const button = Array.from(host.querySelectorAll<HTMLButtonElement>('button'))
      .find((node) => node.textContent?.includes('运行日志'))
    button?.click()
    await nextTick()

    expect(host.textContent).toContain('最近日志')
    expect(host.textContent).not.toContain('启动托管')
    expect(host.textContent).not.toContain('停止托管')
    expect(host.textContent).toContain('ready')
  })
})
