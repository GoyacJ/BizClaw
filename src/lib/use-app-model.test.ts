import { createApp, defineComponent, h, nextTick } from 'vue'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { useAppModel } from './use-app-model'
import type { EnvironmentSnapshot, OperationEvent, OperationTaskSnapshot } from '@/types'

const apiMocks = vi.hoisted(() => ({
  detectEnvironment: vi.fn(),
  streamLogs: vi.fn(),
  getOperationStatus: vi.fn(),
  getOperationEvents: vi.fn(),
  onRuntimeLog: vi.fn(),
  onRuntimeStatus: vi.fn(),
  onOperationStatus: vi.fn(),
  onOperationEvent: vi.fn(),
  onConnectionTestEvent: vi.fn(),
  onRefreshRequested: vi.fn(),
  saveProfile: vi.fn(),
  saveUiPreferences: vi.fn(),
}))

const bizclawUpdaterMocks = vi.hoisted(() => ({
  checkForBizClawUpdate: vi.fn(),
  describeBizClawUpdaterError: vi.fn((error: unknown) => (
    error instanceof Error ? error.message : String(error)
  )),
  getCurrentBizClawVersion: vi.fn(),
  relaunchBizClaw: vi.fn(),
}))

vi.mock('@/lib/api', () => ({
  checkOpenClawUpdate: vi.fn(),
  detectEnvironment: apiMocks.detectEnvironment,
  getOperationEvents: apiMocks.getOperationEvents,
  getOperationStatus: apiMocks.getOperationStatus,
  installOpenClaw: vi.fn(),
  onConnectionTestEvent: apiMocks.onConnectionTestEvent,
  onOperationEvent: apiMocks.onOperationEvent,
  onOperationStatus: apiMocks.onOperationStatus,
  onRefreshRequested: apiMocks.onRefreshRequested,
  onRuntimeLog: apiMocks.onRuntimeLog,
  onRuntimeStatus: apiMocks.onRuntimeStatus,
  openManualInstall: vi.fn(),
  saveProfile: apiMocks.saveProfile,
  saveUiPreferences: apiMocks.saveUiPreferences,
  startRuntime: vi.fn(),
  stopOpenClawOperation: vi.fn(),
  stopRuntime: vi.fn(),
  streamLogs: apiMocks.streamLogs,
  testConnection: vi.fn(),
  updateOpenClaw: vi.fn(),
}))

vi.mock('@/lib/bizclaw-updater', () => ({
  checkForBizClawUpdate: bizclawUpdaterMocks.checkForBizClawUpdate,
  describeBizClawUpdaterError: bizclawUpdaterMocks.describeBizClawUpdaterError,
  getCurrentBizClawVersion: bizclawUpdaterMocks.getCurrentBizClawVersion,
  relaunchBizClaw: bizclawUpdaterMocks.relaunchBizClaw,
}))

