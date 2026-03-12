import { describe, expect, it } from 'vitest'

import { buildWorkflow, phaseLabel } from './runtime-view'
import type { EnvironmentSnapshot } from '@/types'

describe('runtime view helpers', () => {
  it('marks install as active when openclaw is missing', () => {
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

    expect(workflow[1]?.state).toBe('active')
    expect(workflow[2]?.state).toBe('idle')
  })

  it('marks runtime complete when node is running', () => {
    const snapshot = createSnapshot({
      openclawInstalled: true,
      hasSavedProfile: true,
      runtimeStatus: {
        phase: 'running',
        sshConnected: true,
        nodeConnected: true,
        lastError: null,
      },
    })

    const workflow = buildWorkflow(snapshot)

    expect(workflow[3]?.state).toBe('complete')
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
    npmInstalled: true,
    pnpmInstalled: true,
    hasSavedProfile: false,
    hasSavedToken: false,
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
