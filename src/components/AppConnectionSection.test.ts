import { createApp, nextTick, reactive, ref } from 'vue'
import { afterEach, describe, expect, it } from 'vitest'

import AppConnectionSection from './AppConnectionSection.vue'

describe('AppConnectionSection', () => {
  let host: HTMLDivElement | null = null
  let app: ReturnType<typeof createApp> | null = null

  afterEach(() => {
    app?.unmount()
    host?.remove()
    app = null
    host = null
  })

  it('keeps successful inline connection feedback compact after the modal closes', async () => {
    host = document.createElement('div')
    document.body.appendChild(host)

    app = createApp(AppConnectionSection, {
      state: createConnectionSectionState({
        connectionTestInlinePhase: 'success',
        connectionTestInlineSummary: 'Gateway 鉴权通过。',
        connectionTestInlineVisible: true,
        connectionTestInlineResult: {
          success: true,
          step: 'gatewayProbe',
          summary: 'Gateway 鉴权通过。',
          stdout: 'very large stdout payload',
          stderr: '',
        },
      }),
    })
    app.mount(host)
    await nextTick()

    expect(host.textContent).toContain('Gateway 鉴权通过。')
    expect(host.textContent).not.toContain('very large stdout payload')
  })

  it('still shows inline diagnostics for failed connection tests', async () => {
    host = document.createElement('div')
    document.body.appendChild(host)

    app = createApp(AppConnectionSection, {
      state: createConnectionSectionState({
        connectionTestInlinePhase: 'error',
        connectionTestInlineSummary: 'Gateway 鉴权失败。',
        connectionTestInlineVisible: true,
        connectionTestInlineResult: {
          success: false,
          step: 'gatewayProbe',
          summary: 'Gateway 鉴权失败。',
          stdout: '',
          stderr: 'gateway auth failed',
        },
      }),
    })
    app.mount(host)
    await nextTick()

    expect(host.textContent).toContain('Gateway 鉴权失败。')
    expect(host.textContent).toContain('gateway auth failed')
  })

  it('renders a dedicated diagnostics card when inline result is visible', async () => {
    host = document.createElement('div')
    document.body.appendChild(host)

    app = createApp(AppConnectionSection, {
      state: createConnectionSectionState({
        connectionTestInlinePhase: 'success',
        connectionTestInlineSummary: 'Gateway 鉴权通过。',
        connectionTestInlineVisible: true,
      }),
    })
    app.mount(host)
    await nextTick()

    expect(host.querySelector('.connection-diagnostics-card')).not.toBeNull()
  })
})

function createConnectionSectionState(overrides: Partial<{
  connectionTestInlinePhase: 'idle' | 'running' | 'success' | 'error'
  connectionTestInlineSummary: string
  connectionTestInlineVisible: boolean
  connectionTestInlineResult: {
    success: boolean
    step: 'save' | 'sshTunnel' | 'gatewayProbe'
    summary: string
    stdout: string
    stderr: string
  } | null
}> = {}) {
  return {
    advancedOpen: ref(false),
    canSaveProfile: ref(true),
    canTestConnection: ref(true),
    canStartHostedRuntime: ref(true),
    companyProfile: reactive({
      sshHost: 'gateway.example.com',
      sshUser: 'bizclaw',
      localPort: '18889',
      remoteBindHost: '127.0.0.1',
      remoteBindPort: '18789',
    }),
    userProfile: reactive({
      displayName: 'BizClaw',
      autoConnect: true,
      runInBackground: true,
    }),
    targetProfile: reactive({
      wslDistro: 'Ubuntu',
    }),
    tokenInput: ref('token'),
    sshPasswordInput: ref('password'),
    tokenStateToneValue: ref('success'),
    tokenStateLabel: ref('Token saved'),
    profileError: ref<string | null>(null),
    connectionTestDisabledReason: ref<string | null>(null),
    connectionTestInlinePhase: ref(overrides.connectionTestInlinePhase ?? 'idle'),
    connectionTestInlineSummary: ref(overrides.connectionTestInlineSummary ?? ''),
    connectionTestInlineResult: ref(overrides.connectionTestInlineResult ?? null),
    connectionTestInlineVisible: ref(overrides.connectionTestInlineVisible ?? false),
    saveOnly: () => undefined,
    saveAndTest: () => undefined,
    runtimeStatus: ref({
      phase: 'configured' as const,
      sshConnected: false,
      nodeConnected: false,
      gatewayConnected: false,
      lastError: null,
    }),
    platformLabel: ref('Windows Native'),
    runtimeError: ref<string | null>(null),
    runtimeStartBusy: ref(false),
    runtimeStopBusy: ref(false),
    connectDisabledReason: ref<string | null>(null),
    startHostedRuntime: () => undefined,
    stopHostedRuntime: () => undefined,
  }
}
