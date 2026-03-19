<script setup lang="ts">
import { computed, reactive, ref, watch, type Ref } from 'vue'

import { translate } from '@/lib/i18n'
import type {
  ClawHubSkillSearchResult,
  InstallClawHubSkillRequest,
  OpenClawSkillInfo,
  OpenClawSkillInventory,
  OpenClawSkillLocationKind,
  OpenClawSkillSummary,
  OpenClawSkillCheckReport,
  SearchClawHubSkillsRequest,
} from '@/types'

interface SkillsSectionState {
  inventory: Ref<OpenClawSkillInventory>
  checkReport: Ref<OpenClawSkillCheckReport>
  searchResults: Ref<ClawHubSkillSearchResult[]>
  selectedSkillName: Ref<string | null>
  selectedSkillInfo: Ref<OpenClawSkillInfo | null>
  loading: Ref<boolean>
  searchBusy: Ref<boolean>
  detailLoading: Ref<boolean>
  mutationBusy: Ref<boolean>
  error: Ref<string | null>
  refreshSkills: () => Promise<unknown> | unknown
  selectSkill: (name: string) => Promise<unknown> | unknown
  searchInstallableSkills: (request: SearchClawHubSkillsRequest) => Promise<unknown> | unknown
  clearSkillSearch: () => void
  installSkill: (request: InstallClawHubSkillRequest) => Promise<unknown> | unknown
  deleteLocalSkill: (name: string) => Promise<unknown> | unknown
}

const props = defineProps<{
  state: SkillsSectionState
}>()

const installOpen = ref(false)
const hasSearched = ref(false)
const filterText = ref('')
const installDraft = reactive<{
  query: string
  location: InstallClawHubSkillRequest['location']
}>({
  query: '',
  location: 'workspaceLocal',
})

const groupedSkills = computed(() => {
  const groups: Array<{ key: OpenClawSkillLocationKind, label: string, skills: OpenClawSkillSummary[] }> = [
    { key: 'workspaceLocal', label: translate('skills.workspaceLocal'), skills: [] },
    { key: 'sharedLocal', label: translate('skills.sharedLocal'), skills: [] },
    { key: 'bundled', label: translate('skills.bundled'), skills: [] },
    { key: 'external', label: translate('skills.external'), skills: [] },
  ]

  const normalizedFilter = filterText.value.trim().toLowerCase()
  const visibleSkills = normalizedFilter
    ? props.state.inventory.value.skills.filter((skill) => (
        skill.name.toLowerCase().includes(normalizedFilter)
        || skill.description.toLowerCase().includes(normalizedFilter)
      ))
    : props.state.inventory.value.skills

  for (const skill of visibleSkills) {
    const group = groups.find((entry) => entry.key === skill.locationKind)
    group?.skills.push(skill)
  }

  return groups.filter((group) => group.skills.length > 0)
})

const canInstallToWorkspace = computed(() => Boolean(props.state.inventory.value.workspaceDir))
const canInstallToShared = computed(() => Boolean(props.state.inventory.value.managedSkillsDir))
const canSearch = computed(() => (
  installDraft.query.trim().length > 0
  && (canInstallToWorkspace.value || canInstallToShared.value)
))

watch([canInstallToWorkspace, canInstallToShared], ([workspaceAvailable, sharedAvailable]) => {
  if (installDraft.location === 'workspaceLocal' && !workspaceAvailable && sharedAvailable) {
    installDraft.location = 'sharedLocal'
  } else if (installDraft.location === 'sharedLocal' && !sharedAvailable && workspaceAvailable) {
    installDraft.location = 'workspaceLocal'
  }
}, { immediate: true })

function skillTone(skill: OpenClawSkillSummary) {
  if (skill.eligible) {
    return 'success'
  }
  if (skill.blockedByAllowlist) {
    return 'error'
  }
  if (skill.disabled) {
    return 'neutral'
  }
  return 'active'
}

function openInstallModal() {
  installOpen.value = true
}

function closeInstallModal() {
  installOpen.value = false
  hasSearched.value = false
  props.state.clearSkillSearch()
}

async function submitSkillSearch() {
  if (!canSearch.value) {
    return
  }

  hasSearched.value = true
  await props.state.searchInstallableSkills({
    query: installDraft.query.trim(),
    limit: 8,
  })
}

async function installSearchResult(result: ClawHubSkillSearchResult) {
  await props.state.installSkill({
    slug: result.slug,
    location: installDraft.location,
  })
  closeInstallModal()
}

async function deleteSelectedSkill() {
  const skill = props.state.selectedSkillInfo.value
  if (!skill?.canDelete) {
    return
  }

  if (globalThis.confirm && !globalThis.confirm(translate('skills.deleteConfirm', { name: skill.name }))) {
    return
  }

  await props.state.deleteLocalSkill(skill.name)
}
</script>

