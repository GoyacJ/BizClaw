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
    apiMocks.onRuntimeLog.mockResolvedValue(() => {})
    apiMocks.onRuntimeStatus.mockResolvedValue(() => {})
    apiMocks.onOperationStatus.mockResolvedValue(() => {})
    apiMocks.onOperationEvent.mockResolvedValue(() => {})
    apiMocks.onConnectionTestEvent.mockResolvedValue(() => {})
    apiMocks.onRefreshRequested.mockResolvedValue(() => {})

    let model: ReturnType<typeof useAppModel> | null = null
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

    expect(model?.operationTask.value).toEqual(task)
    expect(model?.operationEvents.value).toEqual(events)
    expect(apiMocks.detectEnvironment).toHaveBeenCalledTimes(1)
    expect(apiMocks.streamLogs).toHaveBeenCalledTimes(1)
    expect(apiMocks.getOperationStatus).toHaveBeenCalledTimes(1)
    expect(apiMocks.getOperationEvents).toHaveBeenCalledTimes(1)
    expect(apiMocks.detectEnvironment.mock.invocationCallOrder[0]).toBeLessThan(apiMocks.streamLogs.mock.invocationCallOrder[0])
    expect(apiMocks.streamLogs.mock.invocationCallOrder[0]).toBeLessThan(apiMocks.getOperationStatus.mock.invocationCallOrder[0])
    expect(apiMocks.getOperationStatus.mock.invocationCallOrder[0]).toBeLessThan(apiMocks.getOperationEvents.mock.invocationCallOrder[0])
  })
})

async function flushPromises() {
  await Promise.resolve()
  await nextTick()
  await Promise.resolve()
  await nextTick()
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
