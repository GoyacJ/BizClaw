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

import { describeBizClawUpdaterError } from './bizclaw-updater'

describe('describeBizClawUpdaterError', () => {
  it('translates missing release manifest failures into an actionable message', () => {
    expect(describeBizClawUpdaterError(
      new Error('Could not fetch a valid release JSON from the remote'),
    )).toBe('无法读取 GitHub Release 的更新清单，请确认最新 Release 已上传 latest.json、签名文件和对应安装包。')
  })

  it('passes through unrelated errors unchanged', () => {
    expect(describeBizClawUpdaterError(new Error('network timeout'))).toBe('network timeout')
  })
})
