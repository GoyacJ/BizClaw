import { describe, expect, it, vi } from 'vitest'

vi.mock('@tauri-apps/api/app', () => ({
  getVersion: vi.fn(),
}))

vi.mock('@tauri-apps/plugin-process', () => ({
  relaunch: vi.fn(),
}))

vi.mock('@tauri-apps/plugin-updater', () => ({
  check: vi.fn(),
}))

import { check } from '@tauri-apps/plugin-updater'
import { getVersion } from '@tauri-apps/api/app'
import { relaunch } from '@tauri-apps/plugin-process'

import {
  checkForBizClawUpdate,
  describeBizClawUpdaterError,
  getCurrentBizClawVersion,
  relaunchBizClaw,
} from './bizclaw-updater'

describe('describeBizClawUpdaterError', () => {
  it('translates missing release manifest failures into an actionable message', () => {
    expect(describeBizClawUpdaterError(
      new Error('Could not fetch a valid release JSON from the remote'),
    )).toBe('无法读取 GitHub Release 的更新清单，请确认最新 Release 已上传 latest.json、签名文件和对应安装包。')
  })

  it('passes through unrelated errors unchanged', () => {
    expect(describeBizClawUpdaterError(new Error('network timeout'))).toBe('network timeout')
  })

  it('skips updater calls when tauri internals are unavailable', async () => {
    delete (globalThis as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__

    await expect(getCurrentBizClawVersion()).resolves.toBeNull()
    await expect(checkForBizClawUpdate()).resolves.toBeNull()
    await expect(relaunchBizClaw()).resolves.toBeUndefined()

    expect(getVersion).not.toHaveBeenCalled()
    expect(check).not.toHaveBeenCalled()
    expect(relaunch).not.toHaveBeenCalled()
  })
})
