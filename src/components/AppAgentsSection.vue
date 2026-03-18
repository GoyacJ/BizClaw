<script setup lang="ts">
import { computed, reactive, ref, watch, type Ref } from 'vue'

import { translate } from '@/lib/i18n'
import type {
  CreateOpenClawAgentRequest,
  OpenClawAgentBinding,
  OpenClawAgentSummary,
  UpdateOpenClawAgentIdentityRequest,
} from '@/types'

interface AgentsSectionState {
  agents: Ref<OpenClawAgentSummary[]>
  bindings: Ref<OpenClawAgentBinding[]>
  bindingsLoading: Ref<boolean>
  selectedAgentId: Ref<string | null>
  loading: Ref<boolean>
  mutationBusy: Ref<boolean>
  error: Ref<string | null>
  refreshAgents: () => Promise<unknown> | unknown
  selectAgent: (agentId: string) => Promise<unknown> | unknown
  createAgent: (request: CreateOpenClawAgentRequest) => Promise<unknown> | unknown
  updateAgentIdentity: (request: UpdateOpenClawAgentIdentityRequest) => Promise<unknown> | unknown
  deleteAgent: (agentId: string) => Promise<unknown> | unknown
  addAgentBindings: (agentId: string, bindings: string[]) => Promise<unknown> | unknown
  removeAgentBindings: (agentId: string, bindings: string[]) => Promise<unknown> | unknown
  clearAgentBindings: (agentId: string) => Promise<unknown> | unknown
}

const props = defineProps<{
  state: AgentsSectionState
}>()

const createOpen = ref(false)
const bindingInput = ref('')
const createDraft = reactive({
  name: '',
  workspace: '',
  model: '',
  bindingsText: '',
})
const identityDraft = reactive({
  name: '',
  emoji: '',
  theme: '',
  avatar: '',
})

const selectedAgent = computed(() => (
  props.state.agents.value.find((agent) => agent.id === props.state.selectedAgentId.value) ?? null
))

const selectedBindings = computed(() => (
  props.state.bindings.value.filter((binding) => binding.agentId === selectedAgent.value?.id)
))

watch(selectedAgent, (agent) => {
  identityDraft.name = agent?.identityName ?? ''
  identityDraft.emoji = agent?.identityEmoji ?? ''
  identityDraft.theme = ''
  identityDraft.avatar = ''
}, { immediate: true })

function resetCreateDraft() {
  createDraft.name = ''
  createDraft.workspace = ''
  createDraft.model = ''
  createDraft.bindingsText = ''
}

function closeCreateModal() {
  createOpen.value = false
}

function parseBindings(raw: string) {
  return raw
    .split(/\r?\n/)
    .map((value) => value.trim())
    .filter(Boolean)
}

async function submitCreateAgent() {
  const request: CreateOpenClawAgentRequest = {
    name: createDraft.name.trim(),
    workspace: createDraft.workspace.trim(),
    model: createDraft.model.trim() || null,
    bindings: parseBindings(createDraft.bindingsText),
  }
  if (!request.name || !request.workspace) {
    return
  }

  await props.state.createAgent(request)
  resetCreateDraft()
  closeCreateModal()
}

async function saveIdentity() {
  if (!selectedAgent.value) {
    return
  }

  await props.state.updateAgentIdentity({
    agentId: selectedAgent.value.id,
    name: identityDraft.name.trim() || null,
    emoji: identityDraft.emoji.trim() || null,
    theme: identityDraft.theme.trim() || null,
    avatar: identityDraft.avatar.trim() || null,
  })
}

async function appendBindings() {
  if (!selectedAgent.value) {
    return
  }

  const bindings = parseBindings(bindingInput.value)
  if (bindings.length === 0) {
    return
  }

  await props.state.addAgentBindings(selectedAgent.value.id, bindings)
  bindingInput.value = ''
}

async function removeBinding(binding: OpenClawAgentBinding) {
  const spec = binding.accountId ? `${binding.channel}:${binding.accountId}` : binding.channel
  await props.state.removeAgentBindings(binding.agentId, [spec])
}

