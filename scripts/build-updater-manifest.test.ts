// @vitest-environment node

import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { resolve } from 'node:path'

import { afterEach, describe, expect, it } from 'vitest'

import { buildUpdaterManifest } from './build-updater-manifest.mjs'

describe('buildUpdaterManifest', () => {
  let fixtureDir: string | null = null

  afterEach(() => {
    if (fixtureDir) {
      rmSync(fixtureDir, { recursive: true, force: true })
      fixtureDir = null
    }
  })

  it('builds latest.json for macOS and Windows updater assets hosted on GitHub Releases', () => {
    fixtureDir = mkdtempSync(resolve(tmpdir(), 'bizclaw-updater-'))
    writeFileSync(resolve(fixtureDir, 'BizClaw.app.tar.gz.sig'), 'mac-signature\n')
    writeFileSync(resolve(fixtureDir, 'BizClaw_0.1.9_x64_en-US.msi.sig'), 'windows-signature\n')
    writeFileSync(resolve(fixtureDir, 'BizClaw.app.tar.gz'), 'mac archive')
    writeFileSync(resolve(fixtureDir, 'BizClaw_0.1.9_x64_en-US.msi'), 'windows installer')

    const outputPath = resolve(fixtureDir, 'latest.json')
    buildUpdaterManifest({
      releaseTag: 'v0.1.9',
      repository: 'GoyacJ/BizClaw',
      assetsDirectory: fixtureDir,
      outputPath,
      notes: 'Automated BizClaw release for v0.1.9.',
      pubDate: '2026-03-14T00:00:00.000Z',
    })

    const manifest = JSON.parse(readFileSync(outputPath, 'utf8')) as {
      version: string
      notes: string
      pub_date: string
      platforms: Record<string, { signature: string, url: string }>
    }

    expect(manifest.version).toBe('0.1.9')
    expect(manifest.notes).toBe('Automated BizClaw release for v0.1.9.')
    expect(manifest.pub_date).toBe('2026-03-14T00:00:00.000Z')
    expect(manifest.platforms['darwin-aarch64']).toEqual({
      signature: 'mac-signature',
      url: 'https://github.com/GoyacJ/BizClaw/releases/download/v0.1.9/BizClaw.app.tar.gz',
    })
    expect(manifest.platforms['windows-x86_64']).toEqual({
      signature: 'windows-signature',
      url: 'https://github.com/GoyacJ/BizClaw/releases/download/v0.1.9/BizClaw_0.1.9_x64_en-US.msi',
    })
  })

  it('finds updater assets in nested directories created by downloaded workflow artifacts', () => {
    fixtureDir = mkdtempSync(resolve(tmpdir(), 'bizclaw-updater-'))
    const macArtifactsDir = resolve(fixtureDir, 'macos')
    const windowsArtifactsDir = resolve(fixtureDir, 'msi')
    mkdirSync(macArtifactsDir, { recursive: true })
    mkdirSync(windowsArtifactsDir, { recursive: true })
    writeFileSync(resolve(macArtifactsDir, 'BizClaw.app.tar.gz.sig'), 'mac-signature\n')
    writeFileSync(resolve(windowsArtifactsDir, 'BizClaw_0.1.9_x64_en-US.msi.sig'), 'windows-signature\n')
    writeFileSync(resolve(macArtifactsDir, 'BizClaw.app.tar.gz'), 'mac archive')
    writeFileSync(resolve(windowsArtifactsDir, 'BizClaw_0.1.9_x64_en-US.msi'), 'windows installer')

    const outputPath = resolve(fixtureDir, 'latest.json')
    buildUpdaterManifest({
      releaseTag: 'v0.1.9',
      repository: 'GoyacJ/BizClaw',
      assetsDirectory: fixtureDir,
      outputPath,
      notes: 'Automated BizClaw release for v0.1.9.',
      pubDate: '2026-03-14T00:00:00.000Z',
    })

    const manifest = JSON.parse(readFileSync(outputPath, 'utf8')) as {
      version: string
      notes: string
      pub_date: string
      platforms: Record<string, { signature: string, url: string }>
    }

    expect(manifest.platforms['darwin-aarch64']).toEqual({
      signature: 'mac-signature',
      url: 'https://github.com/GoyacJ/BizClaw/releases/download/v0.1.9/BizClaw.app.tar.gz',
    })
    expect(manifest.platforms['windows-x86_64']).toEqual({
      signature: 'windows-signature',
      url: 'https://github.com/GoyacJ/BizClaw/releases/download/v0.1.9/BizClaw_0.1.9_x64_en-US.msi',
    })
  })
})
