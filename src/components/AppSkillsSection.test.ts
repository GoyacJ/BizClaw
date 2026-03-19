import { createApp, nextTick, ref } from 'vue'
import { afterEach, describe, expect, it, vi } from 'vitest'

import AppSkillsSection from './AppSkillsSection.vue'

describe('AppSkillsSection', () => {
  let host: HTMLDivElement | null = null
  let app: ReturnType<typeof createApp> | null = null

  afterEach(() => {
    app?.unmount()
    host?.remove()
    app = null
    host = null
  })

  it('provides a client-side filter input for skill list', async () => {
    host = document.createElement('div')
    document.body.appendChild(host)

    app = createApp(AppSkillsSection, {
      state: createState(),
    })
    app.mount(host)
    await nextTick()

    expect(host.textContent).toContain('筛选技能')
  })
})

function createState() {
  return {
    inventory: ref({
      workspaceDir: '/Users/goya/.openclaw/workspace',
      managedSkillsDir: '/Users/goya/.openclaw/skills',
      skills: [
        {
          name: 'calendar',
          description: 'Calendar utilities',
          eligible: true,
          disabled: false,
          blockedByAllowlist: false,
          source: 'openclaw-workspace',
          bundled: false,
          locationKind: 'workspaceLocal',
          canDelete: true,
          missing: {
            bins: [],
            anyBins: [],
            env: [],
            config: [],
            os: [],
          },
        },
      ],
    }),
    checkReport: ref({
      summary: {
        total: 1,
        eligible: 1,
        disabled: 0,
        blocked: 0,
        missingRequirements: 0,
      },
      eligible: ['calendar'],
      disabled: [],
      blocked: [],
      missingRequirements: [],
    }),
    searchResults: ref([]),
    selectedSkillName: ref('calendar'),
    selectedSkillInfo: ref({
      name: 'calendar',
      description: 'Calendar utilities',
      source: 'openclaw-workspace',
      bundled: false,
      locationKind: 'workspaceLocal',
      canDelete: true,
      filePath: '/tmp/calendar',
      baseDir: '/tmp',
      skillKey: 'calendar',
      always: false,
      disabled: false,
      blockedByAllowlist: false,
      eligible: true,
      requirements: { bins: [], anyBins: [], env: [], config: [], os: [] },
      missing: { bins: [], anyBins: [], env: [], config: [], os: [] },
      configChecks: [],
      install: [],
    }),
    loading: ref(false),
    searchBusy: ref(false),
    detailLoading: ref(false),
    mutationBusy: ref(false),
    error: ref<string | null>(null),
    refreshSkills: vi.fn(),
    selectSkill: vi.fn(),
    searchInstallableSkills: vi.fn(),
    clearSkillSearch: vi.fn(),
    installSkill: vi.fn(),
    deleteLocalSkill: vi.fn(),
  }
}
