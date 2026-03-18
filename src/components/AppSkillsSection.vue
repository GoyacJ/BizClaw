<script setup lang="ts">
import { computed, reactive, type Ref } from 'vue'

import { translate } from '@/lib/i18n'
import type {
  CreateLocalSkillRequest,
  OpenClawSkillInfo,
  OpenClawSkillInventory,
  OpenClawSkillLocationKind,
  OpenClawSkillSummary,
  OpenClawSkillCheckReport,
} from '@/types'

interface SkillsSectionState {
  inventory: Ref<OpenClawSkillInventory>
  checkReport: Ref<OpenClawSkillCheckReport>
  selectedSkillName: Ref<string | null>
  selectedSkillInfo: Ref<OpenClawSkillInfo | null>
  loading: Ref<boolean>
  detailLoading: Ref<boolean>
  mutationBusy: Ref<boolean>
  error: Ref<string | null>
  refreshSkills: () => Promise<unknown> | unknown
  selectSkill: (name: string) => Promise<unknown> | unknown
  createLocalSkill: (request: CreateLocalSkillRequest) => Promise<unknown> | unknown
  deleteLocalSkill: (name: string) => Promise<unknown> | unknown
}

const props = defineProps<{
  state: SkillsSectionState
}>()

const createDraft = reactive<CreateLocalSkillRequest>({
  name: '',
  location: 'workspaceLocal',
})

const groupedSkills = computed(() => {
  const groups: Array<{ key: OpenClawSkillLocationKind, label: string, skills: OpenClawSkillSummary[] }> = [
    { key: 'workspaceLocal', label: translate('skills.workspaceLocal'), skills: [] },
    { key: 'sharedLocal', label: translate('skills.sharedLocal'), skills: [] },
    { key: 'bundled', label: translate('skills.bundled'), skills: [] },
    { key: 'external', label: translate('skills.external'), skills: [] },
  ]

  for (const skill of props.state.inventory.value.skills) {
    const group = groups.find((entry) => entry.key === skill.locationKind)
    group?.skills.push(skill)
  }

  return groups.filter((group) => group.skills.length > 0)
})

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

async function submitCreateSkill() {
  const name = createDraft.name.trim()
  if (!name) {
    return
  }

  await props.state.createLocalSkill({
    name,
    location: createDraft.location,
  })
  createDraft.name = ''
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
        <button class="secondary-button" type="button" :disabled="props.state.loading.value" @click="props.state.refreshSkills">
          {{ translate('skills.refresh') }}
        </button>
      </div>

      <div class="management-panel">
        <div class="section-header">
          <div>
            <span class="card-label">{{ translate('skills.createTitle') }}</span>
          </div>
        </div>
        <div class="form-grid">
          <label class="field">
            <span>{{ translate('skills.name') }}</span>
            <input v-model="createDraft.name" :placeholder="translate('skills.placeholderName')" />
          </label>
          <label class="field">
            <span>{{ translate('skills.location') }}</span>
            <select v-model="createDraft.location" class="field-select">
              <option value="workspaceLocal">{{ translate('skills.workspaceLocation') }}</option>
              <option value="sharedLocal">{{ translate('skills.sharedLocation') }}</option>
            </select>
          </label>
        </div>
        <div class="button-row button-row--end">
          <button class="primary-button" type="button" :disabled="props.state.mutationBusy.value" @click="submitCreateSkill">
            {{ translate('skills.createSubmit') }}
          </button>
        </div>
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
                @click="props.state.selectSkill(skill.name)"
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

          <div class="support-grid support-grid--metrics">
            <div class="support-tile">
              <span>{{ translate('skills.source') }}</span>
              <strong>{{ props.state.selectedSkillInfo.value.source }}</strong>
            </div>
            <div class="support-tile">
              <span>{{ translate('skills.path') }}</span>
              <strong>{{ props.state.selectedSkillInfo.value.filePath }}</strong>
            </div>
          </div>

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
  </section>
</template>
