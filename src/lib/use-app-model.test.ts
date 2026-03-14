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
  saveProfile: vi.fn(),
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

  afterEach(() => {
    app?.unmount()
    host?.remove()
    host = null
    app = null
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

function createSnapshot(): EnvironmentSnapshot {
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
