// @vitest-environment node

import { existsSync, readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

import { describe, expect, it } from 'vitest'

const rootDir = resolve(fileURLToPath(new URL('..', import.meta.url)))
const iconsDir = resolve(rootDir, 'src-tauri', 'icons')
const tauriConfigPath = resolve(rootDir, 'src-tauri', 'tauri.conf.json')
const tauriConfig = JSON.parse(readFileSync(tauriConfigPath, 'utf8')) as {
  bundle?: {
    createUpdaterArtifacts?: boolean | string
    icon?: string[]
  }
  plugins?: {
    updater?: {
      endpoints?: string[]
      pubkey?: string
    }
  }
}

const desktopIcons = [
  'icons/32x32.png',
  'icons/128x128.png',
  'icons/128x128@2x.png',
  'icons/icon.icns',
  'icons/icon.ico',
]

describe('release assets', () => {
  it('includes the desktop icon files required by tauri-build', () => {
    for (const iconPath of desktopIcons) {
      expect(existsSync(resolve(rootDir, 'src-tauri', iconPath))).toBe(true)
    }
  })

  it('declares the desktop icon set in tauri.conf.json', () => {
    expect(tauriConfig.bundle?.icon).toEqual(desktopIcons)
  })

  it('enables updater artifacts and configures a signed latest.json endpoint', () => {
    expect(tauriConfig.bundle?.createUpdaterArtifacts).toBe(true)
    expect(tauriConfig.plugins?.updater?.endpoints).toEqual([
      'https://github.com/GoyacJ/BizClaw/releases/latest/download/latest.json',
    ])
    expect(tauriConfig.plugins?.updater?.pubkey).toBeTruthy()
  })
})
