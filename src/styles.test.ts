// @ts-nocheck
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'

import { describe, expect, it } from 'vitest'

const styles = readFileSync(resolve(process.cwd(), 'src/styles.css'), 'utf8')

describe('shell layout styles', () => {
  it('keeps the sidebar as a full-height left rail at the default window width', () => {
    expect(styles).toMatch(/\.ops-shell\s*\{[^}]*align-items:\s*stretch;/s)
    expect(styles).toMatch(/\.sidebar\s*\{[^}]*position:\s*sticky;[^}]*top:\s*16px;[^}]*min-height:\s*calc\(100vh\s*-\s*var\(--status-bar-height\)\s*-\s*34px\);/s)
    expect(styles).toMatch(/@media\s*\(max-width:\s*900px\)/)
    expect(styles).not.toMatch(/@media\s*\(max-width:\s*1080px\)/)
  })

  it('pins the status bar to the bottom as a compact project-style strip', () => {
    expect(styles).toMatch(/\.status-bar\s*\{[^}]*position:\s*fixed;[^}]*bottom:\s*0;[^}]*min-height:\s*var\(--status-bar-height\);/s)
    expect(styles).toMatch(/\.status-bar-item\s*\{[^}]*display:\s*inline-flex;[^}]*padding:\s*4px\s*9px;[^}]*border-radius:\s*999px;/s)
    expect(styles).toMatch(/\.status-bar-item span\s*\{[^}]*font-size:\s*0\.68rem;/s)
  })

  it('lets the connection test modal scroll when the output is tall', () => {
    expect(styles).toMatch(/\.modal-backdrop\s*\{[^}]*overflow-y:\s*auto;/s)
    expect(styles).toMatch(/\.modal-card\s*\{[^}]*max-height:\s*calc\(100vh\s*-\s*48px\);[^}]*overflow-y:\s*auto;/s)
    expect(styles).toMatch(/\.connection-test-output pre\s*\{[^}]*max-height:\s*240px;[^}]*overflow:\s*auto;/s)
  })
})
