import { describe, expect, it } from 'vitest'

import {
  buildInstallSummary,
  buildWorkflow,
  phaseLabel,
  shouldShowInstallConsole,
  startRuntimeDisabledReason,
  tokenStatusLabel,
} from './runtime-view'
import type { EnvironmentSnapshot, InstallResult } from '@/types'

describe('runtime view helpers', () => {
  it('builds a 3-step workflow with install, config and runtime', () => {
    const snapshot = createSnapshot({
      openclawInstalled: false,
      runtimeStatus: {
        phase: 'installNeeded',
        sshConnected: false,
        nodeConnected: false,
        lastError: null,
      },
    })

    const workflow = buildWorkflow(snapshot)

    expect(workflow.map((step) => step.key)).toEqual(['install', 'configure', 'runtime'])
    expect(workflow[0]?.state).toBe('active')
    expect(workflow[1]?.state).toBe('idle')
  })

  it('marks configuration complete only when profile and token are both saved', () => {
    const snapshot = createSnapshot({
      hasSavedProfile: true,
      tokenStatus: 'saved',
      openclawInstalled: true,
    })

    const workflow = buildWorkflow(snapshot)

    expect(workflow[1]?.state).toBe('complete')
    expect(workflow[1]?.caption).toContain('已保存')
  })

  it('hides skipped install results when openclaw is already present', () => {
    const installResult: InstallResult = {
      strategy: 'skipped',
      success: true,
      stdout: '',
      stderr: '',
      needsElevation: false,
      manualUrl: 'https://docs.openclaw.ai/install',
      followUp: '已检测到 openclaw，跳过安装。',
    }

    expect(shouldShowInstallConsole(installResult)).toBe(false)
  })

  it('uses token status labels and start disabled reasons', () => {
    const missingToken = createSnapshot({
      openclawInstalled: true,
      hasSavedProfile: true,
      tokenStatus: 'missing',
    })

    expect(tokenStatusLabel(missingToken)).toBe('Token 未保存')
    expect(startRuntimeDisabledReason(missingToken, null)).toBe('请先保存 Gateway Token。')

    const tokenError = createSnapshot({
      openclawInstalled: true,
      hasSavedProfile: true,
      tokenStatus: 'error',
      tokenStatusMessage: 'token file unavailable',
    })

    expect(tokenStatusLabel(tokenError)).toBe('Token 存储异常')
    expect(startRuntimeDisabledReason(tokenError, null)).toBe('token file unavailable')
  })

  it('returns install summary from detected openclaw version', () => {
    const snapshot = createSnapshot({
      openclawInstalled: true,
      openclawVersion: 'OpenClaw 2026.3.8',
    })

    const summary = buildInstallSummary(snapshot, null, null)

    expect(summary.title).toBe('OpenClaw 已安装')
    expect(summary.detail).toBe('OpenClaw 2026.3.8')
    expect(phaseLabel('running')).toBe('运行中')
  })
})

function createSnapshot(
  overrides: Partial<EnvironmentSnapshot> = {},
): EnvironmentSnapshot {
  return {
    os: 'macos',
    sshInstalled: true,
    openclawInstalled: true,
    openclawVersion: null,
    npmInstalled: true,
    pnpmInstalled: true,
    hasSavedProfile: false,
    tokenStatus: 'missing',
    tokenStatusMessage: null,
    savedSettings: null,
    runtimeStatus: {
      phase: 'checking',
      sshConnected: false,
      nodeConnected: false,
      lastError: null,
    },
    installRecommendation: 'test',
    ...overrides,
  }
}
