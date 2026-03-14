<script setup lang="ts">
import type { Ref } from 'vue'

import { formatAppTime, translate } from '@/lib/i18n'
import { operationStepLabel, operationTaskPhaseLabel, type OperationsSummary } from '@/lib/runtime-view'
import type { EnvironmentSnapshot, OperationEvent, OperationTaskSnapshot } from '@/types'

interface InstallSectionState {
  environment: Ref<EnvironmentSnapshot | null>
  operationsSummary: Ref<OperationsSummary>
  platformLabel: Ref<string>
  sshStateLabel: Ref<string>
  operationTask: Ref<OperationTaskSnapshot>
  installBusyAction: Ref<'check-update' | null>
  canStopOperation: Ref<boolean>
  operationHeadline: Ref<string>
  operationError: Ref<string | null>
  operationEvents: Ref<OperationEvent[]>
  installCli: () => Promise<void> | void
  checkForUpdates: () => Promise<void> | void
  updateCli: () => Promise<void> | void
  stopOperation: () => Promise<void> | void
  launchManualInstall: () => Promise<void> | void
}

const props = defineProps<{
  state: InstallSectionState
}>()
</script>

<template>
  <section class="page-stack">
    <article class="surface-card section-card">
      <div class="section-header">
        <div>
          <p class="eyebrow">{{ translate('install.targetEyebrow') }}</p>
          <h3>{{ translate('install.targetTitle') }}</h3>
        </div>
        <span class="status-chip" :data-tone="props.state.operationsSummary.value.tone">
          {{ props.state.operationsSummary.value.title }}
        </span>
      </div>
      <p class="supporting-text">{{ props.state.operationsSummary.value.detail }}</p>
      <div class="support-grid">
        <div class="support-tile">
          <span>{{ translate('install.runtimeTarget') }}</span>
          <strong>{{ props.state.platformLabel.value }}</strong>
        </div>
        <div class="support-tile">
          <span>{{ translate('install.currentVersion') }}</span>
          <strong>{{ props.state.environment.value?.openclawVersion ?? translate('overview.notInstalled') }}</strong>
        </div>
        <div class="support-tile">
          <span>{{ translate('install.latestVersion') }}</span>
          <strong>{{ props.state.environment.value?.latestOpenclawVersion ?? translate('common.notDetected') }}</strong>
        </div>
        <div class="support-tile">
          <span>{{ translate('install.openSsh') }}</span>
          <strong>{{ props.state.sshStateLabel.value }}</strong>
        </div>
      </div>
      <div class="button-row">
        <button
          class="primary-button"
          :disabled="props.state.operationTask.value.phase === 'running' || props.state.operationTask.value.phase === 'cancelling'"
          @click="props.state.installCli"
        >
          {{ translate('install.installOpenClaw') }}
        </button>
        <button
          class="secondary-button"
          :disabled="props.state.installBusyAction.value === 'check-update'"
          @click="props.state.checkForUpdates"
        >
          {{ translate('install.checkUpdate') }}
        </button>
        <button
          class="secondary-button"
          :disabled="!props.state.environment.value?.updateAvailable || props.state.operationTask.value.phase === 'running' || props.state.operationTask.value.phase === 'cancelling'"
          @click="props.state.updateCli"
        >
          {{ translate('install.updateOpenClaw') }}
        </button>
        <button
          v-if="props.state.canStopOperation.value"
          class="secondary-button"
          @click="props.state.stopOperation"
        >
          {{ translate('runtime.operationPhase.cancelling') }}
        </button>
        <button class="ghost-button" @click="props.state.launchManualInstall">{{ translate('install.openManual') }}</button>
      </div>
      <p v-if="props.state.operationTask.value.step" class="helper-text">
        {{ translate('install.currentStep', { step: operationStepLabel(props.state.operationTask.value.step), phase: operationTaskPhaseLabel(props.state.operationTask.value.phase) }) }}
      </p>
      <p v-if="props.state.environment.value?.wslStatus?.message" class="helper-text">
        {{ props.state.environment.value.wslStatus.message }}
      </p>
      <p class="helper-text">{{ props.state.operationHeadline.value }}</p>
      <p v-if="props.state.operationError.value" class="error-banner">{{ props.state.operationError.value }}</p>
    </article>

    <article class="surface-card section-card">
      <div class="section-header">
        <div>
          <p class="eyebrow">{{ translate('install.consoleEyebrow') }}</p>
          <h3>{{ translate('install.consoleTitle') }}</h3>
        </div>
        <span
          class="status-chip"
          :data-tone="props.state.operationTask.value.phase === 'error' ? 'error' : props.state.operationTask.value.phase === 'success' ? 'success' : props.state.operationTask.value.phase === 'cancelled' ? 'neutral' : 'active'"
        >
          {{ operationTaskPhaseLabel(props.state.operationTask.value.phase) }}
        </span>
      </div>
      <ol class="operation-list">
        <li v-for="entry in props.state.operationEvents.value" :key="`${entry.timestampMs}-${entry.message}`">
          <time>{{ formatAppTime(entry.timestampMs) }}</time>
          <strong>{{ operationStepLabel(entry.step) }}</strong>
          <span>{{ entry.message }}</span>
        </li>
      </ol>
    </article>
  </section>
</template>