async function deleteSelectedAgent() {
  if (!selectedAgent.value) {
    return
  }

  if (globalThis.confirm && !globalThis.confirm(translate('agents.deleteConfirm', { id: selectedAgent.value.id }))) {
    return
  }

  await props.state.deleteAgent(selectedAgent.value.id)
}
</script>

<template>
  <section class="page-stack">
    <article class="surface-card section-card">
      <div class="section-header">
        <div>
          <p class="eyebrow">{{ translate('agents.eyebrow') }}</p>
          <h3>{{ translate('agents.title') }}</h3>
        </div>
        <span class="status-chip" :data-tone="props.state.loading.value ? 'active' : 'neutral'">
          {{ props.state.agents.value.length }}
        </span>
      </div>
      <p class="supporting-text">{{ translate('agents.detail') }}</p>
      <div class="button-row">
        <button class="primary-button" type="button" @click="createOpen = true">
          {{ translate('agents.create') }}
        </button>
        <button class="secondary-button" type="button" :disabled="props.state.loading.value" @click="props.state.refreshAgents">
          {{ translate('agents.refresh') }}
        </button>
      </div>
    </article>

    <div class="management-layout">
      <article class="surface-card section-card">
        <div class="section-header">
          <div>
            <p class="eyebrow">{{ translate('agents.identity') }}</p>
            <h3>{{ translate('agents.detailTitle') }}</h3>
          </div>
        </div>

        <div v-if="props.state.agents.value.length === 0" class="empty-state">
          {{ translate('agents.empty') }}
        </div>

        <div v-else class="management-list">
          <button
            v-for="agent in props.state.agents.value"
            :key="agent.id"
            class="management-row"
            :data-active="String(agent.id === props.state.selectedAgentId.value)"
            type="button"
            @click="void props.state.selectAgent(agent.id)"
          >
            <div class="management-row-main">
              <strong>{{ agent.id }}</strong>
              <span>{{ agent.identityEmoji }} {{ agent.identityName || agent.name || agent.id }}</span>
            </div>
            <div class="management-row-meta">
              <span class="status-chip" :data-tone="agent.isDefault ? 'success' : 'neutral'">
                {{ agent.isDefault ? translate('agents.defaultBadge') : `${agent.bindings} ${translate('agents.bindings')}` }}
              </span>
            </div>
          </button>
        </div>
      </article>

      <article class="surface-card section-card" v-if="selectedAgent">
        <div class="section-header">
          <div>
            <p class="eyebrow">{{ selectedAgent.id }}</p>
            <h3>{{ selectedAgent.identityEmoji }} {{ selectedAgent.identityName || selectedAgent.name || selectedAgent.id }}</h3>
          </div>
          <span class="status-chip" :data-tone="selectedAgent.isDefault ? 'success' : 'neutral'">
            {{ selectedAgent.isDefault ? translate('agents.defaultBadge') : `${selectedAgent.bindings} ${translate('agents.bindings')}` }}
          </span>
        </div>

        <dl class="detail-metadata-grid">
          <div class="detail-metadata-card detail-metadata-card--full">
            <dt>{{ translate('agents.workspace') }}</dt>
            <dd :title="selectedAgent.workspace">{{ selectedAgent.workspace }}</dd>
          </div>
          <div class="detail-metadata-card detail-metadata-card--full">
            <dt>{{ translate('agents.agentDir') }}</dt>
            <dd :title="selectedAgent.agentDir">{{ selectedAgent.agentDir }}</dd>
          </div>
          <div class="detail-metadata-card">
            <dt>{{ translate('agents.model') }}</dt>
            <dd :title="selectedAgent.model || 'auto'">{{ selectedAgent.model || 'auto' }}</dd>
          </div>
          <div class="detail-metadata-card">
            <dt>{{ translate('agents.bindings') }}</dt>
            <dd>{{ selectedBindings.length }}</dd>
          </div>
        </dl>

        <p class="helper-text">{{ translate('agents.readOnlyHint') }}</p>

        <div class="form-grid">
          <label class="field">
            <span>{{ translate('agents.identityName') }}</span>
            <input v-model="identityDraft.name" />
          </label>
          <label class="field">
            <span>{{ translate('agents.identityEmoji') }}</span>
            <input v-model="identityDraft.emoji" />
          </label>
          <label class="field">
            <span>{{ translate('agents.identityTheme') }}</span>
            <input v-model="identityDraft.theme" />
          </label>
          <label class="field">
            <span>{{ translate('agents.identityAvatar') }}</span>
            <input v-model="identityDraft.avatar" />
          </label>
        </div>

        <div class="button-row">
          <button class="secondary-button" type="button" :disabled="props.state.mutationBusy.value" @click="saveIdentity">
            {{ translate('agents.saveIdentity') }}
          </button>
          <button class="ghost-button" type="button" :disabled="props.state.mutationBusy.value" @click="deleteSelectedAgent">
            {{ translate('agents.delete') }}
          </button>
        </div>

        <div class="management-panel">
          <div class="section-header">
            <div>
              <span class="card-label">{{ translate('agents.bindings') }}</span>
            </div>
            <span v-if="props.state.bindingsLoading.value" class="status-chip" data-tone="active">
              {{ translate('common.inProgress') }}
            </span>
          </div>

          <div v-if="props.state.bindingsLoading.value" class="empty-state">
            {{ translate('common.inProgress') }}
          </div>

          <div v-else-if="selectedBindings.length === 0" class="empty-state">
            {{ translate('agents.noBindings') }}
          </div>

          <div v-else class="management-list">
            <div v-for="binding in selectedBindings" :key="binding.description" class="management-row management-row--static">
              <div class="management-row-main">
                <strong>{{ binding.channel }}</strong>
                <span>{{ binding.description }}</span>
              </div>
              <div class="management-row-meta">
                <button class="ghost-button" type="button" :disabled="props.state.mutationBusy.value" @click="removeBinding(binding)">
                  {{ translate('skills.delete') }}
                </button>
              </div>
            </div>
          </div>

          <label class="field field--span">
            <span>{{ translate('agents.bindingInput') }}</span>
            <textarea v-model="bindingInput" rows="3" />
          </label>
          <p class="helper-text">{{ translate('agents.bindingHelp') }}</p>
          <div class="button-row">
            <button class="secondary-button" type="button" :disabled="props.state.mutationBusy.value" @click="appendBindings">
              {{ translate('agents.addBindings') }}
            </button>
            <button class="ghost-button" type="button" :disabled="props.state.mutationBusy.value" @click="props.state.clearAgentBindings(selectedAgent.id)">
              {{ translate('agents.clearBindings') }}
            </button>
          </div>
        </div>
      </article>
    </div>

    <p v-if="props.state.error.value" class="error-banner">{{ props.state.error.value }}</p>

    <Teleport to="body">
      <div v-if="createOpen" class="modal-backdrop">
        <section class="modal-card surface-card" role="dialog" aria-modal="true" aria-labelledby="create-agent-title">
          <div class="section-header">
            <div>
              <p class="eyebrow">{{ translate('agents.eyebrow') }}</p>
              <h3 id="create-agent-title">{{ translate('agents.createTitle') }}</h3>
            </div>
            <span class="status-chip" data-tone="active">
              {{ translate('agents.title') }}
            </span>
          </div>
          <div class="form-grid">
            <label class="field">
              <span>{{ translate('agents.name') }}</span>
              <input v-model="createDraft.name" :placeholder="translate('agents.placeholderName')" />
            </label>
            <label class="field">
              <span>{{ translate('agents.workspacePath') }}</span>
              <input v-model="createDraft.workspace" :placeholder="translate('agents.placeholderWorkspace')" />
            </label>
            <label class="field field--span">
              <span>{{ translate('agents.modelOptional') }}</span>
              <input v-model="createDraft.model" placeholder="minimax-cn/MiniMax-M2.5" />
            </label>
            <label class="field field--span">
              <span>{{ translate('agents.bindingsOptional') }}</span>
              <textarea v-model="createDraft.bindingsText" rows="4" />
            </label>
          </div>
          <div class="button-row button-row--end">
            <button class="secondary-button" type="button" :disabled="props.state.mutationBusy.value" @click="closeCreateModal">
              {{ translate('common.close') }}
            </button>
            <button class="primary-button" type="button" :disabled="props.state.mutationBusy.value" @click="submitCreateAgent">
              {{ translate('agents.createSubmit') }}
            </button>
          </div>
        </section>
      </div>
    </Teleport>
  </section>
</template>