describe('useAppModel', () => {
  let host: HTMLDivElement | null = null
  let app: ReturnType<typeof createApp> | null = null
  const originalMatchMedia = window.matchMedia

  afterEach(() => {
    app?.unmount()
    host?.remove()
    host = null
    app = null
    document.documentElement.dataset.theme = ''
    document.documentElement.style.colorScheme = ''
    window.matchMedia = originalMatchMedia
    vi.clearAllMocks()
  })

  it('hydrates operation status and recent events on mount in order', async () => {
    const snapshot = createSnapshot()
    const task: OperationTaskSnapshot = {
      phase: 'cancelled',
      kind: 'install',
      step: 'installOpenClaw',
      canStop: false,
      lastResult: {
        kind: 'install',
        strategy: 'official',
        success: false,
        step: 'installOpenClaw',
        stdout: 'partial output',
        stderr: '',
        needsElevation: false,
        manualUrl: 'https://docs.openclaw.ai/install',
        followUp: '安装已停止。',
      },
      startedAt: 1,
      endedAt: 2,
    }
    const events: OperationEvent[] = [
      {
        kind: 'install',
        step: 'installOpenClaw',
        status: 'cancelled',
        source: 'system',
        message: 'official 已停止',
        timestampMs: 2,
      },
    ]

    apiMocks.detectEnvironment.mockResolvedValue(snapshot)
    apiMocks.streamLogs.mockResolvedValue([])
    apiMocks.getOperationStatus.mockResolvedValue(task)
    apiMocks.getOperationEvents.mockResolvedValue(events)
    bizclawUpdaterMocks.getCurrentBizClawVersion.mockResolvedValue('0.1.8')
    apiMocks.onRuntimeLog.mockResolvedValue(() => {})
    apiMocks.onRuntimeStatus.mockResolvedValue(() => {})
    apiMocks.onOperationStatus.mockResolvedValue(() => {})
    apiMocks.onOperationEvent.mockResolvedValue(() => {})
    apiMocks.onConnectionTestEvent.mockResolvedValue(() => {})
    apiMocks.onRefreshRequested.mockResolvedValue(() => {})

    let model!: ReturnType<typeof useAppModel>
    const TestHarness = defineComponent({
      setup() {
        model = useAppModel()
        return () => h('div')
      },
    })

    host = document.createElement('div')
    document.body.appendChild(host)

    app = createApp(TestHarness)
    app.mount(host)

    await flushPromises()

    expect(model.operationTask.value).toEqual(task)
    expect(model.operationEvents.value).toEqual(events)
    expect(apiMocks.detectEnvironment).toHaveBeenCalledTimes(1)
    expect(apiMocks.streamLogs).toHaveBeenCalledTimes(1)
    expect(apiMocks.getOperationStatus).toHaveBeenCalledTimes(1)
    expect(apiMocks.getOperationEvents).toHaveBeenCalledTimes(1)
    expect(apiMocks.detectEnvironment.mock.invocationCallOrder[0]).toBeLessThan(apiMocks.streamLogs.mock.invocationCallOrder[0])
    expect(apiMocks.streamLogs.mock.invocationCallOrder[0]).toBeLessThan(apiMocks.getOperationStatus.mock.invocationCallOrder[0])
    expect(apiMocks.getOperationStatus.mock.invocationCallOrder[0]).toBeLessThan(apiMocks.getOperationEvents.mock.invocationCallOrder[0])
  })

  it('loads the current BizClaw version on mount without checking remote updates', async () => {
    apiMocks.detectEnvironment.mockResolvedValue(createSnapshot())
    apiMocks.streamLogs.mockResolvedValue([])
    apiMocks.getOperationStatus.mockResolvedValue(createIdleTask())
    apiMocks.getOperationEvents.mockResolvedValue([])
    bizclawUpdaterMocks.getCurrentBizClawVersion.mockResolvedValue('0.1.8')
    bizclawUpdaterMocks.checkForBizClawUpdate.mockResolvedValue(null)
    apiMocks.onRuntimeLog.mockResolvedValue(() => {})
    apiMocks.onRuntimeStatus.mockResolvedValue(() => {})
    apiMocks.onOperationStatus.mockResolvedValue(() => {})
    apiMocks.onOperationEvent.mockResolvedValue(() => {})
    apiMocks.onConnectionTestEvent.mockResolvedValue(() => {})
    apiMocks.onRefreshRequested.mockResolvedValue(() => {})

    let model!: ReturnType<typeof useAppModel>
    const TestHarness = defineComponent({
      setup() {
        model = useAppModel()
        return () => h('div')
      },
    })

    host = document.createElement('div')
    document.body.appendChild(host)
    app = createApp(TestHarness)
    app.mount(host)

    await flushPromises()

    expect(bizclawUpdaterMocks.getCurrentBizClawVersion).toHaveBeenCalledTimes(1)
    expect(bizclawUpdaterMocks.checkForBizClawUpdate).not.toHaveBeenCalled()
    expect(model.bizclawUpdate.value.phase).toBe('idle')
    expect(model.bizclawUpdate.value.currentVersion).toBe('0.1.8')
    expect(model.bizclawUpdateActionLabel.value).toBe('已加载当前版本')
  })

  it('hydrates ui preferences from the environment and persists locale changes', async () => {
    apiMocks.detectEnvironment
      .mockResolvedValueOnce(createSnapshot({
        uiPreferences: {
          theme: 'dark',
          locale: 'en-US',
        },
      }))
      .mockResolvedValue(createSnapshot({
        uiPreferences: {
          theme: 'dark',
          locale: 'zh-CN',
        },
      }))
    apiMocks.streamLogs.mockResolvedValue([])
    apiMocks.getOperationStatus.mockResolvedValue(createIdleTask())
    apiMocks.getOperationEvents.mockResolvedValue([])
    apiMocks.saveUiPreferences.mockResolvedValue({
      theme: 'dark',
      locale: 'zh-CN',
    })
    bizclawUpdaterMocks.getCurrentBizClawVersion.mockResolvedValue('0.1.8')
    apiMocks.onRuntimeLog.mockResolvedValue(() => {})
    apiMocks.onRuntimeStatus.mockResolvedValue(() => {})
    apiMocks.onOperationStatus.mockResolvedValue(() => {})
    apiMocks.onOperationEvent.mockResolvedValue(() => {})
    apiMocks.onConnectionTestEvent.mockResolvedValue(() => {})
    apiMocks.onRefreshRequested.mockResolvedValue(() => {})

    let model!: ReturnType<typeof useAppModel>
    const TestHarness = defineComponent({
      setup() {
        model = useAppModel()
        return () => h('div')
      },
    })

    host = document.createElement('div')
    document.body.appendChild(host)
    app = createApp(TestHarness)
    app.mount(host)

    await flushPromises()

    expect(model.uiPreferences.value).toEqual({
      theme: 'dark',
      locale: 'en-US',
    })
    expect(model.tokenStateLabel.value).toBe('Token saved')

    await model.setLocale('zh-CN')
    await flushPromises()

    expect(apiMocks.saveUiPreferences).toHaveBeenCalledWith({
      theme: 'dark',
      locale: 'zh-CN',
    })
    expect(model.uiPreferences.value.locale).toBe('zh-CN')
    expect(model.tokenStateLabel.value).toBe('Token 已保存')
  })

  it('applies and tracks the system theme when the preference is set to system', async () => {
    const mediaQuery = createMatchMediaMock(true)
    window.matchMedia = vi.fn().mockImplementation((query: string) => {
      expect(query).toBe('(prefers-color-scheme: dark)')
      return mediaQuery
    })

    apiMocks.detectEnvironment.mockResolvedValue(createSnapshot({
      uiPreferences: {
        theme: 'system',
        locale: 'zh-CN',
      },
    }))
    apiMocks.streamLogs.mockResolvedValue([])
    apiMocks.getOperationStatus.mockResolvedValue(createIdleTask())
    apiMocks.getOperationEvents.mockResolvedValue([])
    apiMocks.saveUiPreferences.mockImplementation(async (preferences) => preferences)
    bizclawUpdaterMocks.getCurrentBizClawVersion.mockResolvedValue('0.1.8')
    apiMocks.onRuntimeLog.mockResolvedValue(() => {})
    apiMocks.onRuntimeStatus.mockResolvedValue(() => {})
    apiMocks.onOperationStatus.mockResolvedValue(() => {})
    apiMocks.onOperationEvent.mockResolvedValue(() => {})
    apiMocks.onConnectionTestEvent.mockResolvedValue(() => {})
    apiMocks.onRefreshRequested.mockResolvedValue(() => {})

    let model!: ReturnType<typeof useAppModel>
    const TestHarness = defineComponent({
      setup() {
        model = useAppModel()
        return () => h('div')
      },
    })

    host = document.createElement('div')
    document.body.appendChild(host)
    app = createApp(TestHarness)
    app.mount(host)

    await flushPromises()

    expect(model.uiPreferences.value.theme).toBe('system')
    expect(document.documentElement.dataset.theme).toBe('dark')
    expect(document.documentElement.style.colorScheme).toBe('dark')

    mediaQuery.setMatches(false)
    await flushPromises()

    expect(document.documentElement.dataset.theme).toBe('light')
    expect(document.documentElement.style.colorScheme).toBe('light')

    await model.setTheme('light')
    await flushPromises()

    mediaQuery.setMatches(true)
    await flushPromises()

    expect(apiMocks.saveUiPreferences).toHaveBeenCalledWith({
      theme: 'light',
      locale: 'zh-CN',
    })
    expect(document.documentElement.dataset.theme).toBe('light')
  })

  it('keeps english locale after saving and refreshing localized state', async () => {
    apiMocks.detectEnvironment
      .mockResolvedValueOnce(createSnapshot({
        uiPreferences: {
          theme: 'light',
          locale: 'zh-CN',
        },
      }))
      .mockResolvedValue(createSnapshot({
        uiPreferences: {
          theme: 'light',
          locale: 'en-US',
        },
      }))
    apiMocks.streamLogs.mockResolvedValue([])
    apiMocks.getOperationStatus.mockResolvedValue(createIdleTask())
    apiMocks.getOperationEvents.mockResolvedValue([])
    apiMocks.saveUiPreferences.mockResolvedValue({
      theme: 'light',
      locale: 'en-US',
    })
    bizclawUpdaterMocks.getCurrentBizClawVersion.mockResolvedValue('0.1.8')
    apiMocks.onRuntimeLog.mockResolvedValue(() => {})
    apiMocks.onRuntimeStatus.mockResolvedValue(() => {})
    apiMocks.onOperationStatus.mockResolvedValue(() => {})
    apiMocks.onOperationEvent.mockResolvedValue(() => {})
    apiMocks.onConnectionTestEvent.mockResolvedValue(() => {})
    apiMocks.onRefreshRequested.mockResolvedValue(() => {})

    let model!: ReturnType<typeof useAppModel>
    const TestHarness = defineComponent({
      setup() {
        model = useAppModel()
        return () => h('div')
      },
    })

    host = document.createElement('div')
    document.body.appendChild(host)
    app = createApp(TestHarness)
    app.mount(host)

    await flushPromises()
    await model.setLocale('en-US')
    await flushPromises()

    expect(model.uiPreferences.value.locale).toBe('en-US')
    expect(model.tokenStateLabel.value).toBe('Token saved')
    expect(apiMocks.detectEnvironment).toHaveBeenCalledTimes(2)
    expect(apiMocks.streamLogs).toHaveBeenCalledTimes(1)
    expect(apiMocks.getOperationStatus).toHaveBeenCalledTimes(1)
    expect(apiMocks.getOperationEvents).toHaveBeenCalledTimes(1)
  })

  it('refreshes saved environment without reloading logs or operation history after save only', async () => {
    apiMocks.detectEnvironment
      .mockResolvedValueOnce(createSnapshot({
        savedSettings: null,
      }))
      .mockResolvedValue(createSnapshot({
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
      }))
    apiMocks.streamLogs.mockResolvedValue([])
    apiMocks.getOperationStatus.mockResolvedValue(createIdleTask())
    apiMocks.getOperationEvents.mockResolvedValue([])
    apiMocks.saveProfile.mockResolvedValue({
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
    })
    bizclawUpdaterMocks.getCurrentBizClawVersion.mockResolvedValue('0.1.8')
    apiMocks.onRuntimeLog.mockResolvedValue(() => {})
    apiMocks.onRuntimeStatus.mockResolvedValue(() => {})
    apiMocks.onOperationStatus.mockResolvedValue(() => {})
    apiMocks.onOperationEvent.mockResolvedValue(() => {})
    apiMocks.onConnectionTestEvent.mockResolvedValue(() => {})
    apiMocks.onRefreshRequested.mockResolvedValue(() => {})

    let model!: ReturnType<typeof useAppModel>
    const TestHarness = defineComponent({
      setup() {
        model = useAppModel()
        return () => h('div')
      },
    })

    host = document.createElement('div')
    document.body.appendChild(host)
    app = createApp(TestHarness)
    app.mount(host)

    await flushPromises()
    await model.saveOnly()
    await flushPromises()

    expect(apiMocks.detectEnvironment).toHaveBeenCalledTimes(2)
    expect(apiMocks.streamLogs).toHaveBeenCalledTimes(1)
    expect(apiMocks.getOperationStatus).toHaveBeenCalledTimes(1)
    expect(apiMocks.getOperationEvents).toHaveBeenCalledTimes(1)
  })

  it('checks for BizClaw updates only when requested and exposes the available release', async () => {
    apiMocks.detectEnvironment.mockResolvedValue(createSnapshot())
    apiMocks.streamLogs.mockResolvedValue([])
    apiMocks.getOperationStatus.mockResolvedValue(createIdleTask())
    apiMocks.getOperationEvents.mockResolvedValue([])
    bizclawUpdaterMocks.getCurrentBizClawVersion.mockResolvedValue('0.1.8')
    bizclawUpdaterMocks.checkForBizClawUpdate.mockResolvedValue({
      version: '0.1.9',
      body: 'Bug fixes',
      date: '2026-03-14T00:00:00.000Z',
      downloadAndInstall: vi.fn(),
    })
    apiMocks.onRuntimeLog.mockResolvedValue(() => {})
    apiMocks.onRuntimeStatus.mockResolvedValue(() => {})
    apiMocks.onOperationStatus.mockResolvedValue(() => {})
    apiMocks.onOperationEvent.mockResolvedValue(() => {})
    apiMocks.onConnectionTestEvent.mockResolvedValue(() => {})
    apiMocks.onRefreshRequested.mockResolvedValue(() => {})

    let model!: ReturnType<typeof useAppModel>
    const TestHarness = defineComponent({
      setup() {
        model = useAppModel()
        return () => h('div')
      },
    })

    host = document.createElement('div')
    document.body.appendChild(host)
    app = createApp(TestHarness)
    app.mount(host)

    await flushPromises()
    await model.checkBizClawUpdates()
    await flushPromises()

    expect(bizclawUpdaterMocks.checkForBizClawUpdate).toHaveBeenCalledTimes(1)
    expect(model.bizclawUpdate.value.phase).toBe('available')
    expect(model.bizclawUpdate.value.currentVersion).toBe('0.1.8')
    expect(model.bizclawUpdate.value.latestVersion).toBe('0.1.9')
    expect(model.bizclawUpdate.value.releaseNotes).toBe('Bug fixes')
  })

  it('keeps the current section when checking BizClaw updates from settings', async () => {
    apiMocks.detectEnvironment.mockResolvedValue(createSnapshot())
    apiMocks.streamLogs.mockResolvedValue([])
    apiMocks.getOperationStatus.mockResolvedValue(createIdleTask())
    apiMocks.getOperationEvents.mockResolvedValue([])
    bizclawUpdaterMocks.getCurrentBizClawVersion.mockResolvedValue('0.1.8')
    bizclawUpdaterMocks.checkForBizClawUpdate.mockResolvedValue(null)
    apiMocks.onRuntimeLog.mockResolvedValue(() => {})
    apiMocks.onRuntimeStatus.mockResolvedValue(() => {})
    apiMocks.onOperationStatus.mockResolvedValue(() => {})
    apiMocks.onOperationEvent.mockResolvedValue(() => {})
    apiMocks.onConnectionTestEvent.mockResolvedValue(() => {})
    apiMocks.onRefreshRequested.mockResolvedValue(() => {})

    let model!: ReturnType<typeof useAppModel>
    const TestHarness = defineComponent({
      setup() {
        model = useAppModel()
        return () => h('div')
      },
    })

    host = document.createElement('div')
    document.body.appendChild(host)
    app = createApp(TestHarness)
    app.mount(host)

    await flushPromises()
    model.activeSection.value = 'settings'

    await model.checkBizClawUpdates()
    await flushPromises()

    expect(model.activeSection.value).toBe('settings')
  })

  it('shows an actionable BizClaw updater error when the release manifest is missing', async () => {
    apiMocks.detectEnvironment.mockResolvedValue(createSnapshot())
    apiMocks.streamLogs.mockResolvedValue([])
    apiMocks.getOperationStatus.mockResolvedValue(createIdleTask())
    apiMocks.getOperationEvents.mockResolvedValue([])
    bizclawUpdaterMocks.getCurrentBizClawVersion.mockResolvedValue('0.1.8')
    bizclawUpdaterMocks.describeBizClawUpdaterError.mockReturnValueOnce(
      '无法读取 GitHub Release 的更新清单，请确认最新 Release 已上传 latest.json、签名文件和对应安装包。',
    )
    bizclawUpdaterMocks.checkForBizClawUpdate.mockRejectedValue(
      new Error('Could not fetch a valid release JSON from the remote'),
    )
    apiMocks.onRuntimeLog.mockResolvedValue(() => {})
    apiMocks.onRuntimeStatus.mockResolvedValue(() => {})
    apiMocks.onOperationStatus.mockResolvedValue(() => {})
    apiMocks.onOperationEvent.mockResolvedValue(() => {})
    apiMocks.onConnectionTestEvent.mockResolvedValue(() => {})
    apiMocks.onRefreshRequested.mockResolvedValue(() => {})

    let model!: ReturnType<typeof useAppModel>
    const TestHarness = defineComponent({
      setup() {
        model = useAppModel()
        return () => h('div')
      },
    })

    host = document.createElement('div')
    document.body.appendChild(host)
    app = createApp(TestHarness)
    app.mount(host)

    await flushPromises()
    await model.checkBizClawUpdates()
    await flushPromises()

    expect(model.bizclawUpdate.value.phase).toBe('error')
    expect(model.bizclawUpdate.value.errorMessage).toContain('latest.json')
    expect(model.bizclawUpdate.value.errorMessage).toContain('GitHub Release')
  })

  it('installs an available BizClaw update and keeps the restart prompt visible when deferred', async () => {
    const downloadAndInstall = vi.fn(async (handler?: (event: { event: string, data?: { contentLength?: number, chunkLength?: number } }) => void) => {
      handler?.({ event: 'Started', data: { contentLength: 1024 } })
      handler?.({ event: 'Progress', data: { chunkLength: 256 } })
      handler?.({ event: 'Finished' })
    })

    apiMocks.detectEnvironment.mockResolvedValue(createSnapshot())
    apiMocks.streamLogs.mockResolvedValue([])
    apiMocks.getOperationStatus.mockResolvedValue(createIdleTask())
    apiMocks.getOperationEvents.mockResolvedValue([])
    bizclawUpdaterMocks.getCurrentBizClawVersion.mockResolvedValue('0.1.8')
    bizclawUpdaterMocks.checkForBizClawUpdate.mockResolvedValue({
      version: '0.1.9',
      body: 'Bug fixes',
      date: '2026-03-14T00:00:00.000Z',
      downloadAndInstall,
    })
    apiMocks.onRuntimeLog.mockResolvedValue(() => {})
    apiMocks.onRuntimeStatus.mockResolvedValue(() => {})
    apiMocks.onOperationStatus.mockResolvedValue(() => {})
    apiMocks.onOperationEvent.mockResolvedValue(() => {})
    apiMocks.onConnectionTestEvent.mockResolvedValue(() => {})
    apiMocks.onRefreshRequested.mockResolvedValue(() => {})

    let model!: ReturnType<typeof useAppModel>
    const TestHarness = defineComponent({
      setup() {
        model = useAppModel()
        return () => h('div')
      },
    })

    host = document.createElement('div')
    document.body.appendChild(host)
    app = createApp(TestHarness)
    app.mount(host)

    await flushPromises()
    await model.checkBizClawUpdates()
    await flushPromises()
    await model.installBizClawUpdate()
    await flushPromises()
    model.deferBizClawRestart()

    expect(downloadAndInstall).toHaveBeenCalledTimes(1)
    expect(model.bizclawUpdate.value.phase).toBe('readyToRestart')
    expect(model.bizclawUpdate.value.downloadedBytes).toBe(256)
    expect(model.bizclawUpdate.value.totalBytes).toBe(1024)
    expect(model.bizclawUpdatePrimaryAction.value).toBe('立即重启')
  })
})

