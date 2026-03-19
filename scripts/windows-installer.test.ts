// @vitest-environment node

import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

import { describe, expect, it } from 'vitest'

const rootDir = resolve(fileURLToPath(new URL('..', import.meta.url)))
const tauriConfigPath = resolve(rootDir, 'src-tauri', 'tauri.conf.json')
const wixTemplatePath = resolve(rootDir, 'src-tauri', 'wix', 'main.wxs')

const tauriConfig = JSON.parse(readFileSync(tauriConfigPath, 'utf8')) as {
  bundle?: {
    windows?: {
      wix?: {
        template?: string
      }
    }
  }
}
const wixTemplate = readFileSync(wixTemplatePath, 'utf8')

describe('windows installer template', () => {
  it('uses the tracked WiX template for Windows bundles', () => {
    expect(tauriConfig.bundle?.windows?.wix?.template).toBe('wix/main.wxs')
  })

  it('does not auto-launch BizClaw after installation or updater-driven MSI installs', () => {
    expect(wixTemplate).not.toMatch(/AUTOLAUNCHAPP/u)
    expect(wixTemplate).not.toMatch(/LAUNCHAPPARGS/u)
    expect(wixTemplate).not.toMatch(/LaunchApplication/u)
    expect(wixTemplate).not.toMatch(/WIXUI_EXITDIALOGOPTIONALCHECKBOX/u)
  })
})
