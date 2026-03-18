import { afterEach, describe, expect, it, vi } from 'vitest'

import type { UiPreferences } from '@/types'

afterEach(() => {
  vi.resetModules()
  vi.unmock('@tauri-apps/api/core')
  vi.unmock('@tauri-apps/api/event')
  window.localStorage.clear()
})

describe('tauri api fallback', () => {
  it('returns browser-safe defaults when tauri invoke is unavailable', async () => {
    vi.doMock('@tauri-apps/api/core', () => ({
      invoke: undefined,
    }))
    vi.doMock('@tauri-apps/api/event', () => ({
      listen: undefined,
    }))

    const api = await import('./api')
    const preferences: UiPreferences = {
      theme: 'system',
      locale: 'en-US',
      sidebarCollapsed: true,
    }

    await expect(api.detectEnvironment()).resolves.toEqual({
      os: 'browser',
      runtimeTarget: 'macNative',
      hostSshInstalled: false,
      hostOpenclawInstalled: false,
      targetSshInstalled: false,
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
      installRecommendation: '',
      wslStatus: null,
    })
    await expect(api.getOperationStatus()).resolves.toEqual({
      phase: 'idle',
      kind: null,
      step: null,
      canStop: false,
      lastResult: null,
      startedAt: null,
      endedAt: null,
    })
    await expect(api.listOpenClawAgents()).rejects.toThrow('BizClaw desktop APIs are unavailable in the browser preview.')
    await expect(api.listOpenClawSkills()).rejects.toThrow('BizClaw desktop APIs are unavailable in the browser preview.')
    await expect(api.getOperationEvents()).resolves.toEqual([])
    await expect(api.streamLogs()).resolves.toEqual([])
    await expect(api.saveUiPreferences(preferences)).resolves.toEqual(preferences)
    await expect(api.detectEnvironment()).resolves.toMatchObject({
      uiPreferences: preferences,
    })
    await expect(api.onRefreshRequested(() => {})).resolves.toEqual(expect.any(Function))
  })

  it('falls back even when invoke is defined but tauri internals are missing', async () => {
    const invoke = vi.fn()
    const listen = vi.fn()
    vi.doMock('@tauri-apps/api/core', () => ({
      invoke,
    }))
    vi.doMock('@tauri-apps/api/event', () => ({
      listen,
    }))
    delete (globalThis as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__

    const api = await import('./api')

    await expect(api.detectEnvironment()).resolves.toMatchObject({
      os: 'browser',
      uiPreferences: {
        theme: 'light',
        locale: 'zh-CN',
        sidebarCollapsed: false,
      },
    })
    expect(invoke).not.toHaveBeenCalled()
    await api.onRuntimeLog(() => {})
    expect(listen).not.toHaveBeenCalled()
  })

  it('reads a stored system theme preference in browser fallback mode', async () => {
    vi.doMock('@tauri-apps/api/core', () => ({
      invoke: undefined,
    }))
    vi.doMock('@tauri-apps/api/event', () => ({
      listen: undefined,
    }))
    window.localStorage.setItem('bizclaw-ui-preferences', JSON.stringify({
      theme: 'system',
      locale: 'zh-CN',
    }))

    const api = await import('./api')

    await expect(api.detectEnvironment()).resolves.toMatchObject({
      uiPreferences: {
        theme: 'system',
        locale: 'zh-CN',
        sidebarCollapsed: false,
      },
    })
  })

  it('forwards agent and skill commands to tauri invoke when available', async () => {
    const invoke = vi.fn(async (_command: string, args?: Record<string, unknown>) => args ?? null)
    vi.doMock('@tauri-apps/api/core', () => ({
      invoke,
    }))
    vi.doMock('@tauri-apps/api/event', () => ({
      listen: vi.fn(),
    }))
    ;(globalThis as { __TAURI_INTERNALS__?: { invoke?: unknown } }).__TAURI_INTERNALS__ = {
      invoke,
    }

    const api = await import('./api')

    await api.listOpenClawAgents()
    await api.listOpenClawAgentBindings('main')
    await api.createOpenClawAgent({
      name: 'Ops',
      workspace: '/tmp/openclaw-ops',
      model: 'minimax-cn/MiniMax-M2.5',
      bindings: ['telegram'],
    })
    await api.listOpenClawSkills()
    await api.checkOpenClawSkills()
    await api.getOpenClawSkillInfo('coding-agent')

    expect(invoke).toHaveBeenCalledWith('list_openclaw_agents', undefined)
    expect(invoke).toHaveBeenCalledWith('list_openclaw_agent_bindings', { agentId: 'main' })
    expect(invoke).toHaveBeenCalledWith('create_openclaw_agent', {
      request: {
        name: 'Ops',
        workspace: '/tmp/openclaw-ops',
        model: 'minimax-cn/MiniMax-M2.5',
        bindings: ['telegram'],
      },
    })
    expect(invoke).toHaveBeenCalledWith('list_openclaw_skills', undefined)
    expect(invoke).toHaveBeenCalledWith('check_openclaw_skills', undefined)
    expect(invoke).toHaveBeenCalledWith('get_openclaw_skill_info', { name: 'coding-agent' })
  })
})