async function flushPromises() {
  for (let index = 0; index < 6; index += 1) {
    await Promise.resolve()
    await nextTick()
  }
}

function createSnapshot(overrides: Partial<EnvironmentSnapshot> = {}): EnvironmentSnapshot {
  return {
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
    uiPreferences: {
      theme: 'light',
      locale: 'zh-CN',
    },
    savedSettings: null,
    runtimeStatus: {
      phase: 'configured',
      sshConnected: false,
      nodeConnected: false,
      gatewayConnected: false,
      lastError: null,
    },
    installRecommendation: 'test',
    wslStatus: null,
    ...overrides,
  }
}

function createIdleTask(): OperationTaskSnapshot {
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

function createMatchMediaMock(initialMatches: boolean) {
  let matches = initialMatches
  type MatchMediaListener = EventListenerOrEventListenerObject | ((event: MediaQueryListEvent) => void)
  const listeners = new Set<MatchMediaListener>()

  const notifyListener = (listener: MatchMediaListener, event: MediaQueryListEvent) => {
    if (typeof listener === 'function') {
      listener(event)
      return
    }

    listener.handleEvent(event)
  }

  return {
    get matches() {
      return matches
    },
    media: '(prefers-color-scheme: dark)',
    onchange: null,
    addEventListener: vi.fn((_type: string, listener: EventListenerOrEventListenerObject) => {
      listeners.add(listener)
    }),
    removeEventListener: vi.fn((_type: string, listener: EventListenerOrEventListenerObject) => {
      listeners.delete(listener)
    }),
    addListener: vi.fn((listener: ((event: MediaQueryListEvent) => void) | null) => {
      if (listener) {
        listeners.add(listener)
      }
    }),
    removeListener: vi.fn((listener: ((event: MediaQueryListEvent) => void) | null) => {
      if (listener) {
        listeners.delete(listener)
      }
    }),
    dispatchEvent: vi.fn(() => true),
    setMatches(nextMatches: boolean) {
      matches = nextMatches
      const event = { matches, media: '(prefers-color-scheme: dark)' } as MediaQueryListEvent
      for (const listener of listeners) {
        notifyListener(listener, event)
      }
    },
  } satisfies MediaQueryList & { setMatches: (nextMatches: boolean) => void }
}
