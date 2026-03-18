import { afterEach, describe, expect, it } from 'vitest'

import {
  buildOperationsSummary,
  latestOperationDetail,
  operationTaskPhaseLabel,
  operationStepLabel,
  runtimeDetail,
  runtimeTargetLabel,
  startRuntimeDisabledReason,
  tokenStatusLabel,
} from './runtime-view'
import { setAppLocale, translate } from './i18n'
import type { EnvironmentSnapshot, OperationResult, OperationTaskSnapshot } from '@/types'

describe('runtime view helpers', () => {
  afterEach(() => {
    setAppLocale('zh-CN')
  })

  it('renders the runtime target and operation step labels', () => {
    expect(runtimeTargetLabel('macNative')).toBe('macOS 本机')
    expect(runtimeTargetLabel('windowsNative')).toBe('Windows 本机')
    expect(runtimeTargetLabel('windowsWsl')).toBe('Windows WSL')
    expect(operationStepLabel('bootstrapWsl')).toBe('WSL 初始化')
    expect(operationTaskPhaseLabel('cancelling')).toBe('停止中')
  })

  it('renders helper labels in English when the locale changes', () => {
    setAppLocale('en-US')

    expect(runtimeTargetLabel('macNative')).toBe('macOS')
    expect(runtimeTargetLabel('windowsNative')).toBe('Windows Native')
    expect(operationStepLabel('bootstrapWsl')).toBe('Initialize WSL')
    expect(operationTaskPhaseLabel('cancelling')).toBe('Stopping')
    expect(tokenStatusLabel(createSnapshot({
      tokenStatus: 'saved',
    }))).toBe('Token saved')
  })

  it('marks operations summary as update-ready when a newer version exists', () => {
    const summary = buildOperationsSummary(createSnapshot({
      openclawInstalled: true,
      openclawVersion: 'OpenClaw 2026.3.8',
      latestOpenclawVersion: '2026.3.9',
      updateAvailable: true,
    }), null, null)

    expect(summary.title).toBe('检测到可用更新')
    expect(summary.detail).toContain('2026.3.9')
    expect(summary.tone).toBe('active')
  })

  it('prefers follow-up messaging when an operation failed', () => {
    const result: OperationResult = {
      kind: 'install',
      strategy: 'official',
      success: false,
      step: 'installOpenClaw',
      stdout: '',
      stderr: '',
      needsElevation: false,
      manualUrl: 'https://docs.openclaw.ai/install',
      followUp: '请完成 WSL / Ubuntu 初始化后重试。',
    }

    const task: OperationTaskSnapshot = {
      phase: 'error',
      kind: 'install',
      step: 'installOpenClaw',
      canStop: false,
      lastResult: result,
      startedAt: 1,
      endedAt: 2,
    }

    const summary = buildOperationsSummary(createSnapshot(), task, null)
    expect(summary.title).toBe('操作未完成')
    expect(summary.detail).toContain('WSL')
    expect(summary.tone).toBe('error')
  })

  it('describes a running install task as a background action', () => {
    const task: OperationTaskSnapshot = {
      phase: 'running',
      kind: 'install',
      step: 'installOpenClaw',
      canStop: true,
      lastResult: null,
      startedAt: 1,
      endedAt: null,
    }

    const summary = buildOperationsSummary(createSnapshot(), task, null)
    expect(summary.title).toBe('正在安装 OpenClaw')
    expect(summary.detail).toContain('后台')
    expect(summary.tone).toBe('active')
  })

  it('describes a cancelled task as stopped instead of failed', () => {
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
        stdout: '',
        stderr: '',
        needsElevation: false,
        manualUrl: 'https://docs.openclaw.ai/install',
        followUp: '安装已停止。',
      },
      startedAt: 1,
      endedAt: 2,
    }

    const summary = buildOperationsSummary(createSnapshot(), task, null)
    expect(summary.title).toBe('任务已停止')
    expect(summary.detail).toContain('已停止')
    expect(summary.tone).toBe('active')
  })

  it('blocks runtime start when windows wsl is not ready', () => {
    const snapshot = createSnapshot({
      runtimeTarget: 'windowsWsl',
      wslStatus: {
        available: true,
        distroName: 'Ubuntu',
        distroInstalled: false,
        ready: false,
        needsReboot: false,
        message: '尚未检测到 Ubuntu',
      },
      targetSshInstalled: false,
    })

    expect(tokenStatusLabel(snapshot)).toBe('Token 未保存')
    expect(startRuntimeDisabledReason(snapshot, false)).toBe('请先完成 WSL / Ubuntu 初始化。')
  })

  it('does not require wsl readiness when using windows native runtime', () => {
    const snapshot = createSnapshot({
      runtimeTarget: 'windowsNative',
      targetSshInstalled: false,
    })

    expect(startRuntimeDisabledReason(snapshot, false)).toBe('请先补齐目标运行环境中的 OpenSSH。')
  })

  it('does not block runtime start while environment detection is still pending', () => {
    expect(startRuntimeDisabledReason(null, false)).toBeNull()
  })

  it('falls back when token status messaging is mojibake', () => {
    const snapshot = createSnapshot({
      openclawInstalled: true,
      hasSavedProfile: true,
      tokenStatus: 'error',
      tokenStatusMessage: '鍒濆鍖栧け璐ワ紝璇风◢鍚庨噸璇曘€',
    })

    expect(startRuntimeDisabledReason(snapshot, false)).toBe(translate('runtime.startDisabled.tokenError'))
  })

  it('falls back when runtime errors are mojibake', () => {
    expect(runtimeDetail({
      phase: 'error',
      sshConnected: false,
      nodeConnected: false,
      gatewayConnected: false,
      lastError: '鍒濆鍖栧け璐ワ紝璇风◢鍚庨噸璇曘€',
    })).toBe(translate('runtime.detail.idle'))
  })

  it('falls back when the latest operation event is mojibake', () => {
    expect(latestOperationDetail([
      {
        kind: 'install',
        step: 'installOpenClaw',
        status: 'log',
        source: 'stdout',
        message: '鍒濆鍖栧け璐ワ紝璇风◢鍚庨噸璇曘€',
        timestampMs: Date.parse('2026-03-18T10:00:00Z'),
      },
    ])).toBe(translate('runtime.operationsSummary.latestOperationEmpty'))
  })
})

function createSnapshot(
  overrides: Partial<EnvironmentSnapshot> = {},
): EnvironmentSnapshot {
  return {
    os: 'macos',
    runtimeTarget: 'macNative',
    hostSshInstalled: true,
    hostOpenclawInstalled: false,
    targetSshInstalled: true,
    openclawInstalled: false,
    openclawVersion: null,
    latestOpenclawVersion: null,
    updateAvailable: false,
    wslOpenclawInstalled: false,
    hasSavedProfile: false,
    tokenStatus: 'missing',
    tokenStatusMessage: null,
    uiPreferences: {
      theme: 'light',
      locale: 'zh-CN',
      sidebarCollapsed: false,
    },
    savedSettings: null,
    runtimeStatus: {
      phase: 'checking',
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
