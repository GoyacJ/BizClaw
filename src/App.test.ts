import { createApp, nextTick, reactive, ref } from 'vue'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type {
  EnvironmentSnapshot,
  OpenClawAgentBinding,
  OpenClawAgentSummary,
  OpenClawSkillCheckReport,
  OpenClawSkillInfo,
  OpenClawSkillInventory,
  OperationTaskSnapshot,
} from '@/types'

const mockUiPreferences = {
  theme: 'light' as const,
  locale: 'zh-CN' as const,
  sidebarCollapsed: false,
}

let uiPreferencesRef = ref({
  theme: mockUiPreferences.theme,
  locale: mockUiPreferences.locale,
  sidebarCollapsed: mockUiPreferences.sidebarCollapsed,
})

const setSidebarCollapsedMock = vi.fn(async (sidebarCollapsed: boolean) => {
  uiPreferencesRef.value = {
    ...uiPreferencesRef.value,
    sidebarCollapsed,
  }
})

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
    hostOpenclawInstalled: false,
    targetSshInstalled: true,
    openclawInstalled: true,
    openclawVersion: 'OpenClaw 2026.3.8',
    latestOpenclawVersion: '2026.3.9',
    updateAvailable: true,
    wslOpenclawInstalled: true,
    hasSavedProfile: true,
    tokenStatus: 'saved',
    tokenStatusMessage: null,
    uiPreferences: {
      theme: 'light',
      locale: 'zh-CN',
      sidebarCollapsed: false,
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
  const agents = ref<OpenClawAgentSummary[]>([
    {
      id: 'main',
      name: 'Main',
      identityName: '霸天 (Bàtiān)',
      identityEmoji: '🐯',
      identitySource: 'identity',
      workspace: '/Users/goya/.openclaw/workspace',
      agentDir: '/Users/goya/.openclaw/agents/main/agent',
      model: 'minimax-cn/MiniMax-M2.5',
      bindings: 1,
      isDefault: true,
      routes: ['telegram default'],
    },
    {
      id: 'ops',
      name: 'Ops',
      identityName: '值班',
      identityEmoji: '🛠',
      identitySource: 'config',
      workspace: '/Users/goya/.openclaw/workspace-ops',
      agentDir: '/Users/goya/.openclaw/agents/ops/agent',
      model: 'minimax-cn/MiniMax-M2.5',
      bindings: 0,
      isDefault: false,
      routes: [],
    },
  ])
  const agentBindings = ref<OpenClawAgentBinding[]>([
    {
      agentId: 'main',
      channel: 'telegram',
      accountId: null,
      description: 'telegram (default account)',
    },
  ])
  const skillsInventory = ref<OpenClawSkillInventory>({
    workspaceDir: '/Users/goya/.openclaw/workspace',
    managedSkillsDir: '/Users/goya/.openclaw/skills',
    skills: [
      {
        name: 'sonoscli',
        description: 'Control Sonos speakers.',
        eligible: true,
        disabled: false,
        blockedByAllowlist: false,
        source: 'openclaw-workspace',
        bundled: false,
        locationKind: 'workspaceLocal',
        canDelete: true,
        missing: {
          bins: [],
          anyBins: [],
          env: [],
          config: [],
          os: [],
        },
      },
      {
        name: 'coding-agent',
        description: 'Delegate coding tasks.',
        eligible: true,
        disabled: false,
        blockedByAllowlist: false,
        source: 'openclaw-bundled',
        bundled: true,
        locationKind: 'bundled',
        canDelete: false,
        missing: {
          bins: [],
          anyBins: [],
          env: [],
          config: [],
          os: [],
        },
      },
    ],
  })
  const skillsCheck = ref<OpenClawSkillCheckReport>({
    summary: {
      total: 2,
      eligible: 2,
      disabled: 0,
      blocked: 0,
      missingRequirements: 0,
    },
    eligible: ['sonoscli', 'coding-agent'],
    disabled: [],
    blocked: [],
    missingRequirements: [],
  })
  const selectedSkillInfo = ref<OpenClawSkillInfo | null>({
    name: 'sonoscli',
    description: 'Control Sonos speakers.',
    source: 'openclaw-workspace',
    bundled: false,
    locationKind: 'workspaceLocal',
    canDelete: true,
    filePath: '/Users/goya/.openclaw/workspace/skills/sonoscli/SKILL.md',
    baseDir: '/Users/goya/.openclaw/workspace/skills/sonoscli',
    skillKey: 'sonoscli',
    homepage: 'https://sonoscli.sh',
    always: false,
    disabled: false,
    blockedByAllowlist: false,
    eligible: true,
    requirements: {
      bins: [],
      anyBins: [],
      env: [],
      config: [],
      os: [],
    },
    missing: {
      bins: [],
      anyBins: [],
      env: [],
      config: [],
      os: [],
    },
    configChecks: [],
    install: [],
  })

  return {
    useAppModel: () => ({
      activeSection: ref<'overview' | 'agent' | 'install' | 'connection' | 'runtime' | 'skill' | 'settings'>('overview'),
      advancedOpen: ref(true),
      agentsState: {
        agents,
        bindings: agentBindings,
        bindingsLoading: ref(false),
        selectedAgentId: ref('main'),
        loading: ref(false),
        mutationBusy: ref(false),
        error: ref<string | null>(null),
        refreshAgents: vi.fn(),
        selectAgent: vi.fn(),
        createAgent: vi.fn(),
        updateAgentIdentity: vi.fn(),
        deleteAgent: vi.fn(),
        addAgentBindings: vi.fn(),
        removeAgentBindings: vi.fn(),
        clearAgentBindings: vi.fn(),
      },
      busyAction: ref<string | null>(null),
      canSaveProfile: ref(true),
      canStartHostedRuntime: ref(true),
      canTestConnection: ref(true),
      chooseWindowsInstallTarget: vi.fn(),
      checkForUpdates: vi.fn(),
      closeConnectionTestModal: vi.fn(),
      closeInstallRemediationModal: vi.fn(),
      closeWindowsInstallChoiceModal: vi.fn(),
      connectionTestBusy: ref(false),
      connectionTestDisabledReason: ref<string | null>(null),
      connectionTestCloseDisabled: ref(false),
      connectionTestModal,
      confirmInstallRemediation: vi.fn(),
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
      installRemediationModal: reactive({
        open: false,
        kind: null,
        actionKind: null,
        title: '',
        detail: '',
        confirmLabel: '',
        supportLabel: '',
        supportUrlTarget: null,
      }),
      launchManualInstall: vi.fn(),
      manualInstallBusy: ref(false),
      openInstallRemediationSupportUrl: vi.fn(),
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
      skillsState: {
        inventory: skillsInventory,
        checkReport: skillsCheck,
        searchResults: ref([]),
        selectedSkillName: ref('sonoscli'),
        selectedSkillInfo,
        loading: ref(false),
        searchBusy: ref(false),
        detailLoading: ref(false),
        mutationBusy: ref(false),
        error: ref<string | null>(null),
        refreshSkills: vi.fn(),
        selectSkill: vi.fn(),
        searchInstallableSkills: vi.fn(),
        clearSkillSearch: vi.fn(),
        installSkill: vi.fn(),
        deleteLocalSkill: vi.fn(),
      },
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
      uiPreferences: uiPreferencesRef,
      updateCli: vi.fn(),
      userProfile: reactive({
        displayName: 'BizClaw',
        autoConnect: true,
        runInBackground: true,
      }),
      windowsInstallChoiceModalOpen: ref(false),
      setSidebarCollapsed: setSidebarCollapsedMock,
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
    mockUiPreferences.sidebarCollapsed = false
    uiPreferencesRef.value = {
      theme: mockUiPreferences.theme,
      locale: mockUiPreferences.locale,
      sidebarCollapsed: mockUiPreferences.sidebarCollapsed,
    }
    setSidebarCollapsedMock.mockClear()
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

    expect(navButtons).toEqual(['概览', '代理', '安装与更新', '连接与配置', '运行日志', '技能', '设置'])
    expect(host.textContent).toContain('BIZCLAW')
    expect(host.textContent).not.toContain('操作中心')
    expect(host.textContent).not.toContain('macOS 本机')
    expect(host.querySelector('.sidebar-summary')).toBeNull()
  })

  it('lets the sidebar collapse into an icon rail while preserving accessible labels', async () => {
    const { default: App } = await import('./App.vue')

    host = document.createElement('div')
    document.body.appendChild(host)

    app = createApp(App)
    app.mount(host)
    await nextTick()

    const sidebar = host.querySelector('.sidebar')
    const toggle = host.querySelector<HTMLButtonElement>('.sidebar-toggle')

    expect(sidebar?.getAttribute('data-collapsed')).toBe('false')
    expect(toggle?.getAttribute('aria-label')).toContain('折叠')

    toggle?.click()
    await nextTick()

    expect(setSidebarCollapsedMock).toHaveBeenCalledWith(true)
    expect(host.querySelector('.sidebar')?.getAttribute('data-collapsed')).toBe('true')

    const firstNavButton = host.querySelector<HTMLButtonElement>('.sidebar-nav .nav-button')
    expect(firstNavButton?.getAttribute('aria-label')).toBe('概览')
    expect(firstNavButton?.title).toBe('概览')
  })

  it('temporarily expands a collapsed sidebar while hovered without changing the saved preference', async () => {
    mockUiPreferences.sidebarCollapsed = true
    uiPreferencesRef.value = {
      ...uiPreferencesRef.value,
      sidebarCollapsed: true,
    }

    const { default: App } = await import('./App.vue')

    host = document.createElement('div')
    document.body.appendChild(host)

    app = createApp(App)
    app.mount(host)
    await nextTick()

    const shell = host.querySelector('main.ops-shell')
    const sidebar = host.querySelector<HTMLElement>('.sidebar')

    expect(shell?.getAttribute('data-sidebar-collapsed')).toBe('true')
    expect(sidebar?.getAttribute('data-collapsed')).toBe('true')
    expect(host.querySelector('.nav-button-label')).toBeNull()

    sidebar?.dispatchEvent(new MouseEvent('mouseenter'))
    await nextTick()

    expect(setSidebarCollapsedMock).not.toHaveBeenCalled()
    expect(shell?.getAttribute('data-sidebar-collapsed')).toBe('false')
    expect(sidebar?.getAttribute('data-collapsed')).toBe('false')
    expect(host.querySelector('.nav-button-label')?.textContent).toBe('概览')

    sidebar?.dispatchEvent(new MouseEvent('mouseleave'))
    await nextTick()

    expect(shell?.getAttribute('data-sidebar-collapsed')).toBe('true')
    expect(sidebar?.getAttribute('data-collapsed')).toBe('true')
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

  it('renders agent management content on the agent page', async () => {
    const { default: App } = await import('./App.vue')

    host = document.createElement('div')
    document.body.appendChild(host)

    app = createApp(App)
    app.mount(host)
    await nextTick()

    const button = Array.from(host.querySelectorAll<HTMLButtonElement>('button'))
      .find((node) => node.textContent?.includes('代理'))
    button?.click()
    await nextTick()

    expect(host.textContent).toContain('代理')
    expect(host.textContent).toContain('main')
    expect(host.textContent).toContain('霸天 (Bàtiān)')
    expect(host.textContent).toContain('工作区')
    expect(host.textContent).toContain('telegram (default account)')
  })

  it('renders skill inventory and keeps bundled skills read-only on the skill page', async () => {
    const { default: App } = await import('./App.vue')

    host = document.createElement('div')
    document.body.appendChild(host)

    app = createApp(App)
    app.mount(host)
    await nextTick()

    const button = Array.from(host.querySelectorAll<HTMLButtonElement>('button'))
      .find((node) => node.textContent?.includes('技能'))
    button?.click()
    await nextTick()

    expect(host.textContent).toContain('技能')
    expect(host.textContent).toContain('sonoscli')
    expect(host.textContent).toContain('coding-agent')
    expect(host.textContent).toContain('可删除')
    expect(host.textContent).toContain('只读')
    expect(host.textContent).toContain('/Users/goya/.openclaw/workspace/skills/sonoscli/SKILL.md')
  })

  it('opens and closes the create-agent modal on the agent page', async () => {
    const { default: App } = await import('./App.vue')

    host = document.createElement('div')
    document.body.appendChild(host)

    app = createApp(App)
    app.mount(host)
    await nextTick()

    const agentNavButton = Array.from(host.querySelectorAll<HTMLButtonElement>('button'))
      .find((node) => node.textContent?.includes('代理'))
    agentNavButton?.click()
    await nextTick()

    const openButton = Array.from(host.querySelectorAll<HTMLButtonElement>('button'))
      .find((node) => node.textContent?.includes('创建代理'))
    openButton?.click()
    await nextTick()

    expect(document.body.textContent).toContain('创建代理')
    expect(document.body.querySelector('[role="dialog"]')).not.toBeNull()

    const closeButton = Array.from(document.body.querySelectorAll<HTMLButtonElement>('button'))
      .find((node) => node.textContent?.trim() === '关闭')
    closeButton?.click()
    await nextTick()

    expect(document.body.querySelector('[role="dialog"]')).toBeNull()
  })

  it('opens and closes the install-skill modal on the skill page', async () => {
    const { default: App } = await import('./App.vue')

    host = document.createElement('div')
    document.body.appendChild(host)

    app = createApp(App)
    app.mount(host)
    await nextTick()

    const skillNavButton = Array.from(host.querySelectorAll<HTMLButtonElement>('button'))
      .find((node) => node.textContent?.includes('技能'))
    skillNavButton?.click()
    await nextTick()

    const openButton = Array.from(host.querySelectorAll<HTMLButtonElement>('button'))
      .find((node) => node.textContent?.includes('安装技能'))
    openButton?.click()
    await nextTick()

    expect(document.body.textContent).toContain('安装技能')
    expect(document.body.querySelector('[role="dialog"]')).not.toBeNull()

    const closeButton = Array.from(document.body.querySelectorAll<HTMLButtonElement>('button'))
      .find((node) => node.textContent?.trim() === '关闭')
    closeButton?.click()
    await nextTick()

    expect(document.body.querySelector('[role="dialog"]')).toBeNull()
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
  })

  it('renders named footer items with status dots instead of visible status text', async () => {
    const { default: App } = await import('./App.vue')

    host = document.createElement('div')
    document.body.appendChild(host)

    app = createApp(App)
    app.mount(host)
    await nextTick()

    const statusBar = host.querySelector('.status-bar')
    const dots = host.querySelectorAll('.status-bar-item .status-indicator')

    expect(statusBar).not.toBeNull()
    expect(dots).toHaveLength(4)
    expect(statusBar?.textContent).not.toContain('未连接')
    expect(host.querySelector('.status-bar-item[data-tone="neutral"] .status-indicator')?.getAttribute('title')).toBe('未连接')
  })

  it('shows the latest log summary on the left and keeps status items grouped on the right', async () => {
    const { default: App } = await import('./App.vue')

    host = document.createElement('div')
    document.body.appendChild(host)

    app = createApp(App)
    app.mount(host)
    await nextTick()

    const statusBar = host.querySelector('.status-bar')
    const latestLog = host.querySelector('.status-bar-latest-log')
    const statuses = host.querySelector('.status-bar-statuses')

    expect(statusBar?.firstElementChild).toBe(latestLog)
    expect(statusBar?.lastElementChild).toBe(statuses)
    expect(latestLog?.textContent).toContain('最新日志')
    expect(latestLog?.textContent).toContain('SYSTEM')
    expect(latestLog?.textContent).toContain('ready')
    expect(statuses?.querySelectorAll('.status-bar-item')).toHaveLength(4)
    expect(statuses?.textContent).toContain('Token')
    expect(statuses?.textContent).toContain('Gateway')
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
