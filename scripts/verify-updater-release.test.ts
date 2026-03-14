// @vitest-environment node

import { describe, expect, it } from 'vitest'

import { assertUpdaterReleaseAssets } from './verify-updater-release.mjs'

describe('assertUpdaterReleaseAssets', () => {
  it('accepts a release payload that includes the updater manifest and signed assets', () => {
    expect(() => assertUpdaterReleaseAssets({
      assets: [
        { name: 'BizClaw.app.tar.gz' },
        { name: 'BizClaw.app.tar.gz.sig' },
        { name: 'BizClaw_0.1.9_x64_zh-CN.msi' },
        { name: 'BizClaw_0.1.9_x64_zh-CN.msi.sig' },
        { name: 'latest.json' },
      ],
    })).not.toThrow()
  })

  it('fails when latest.json is missing from the published release', () => {
    expect(() => assertUpdaterReleaseAssets({
      assets: [
        { name: 'BizClaw.app.tar.gz' },
        { name: 'BizClaw.app.tar.gz.sig' },
        { name: 'BizClaw_0.1.9_x64_zh-CN.msi' },
        { name: 'BizClaw_0.1.9_x64_zh-CN.msi.sig' },
      ],
    })).toThrowError(/latest\.json/u)
  })
})