<template>
  <section class="page-stack">
    <article class="surface-card section-card">
      <div class="section-header">
        <div>
          <p class="eyebrow">{{ translate('skills.eyebrow') }}</p>
          <h3>{{ translate('skills.title') }}</h3>
        </div>
        <span class="status-chip" :data-tone="props.state.loading.value ? 'active' : 'neutral'">
          {{ props.state.checkReport.value.summary.total }}
        </span>
      </div>
      <p class="supporting-text">{{ translate('skills.detail') }}</p>

      <div class="support-grid support-grid--metrics">
        <div class="support-tile">
          <span>{{ translate('skills.total') }}</span>
          <strong>{{ props.state.checkReport.value.summary.total }}</strong>
        </div>
        <div class="support-tile">
          <span>{{ translate('skills.eligible') }}</span>
          <strong>{{ props.state.checkReport.value.summary.eligible }}</strong>
        </div>
        <div class="support-tile">
          <span>{{ translate('skills.missing') }}</span>
          <strong>{{ props.state.checkReport.value.summary.missingRequirements }}</strong>
        </div>
        <div class="support-tile">
          <span>{{ translate('skills.disabled') }}</span>
          <strong>{{ props.state.checkReport.value.summary.disabled }}</strong>
        </div>
      </div>

      <div class="button-row">
        <button class="primary-button" type="button" @click="openInstallModal">
          {{ translate('skills.create') }}
        </button>
        <button class="secondary-button" type="button" :disabled="props.state.loading.value" @click="props.state.refreshSkills">
          {{ translate('skills.refresh') }}
        </button>
      </div>
    </article>

    <div class="management-layout">
      <article class="surface-card section-card">
        <div class="section-header">
          <div>
            <p class="eyebrow">{{ translate('skills.summaryTitle') }}</p>
            <h3>{{ translate('skills.title') }}</h3>
          </div>
        </div>

        <div v-if="props.state.inventory.value.skills.length === 0" class="empty-state">
          {{ translate('skills.empty') }}
        </div>

        <div v-else class="page-stack">
          <label class="field field--span">
            <span>{{ translate('skills.filterLabel') }}</span>
            <input v-model="filterText" :placeholder="translate('skills.filterPlaceholder')" />
          </label>
          <section v-for="group in groupedSkills" :key="group.key" class="management-group">
            <div class="section-header">
              <div>
                <span class="card-label">{{ group.label }}</span>
              </div>
            </div>
            <div class="management-list">
              <button
                v-for="skill in group.skills"
                :key="skill.name"
                class="management-row"
                :data-active="String(skill.name === props.state.selectedSkillName.value)"
                type="button"
                @click="void props.state.selectSkill(skill.name)"
              >
                <div class="management-row-main">
                  <strong>{{ skill.name }}</strong>
                  <span>{{ skill.description }}</span>
                </div>
                <div class="management-row-meta">
                  <span class="status-chip" :data-tone="skillTone(skill)">
                    {{ skill.eligible ? translate('skills.eligible') : translate('skills.missing') }}
                  </span>
                  <span class="status-chip" :data-tone="skill.canDelete ? 'success' : 'neutral'">
                    {{ skill.canDelete ? translate('skills.deletable') : translate('skills.readOnly') }}
                  </span>
                </div>
              </button>
            </div>
          </section>
        </div>
      </article>

      <article class="surface-card section-card">
        <div class="section-header">
          <div>
            <p class="eyebrow">{{ translate('skills.details') }}</p>
            <h3>{{ props.state.selectedSkillInfo.value?.name || translate('skills.title') }}</h3>
          </div>
          <span
            v-if="props.state.selectedSkillInfo.value"
            class="status-chip"
            :data-tone="props.state.selectedSkillInfo.value.canDelete ? 'success' : 'neutral'"
          >
            {{ props.state.selectedSkillInfo.value.canDelete ? translate('skills.deletable') : translate('skills.readOnly') }}
          </span>
        </div>

        <div v-if="props.state.detailLoading.value" class="empty-state">
          {{ translate('common.inProgress') }}
        </div>

        <div v-else-if="props.state.selectedSkillInfo.value" class="page-stack">
          <p class="supporting-text">{{ props.state.selectedSkillInfo.value.description }}</p>

          <dl class="detail-metadata-grid">
            <div class="detail-metadata-card">
              <dt>{{ translate('skills.source') }}</dt>
              <dd :title="props.state.selectedSkillInfo.value.source">{{ props.state.selectedSkillInfo.value.source }}</dd>
            </div>
            <div class="detail-metadata-card detail-metadata-card--full">
              <dt>{{ translate('skills.path') }}</dt>
              <dd :title="props.state.selectedSkillInfo.value.filePath">{{ props.state.selectedSkillInfo.value.filePath }}</dd>
            </div>
          </dl>

          <section class="management-panel">
            <span class="card-label">{{ translate('skills.requirements') }}</span>
            <div v-if="props.state.selectedSkillInfo.value.requirements.bins.length === 0
              && props.state.selectedSkillInfo.value.requirements.anyBins.length === 0
              && props.state.selectedSkillInfo.value.requirements.env.length === 0
              && props.state.selectedSkillInfo.value.requirements.config.length === 0
              && props.state.selectedSkillInfo.value.requirements.os.length === 0" class="empty-state">
              {{ translate('skills.noRequirements') }}
            </div>
            <ul v-else class="detail-list">
              <li v-for="bin in props.state.selectedSkillInfo.value.requirements.bins" :key="`bin-${bin}`">{{ bin }}</li>
              <li v-for="envVar in props.state.selectedSkillInfo.value.requirements.env" :key="`env-${envVar}`">{{ envVar }}</li>
              <li v-for="config in props.state.selectedSkillInfo.value.requirements.config" :key="`config-${config}`">{{ config }}</li>
              <li v-for="os in props.state.selectedSkillInfo.value.requirements.os" :key="`os-${os}`">{{ os }}</li>
            </ul>
          </section>

          <section class="management-panel">
            <span class="card-label">{{ translate('skills.installHints') }}</span>
            <div v-if="props.state.selectedSkillInfo.value.install.length === 0" class="empty-state">
              {{ translate('skills.noInstallHints') }}
            </div>
            <ul v-else class="detail-list">
              <li v-for="hint in props.state.selectedSkillInfo.value.install" :key="hint.id">{{ hint.label }}</li>
            </ul>
          </section>

          <div class="button-row button-row--end">
            <button
              v-if="props.state.selectedSkillInfo.value.canDelete"
              class="ghost-button"
              type="button"
              :disabled="props.state.mutationBusy.value"
              @click="deleteSelectedSkill"
            >
              {{ translate('skills.delete') }}
            </button>
          </div>
        </div>

        <div v-else class="empty-state">
          {{ translate('skills.openDetailPrompt') }}
        </div>
      </article>
    </div>

    <p v-if="props.state.error.value" class="error-banner">{{ props.state.error.value }}</p>

    <Teleport to="body">
      <div v-if="installOpen" class="modal-backdrop">
        <section class="modal-card surface-card" role="dialog" aria-modal="true" aria-labelledby="install-skill-title">
          <div class="section-header">
            <div>
              <p class="eyebrow">{{ translate('skills.eyebrow') }}</p>
              <h3 id="install-skill-title">{{ translate('skills.createTitle') }}</h3>
            </div>
            <span class="status-chip" data-tone="active">
              {{ translate('skills.title') }}
            </span>
          </div>

          <div class="form-grid">
            <label class="field field--span">
              <span>{{ translate('skills.searchLabel') }}</span>
              <input v-model="installDraft.query" :placeholder="translate('skills.searchPlaceholder')" />
            </label>
            <label class="field">
              <span>{{ translate('skills.location') }}</span>
              <select v-model="installDraft.location" class="field-select">
                <option value="workspaceLocal" :disabled="!canInstallToWorkspace">{{ translate('skills.workspaceLocation') }}</option>
                <option value="sharedLocal" :disabled="!canInstallToShared">{{ translate('skills.sharedLocation') }}</option>
              </select>
            </label>
          </div>

          <div class="button-row button-row--end">
            <button class="secondary-button" type="button" :disabled="props.state.searchBusy.value || props.state.mutationBusy.value" @click="closeInstallModal">
              {{ translate('common.close') }}
            </button>
            <button class="primary-button" type="button" :disabled="!canSearch || props.state.searchBusy.value || props.state.mutationBusy.value" @click="submitSkillSearch">
              {{ translate('skills.searchAction') }}
            </button>
          </div>

          <section class="management-panel">
            <div class="section-header">
              <div>
                <span class="card-label">{{ translate('skills.searchResults') }}</span>
              </div>
              <span v-if="props.state.searchBusy.value" class="status-chip" data-tone="active">
                {{ translate('common.inProgress') }}
              </span>
            </div>

            <div v-if="props.state.searchBusy.value" class="empty-state">
              {{ translate('common.inProgress') }}
            </div>
            <div v-else-if="!hasSearched" class="empty-state">
              {{ translate('skills.searchPrompt') }}
            </div>
            <div v-else-if="props.state.searchResults.value.length === 0" class="empty-state">
              {{ translate('skills.noSearchResults') }}
            </div>
            <div v-else class="management-list">
              <div v-for="result in props.state.searchResults.value" :key="result.slug" class="management-row management-row--static">
                <div class="management-row-main">
                  <strong>{{ result.title }}</strong>
                  <span>{{ result.slug }}<template v-if="result.score !== null"> · {{ result.score.toFixed(3) }}</template></span>
                </div>
                <div class="management-row-meta">
                  <button class="primary-button" type="button" :disabled="props.state.mutationBusy.value" @click="installSearchResult(result)">
                    {{ translate('skills.createSubmit') }}
                  </button>
                </div>
              </div>
            </div>
          </section>
        </section>
      </div>
    </Teleport>
  </section>
</template>
