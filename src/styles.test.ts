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
    expect(styles).toMatch(/\.modal-backdrop\s*\{[^}]*backdrop-filter:\s*blur\(4px\);/s)
    expect(styles).toMatch(/\.modal-card\s*\{[^}]*max-height:\s*calc\(100vh\s*-\s*48px\);[^}]*overflow-y:\s*auto;/s)
    expect(styles).toMatch(/\.connection-test-output pre\s*\{[^}]*max-height:\s*240px;[^}]*overflow:\s*auto;/s)
  })

  it('avoids blur effects on regular shell cards to keep desktop repaints light', () => {
    expect(styles).toMatch(/\.surface-card\s*\{[^}]*box-shadow:\s*var\(--shadow\);/s)
    expect(styles).not.toMatch(/\.surface-card\s*\{[^}]*backdrop-filter:/s)
  })

  it('uses dark-theme-specific tokens for shell background, secondary buttons, and status bar chrome', () => {
    expect(styles).toMatch(/\[data-theme="dark"\]\s*\{[^}]*--button-secondary-bg:\s*[^;]+;/s)
    expect(styles).toMatch(/\[data-theme="dark"\]\s*\{[^}]*--status-bar-bg:\s*[^;]+;/s)
    expect(styles).toMatch(/\[data-theme="dark"\]\s*\{[^}]*--nav-button-bg:\s*[^;]+;/s)
    expect(styles).toMatch(/body\s*\{[^}]*radial-gradient\([^)]+var\(--bg-accent-primary\)[^)]+\)[^}]*radial-gradient\([^)]+var\(--bg-accent-secondary\)[^)]+\)[^}]*linear-gradient\(180deg,\s*var\(--body-bg-start\)\s*0%,\s*var\(--bg\)\s*48%,\s*var\(--body-bg-end\)\s*100%\);/s)
    expect(styles).not.toMatch(/body::before\s*\{/)
    expect(styles).toMatch(/\.secondary-button\s*\{[^}]*color:\s*var\(--button-secondary-ink\);[^}]*background:\s*var\(--button-secondary-bg\);[^}]*border-color:\s*var\(--button-secondary-border\);/s)
    expect(styles).toMatch(/\.nav-button\s*\{[^}]*background:\s*var\(--nav-button-bg\);/s)
    expect(styles).toMatch(/\.status-bar\s*\{[^}]*border-top:\s*1px\s*solid\s*var\(--status-bar-border\);/s)
    expect(styles).toMatch(/\.status-bar\s*\{[^}]*background:\s*var\(--status-bar-bg\);/s)
    expect(styles).toMatch(/\.status-bar-item\s*\{[^}]*border:\s*1px\s*solid\s*var\(--status-bar-item-border\);/s)
    expect(styles).toMatch(/\.status-bar-item\s*\{[^}]*background:\s*var\(--status-bar-item-bg\);/s)
  })

  it('keeps shell chrome lighter than content cards with dedicated shadow and edge tokens', () => {
    expect(styles).toMatch(/:root,\s*\[data-theme="light"\]\s*\{[^}]*--chrome-line:\s*[^;]+;/s)
    expect(styles).toMatch(/:root,\s*\[data-theme="light"\]\s*\{[^}]*--chrome-shadow:\s*[^;]+;/s)
    expect(styles).toMatch(/:root,\s*\[data-theme="light"\]\s*\{[^}]*--card-line:\s*[^;]+;/s)
    expect(styles).toMatch(/:root,\s*\[data-theme="light"\]\s*\{[^}]*--card-shadow:\s*[^;]+;/s)
    expect(styles).toMatch(/\.sidebar\s*\{[^}]*border-color:\s*var\(--chrome-line\);[^}]*box-shadow:\s*var\(--chrome-shadow\);/s)
    expect(styles).toMatch(/\.workspace-header\s*\{[^}]*border-color:\s*var\(--chrome-line\);[^}]*box-shadow:\s*var\(--chrome-shadow\);/s)
    expect(styles).toMatch(/\.overview-card,\s*\.section-card\s*\{[^}]*border-color:\s*var\(--card-line\);[^}]*box-shadow:\s*var\(--card-shadow\);/s)
  })

  it('styles buttons and status chips with dedicated depth and tone-specific edges', () => {
    expect(styles).toMatch(/:root,\s*\[data-theme="light"\]\s*\{[^}]*--button-secondary-shadow:\s*[^;]+;/s)
    expect(styles).toMatch(/:root,\s*\[data-theme="light"\]\s*\{[^}]*--button-secondary-hover-shadow:\s*[^;]+;/s)
    expect(styles).toMatch(/:root,\s*\[data-theme="light"\]\s*\{[^}]*--primary-button-hover-shadow:\s*[^;]+;/s)
    expect(styles).toMatch(/:root,\s*\[data-theme="light"\]\s*\{[^}]*--chip-active-border:\s*[^;]+;/s)
    expect(styles).toMatch(/:root,\s*\[data-theme="light"\]\s*\{[^}]*--chip-success-border:\s*[^;]+;/s)
    expect(styles).toMatch(/\.secondary-button\s*\{[^}]*box-shadow:\s*var\(--button-secondary-shadow\);/s)
    expect(styles).toMatch(/\.secondary-button:hover\s*\{[^}]*box-shadow:\s*var\(--button-secondary-hover-shadow\);/s)
    expect(styles).toMatch(/\.primary-button:hover\s*\{[^}]*box-shadow:\s*var\(--primary-button-hover-shadow\);/s)
    expect(styles).toMatch(/\.status-chip\[data-tone="success"\]\s*\{[^}]*border-color:\s*var\(--chip-success-border\);/s)
    expect(styles).toMatch(/\.status-chip\[data-tone="active"\]\s*\{[^}]*border-color:\s*var\(--chip-active-border\);/s)
  })

  it('gives form fields and settings option buttons the same polished interaction language', () => {
    expect(styles).toMatch(/:root,\s*\[data-theme="light"\]\s*\{[^}]*--input-border:\s*[^;]+;/s)
    expect(styles).toMatch(/:root,\s*\[data-theme="light"\]\s*\{[^}]*--input-shadow:\s*[^;]+;/s)
    expect(styles).toMatch(/:root,\s*\[data-theme="light"\]\s*\{[^}]*--input-focus-border:\s*[^;]+;/s)
    expect(styles).toMatch(/:root,\s*\[data-theme="light"\]\s*\{[^}]*--input-focus-shadow:\s*[^;]+;/s)
    expect(styles).toMatch(/\.field input\s*\{[^}]*border-color:\s*var\(--input-border\);[^}]*box-shadow:\s*var\(--input-shadow\);/s)
    expect(styles).toMatch(/\.field input:focus\s*\{[^}]*border-color:\s*var\(--input-focus-border\);[^}]*box-shadow:\s*var\(--input-focus-shadow\);/s)
    expect(styles).toMatch(/\.settings-option-row\s+\.secondary-button\s*\{[^}]*min-width:\s*104px;[^}]*justify-content:\s*center;[^}]*border-radius:\s*999px;/s)
    expect(styles).toMatch(/\.settings-option-row\s+\.secondary-button\[data-active=\"true\"\]\s*\{[^}]*box-shadow:\s*var\(--button-secondary-hover-shadow\);/s)
  })

  it('gives support tiles a distinct compact card treatment for version metadata', () => {
    expect(styles).toMatch(/:root,\s*\[data-theme="light"\]\s*\{[^}]*--support-tile-line:\s*[^;]+;/s)
    expect(styles).toMatch(/:root,\s*\[data-theme="light"\]\s*\{[^}]*--support-tile-shadow:\s*[^;]+;/s)
    expect(styles).toMatch(/:root,\s*\[data-theme="light"\]\s*\{[^}]*--support-tile-top:\s*[^;]+;/s)
    expect(styles).toMatch(/\.support-tile\s*\{[^}]*border:\s*1px\s*solid\s*var\(--support-tile-line\);[^}]*background:\s*linear-gradient\(180deg,\s*var\(--support-tile-top\),\s*var\(--surface-muted\)\);[^}]*box-shadow:\s*var\(--support-tile-shadow\);/s)
    expect(styles).toMatch(/\.support-tile strong\s*\{[^}]*font-size:\s*1\.04rem;[^}]*line-height:\s*1\.35;/s)
  })

  it('gives the version panel a clear action row and note stack rhythm', () => {
    expect(styles).toMatch(/\.version-actions\s*\{[^}]*margin-top:\s*6px;[^}]*padding-top:\s*4px;/s)
    expect(styles).toMatch(/\.version-notes\s*\{[^}]*display:\s*grid;[^}]*gap:\s*10px;[^}]*margin-top:\s*14px;[^}]*padding-top:\s*14px;[^}]*border-top:\s*1px\s*solid\s*var\(--line\);/s)
    expect(styles).toMatch(/\.version-note\s*\{[^}]*padding:\s*12px\s*14px;[^}]*border:\s*1px\s*solid\s*var\(--line\);[^}]*border-radius:\s*14px;[^}]*background:\s*var\(--row-bg\);/s)
    expect(styles).toMatch(/\.version-note--release\s*\{[^}]*line-height:\s*1\.6;/s)
  })

  it('gives the settings page its own calmer spacing and title hierarchy', () => {
    expect(styles).toMatch(/\.settings-stack\s*\{[^}]*gap:\s*20px;/s)
    expect(styles).toMatch(/\.settings-card\s*\{[^}]*padding:\s*22px\s*24px;/s)
    expect(styles).toMatch(/\.settings-card-header\s*\{[^}]*align-items:\s*flex-start;[^}]*margin-bottom:\s*0;/s)
    expect(styles).toMatch(/\.settings-title-block\s*\{[^}]*display:\s*grid;[^}]*gap:\s*5px;/s)
    expect(styles).toMatch(/\.settings-card\s+\.supporting-text\s*\{[^}]*max-width:\s*54ch;[^}]*margin-top:\s*10px;/s)
    expect(styles).toMatch(/\.settings-status-chip\s*\{[^}]*min-height:\s*36px;[^}]*padding:\s*0\s*14px;[^}]*box-shadow:\s*var\(--button-secondary-shadow\);/s)
  })
})
