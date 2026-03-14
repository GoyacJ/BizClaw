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
      targetSshInstalled: false,
      openclawInstalled: false,
      openclawVersion: null,
      latestOpenclawVersion: null,
      updateAvailable: false,
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
})
