// @vitest-environment node

import { existsSync } from 'node:fs'
import { resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

import { describe, expect, it } from 'vitest'

const rootDir = resolve(fileURLToPath(new URL('..', import.meta.url)))
const iconsDir = resolve(rootDir, 'src-tauri', 'icons')

describe('release assets', () => {
  it('includes the Windows icon required by tauri-build', () => {
    expect(existsSync(resolve(iconsDir, 'icon.ico'))).toBe(true)
  })
})
