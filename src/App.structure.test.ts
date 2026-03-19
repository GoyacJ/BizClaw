// @ts-nocheck
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'

import { describe, expect, it } from 'vitest'

const appSource = readFileSync(resolve(process.cwd(), 'src/App.vue'), 'utf8')

describe('App structure', () => {
  it('delegates each workspace section to a dedicated component', () => {
    expect(appSource).toMatch(/import AppChatSection from ['"]\.\/components\/AppChatSection\.vue['"]/)
    expect(appSource).toMatch(/import AppAgentsSection from ['"]\.\/components\/AppAgentsSection\.vue['"]/)
    expect(appSource).toMatch(/import AppOverviewSection from ['"]\.\/components\/AppOverviewSection\.vue['"]/)
    expect(appSource).toMatch(/import AppInstallSection from ['"]\.\/components\/AppInstallSection\.vue['"]/)
    expect(appSource).toMatch(/import AppConnectionSection from ['"]\.\/components\/AppConnectionSection\.vue['"]/)
    expect(appSource).toMatch(/import AppSkillsSection from ['"]\.\/components\/AppSkillsSection\.vue['"]/)
    expect(appSource).toMatch(/import AppSettingsSection from ['"]\.\/components\/AppSettingsSection\.vue['"]/)
    expect(appSource).toMatch(/import AppRuntimeLogsSection from ['"]\.\/components\/AppRuntimeLogsSection\.vue['"]/)
    expect(appSource).toMatch(/<AppOverviewSection[\s\S]*v-if="activeSection === 'overview'"/)
    expect(appSource).toMatch(/<AppChatSection[\s\S]*v-else-if="activeSection === 'chat'"/)
    expect(appSource).toMatch(/<AppAgentsSection[\s\S]*v-else-if="activeSection === 'agent'"/)
    expect(appSource).toMatch(/<AppInstallSection[\s\S]*v-else-if="activeSection === 'install'"/)
    expect(appSource).toMatch(/<AppConnectionSection[\s\S]*v-else-if="activeSection === 'connection'"/)
    expect(appSource).toMatch(/<AppSkillsSection[\s\S]*v-else-if="activeSection === 'skill'"/)
    expect(appSource).toMatch(/<AppSettingsSection[\s\S]*v-else-if="activeSection === 'settings'"/)
    expect(appSource).toMatch(/<AppRuntimeLogsSection[\s\S]*v-else/)
  })

  it('keeps workspace sections cached between switches', () => {
    expect(appSource).toMatch(/<KeepAlive>/)
    expect(appSource).toMatch(/<\/KeepAlive>/)
    expect(appSource).toMatch(/<KeepAlive>[\s\S]*<AppOverviewSection/)
  })

  it('extracts the shell chrome into dedicated components', () => {
    expect(appSource).toMatch(/import AppSidebar from ['"]\.\/components\/AppSidebar\.vue['"]/)
    expect(appSource).toMatch(/import AppWorkspaceHeader from ['"]\.\/components\/AppWorkspaceHeader\.vue['"]/)
    expect(appSource).toMatch(/import AppStatusBar from ['"]\.\/components\/AppStatusBar\.vue['"]/)
    expect(appSource).toMatch(/<AppSidebar[\s\S]*:sections="sections"/)
    expect(appSource).toMatch(/<AppWorkspaceHeader[\s\S]*:section-title="sectionTitle"/)
    expect(appSource).toMatch(/<AppStatusBar[\s\S]*:status-items="statusItems"/)
  })
})
