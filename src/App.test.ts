import { createApp, nextTick, reactive, ref } from 'vue'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { EnvironmentSnapshot } from '@/types'

vi.mock('@/lib/runtime-view', () => ({
  phaseLabel: (phase: string) => {
    if (phase === 'running') {
      return '运行中'
    }

    if (phase === 'connecting') {
      return '连接中'
    }

    return phase
  },
}))

vi.mock('@/lib/use-app-model', () => {
  const environment = ref<EnvironmentSnapshot | null>({
    os: 'macOS',
    sshInstalled: true,
    openclawInstalled: true,
    openclawVersion: 'OpenClaw 2026.3.8',
    npmInstalled: true,
    pnpmInstalled: true,
    hasSavedProfile: true,
    tokenStatus: 'saved',
    tokenStatusMessage: null,
    savedSettings: null,
    runtimeStatus: {
      phase: 'running',
      sshConnected: true,
      nodeConnected: true,
      lastError: null,
    },
    installRecommendation: '已完成',
  })

  return {
    useAppModel: () => ({
      advancedOpen: ref(true),
      busyAction: ref<string | null>(null),
      canSaveAndConnect: ref(true),
      canSaveProfile: ref(true),
      canStartHostedRuntime: ref(true),
      companyProfile: reactive({
        sshHost: '127.0.0.1',
        sshUser: 'root',
        localPort: '18889',
        remoteBindHost: '127.0.0.1',
        remoteBindPort: '18789',
      }),
      connectDisabledReason: ref(''),
      environment,
      installCli: vi.fn(),
      installResult: ref(null),
      installSummary: ref({
        tone: 'success',
        title: '已安装',
        detail: 'OpenClaw CLI 已就绪',
      }),
      launchManualInstall: vi.fn(),
      logs: ref([
        {
          source: 'system',
          level: 'info',
          message: 'ready',
          timestampMs: Date.now(),
        },
      ]),
      profileError: ref<string | null>(null),
      refreshEnvironment: vi.fn(),
      runtimeError: ref<string | null>(null),
      runtimeStatus: ref({
        phase: 'running',
        sshConnected: true,
        nodeConnected: true,
        lastError: null,
      }),
      saveAndConnect: vi.fn(),
      saveOnly: vi.fn(),
      showInstallConsole: ref(false),
      sshPasswordInput: ref(''),
      startHostedRuntime: vi.fn(),
      stopHostedRuntime: vi.fn(),
      tokenInput: ref('saved-token'),
      tokenStateLabel: ref('已保存'),
      tokenStateToneValue: ref('success'),
      userProfile: reactive({
        displayName: 'Goya Mac',
        autoConnect: true,
        runInBackground: true,
      }),
      workflow: ref([
        {
          key: 'install',
          state: 'complete',
          title: '安装 OpenClaw',
          caption: 'CLI 已就绪',
        },
        {
          key: 'config',
          state: 'complete',
          title: '保存连接配置',
          caption: '配置已保存',
        },
        {
          key: 'runtime',
          state: 'active',
          title: '托管运行时',
          caption: '正在运行',
        },
      ]),
    }),
  }
})

describe('App environment status bar', () => {
  let host: HTMLDivElement | null = null
  let app: ReturnType<typeof createApp> | null = null

  afterEach(() => {
    app?.unmount()
    host?.remove()
    host = null
    app = null
  })

  it('renders the environment area as a compact status bar', async () => {
    const { default: App } = await import('./App.vue')

    host = document.createElement('div')
    document.body.appendChild(host)

    app = createApp(App)
    app.mount(host)
    await nextTick()

    const footer = host.querySelector('footer.environment-bar')
    expect(footer).not.toBeNull()
    expect(footer?.classList.contains('surface-card')).toBe(false)
    expect(footer?.getAttribute('role')).toBe('status')
    expect(footer?.getAttribute('aria-label')).toBe('当前环境状态')

    const list = footer?.querySelector('.environment-bar__list')
    expect(list?.tagName).toBe('UL')
    expect(list?.querySelectorAll('.environment-pill')).toHaveLength(5)
    expect(list?.textContent).toContain('托管')
    expect(list?.textContent).toContain('运行中')

    const details = Array.from(
      list?.querySelectorAll<HTMLElement>('.environment-pill__detail') ?? [],
    )
    expect(details).toHaveLength(5)
    for (const detail of details) {
      expect(detail.getAttribute('title')).toBe(detail.textContent)
    }
  })

  it('shows an SSH password field in advanced settings', async () => {
    const { default: App } = await import('./App.vue')

    host = document.createElement('div')
    document.body.appendChild(host)

    app = createApp(App)
    app.mount(host)
    await nextTick()

    const fields = Array.from(
      host.querySelectorAll<HTMLElement>('.advanced-grid .field span'),
    ).map((node) => node.textContent?.trim())

    expect(fields).toContain('SSH Password')
    expect(host.querySelectorAll('input[placeholder="如已保存，可留空保持原值"]')).toHaveLength(2)
  })
})
