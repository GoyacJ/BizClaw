<script setup lang="ts">
import type { Ref } from 'vue'

import { formatAppTime, translate } from '@/lib/i18n'
import { operationStepLabel, operationTaskPhaseLabel, type OperationsSummary } from '@/lib/runtime-view'
import type {
  EnvironmentSnapshot,
  OperationEvent,
  OperationTaskSnapshot,
  WindowsDiscovery,
} from '@/types'

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
  installWslCli: () => Promise<void> | void
  checkForUpdates: () => Promise<void> | void
  updateCli: () => Promise<void> | void
  stopOperation: () => Promise<void> | void
  launchManualInstall: () => Promise<void> | void
}

const props = defineProps<{
  state: InstallSectionState
}>()

function detectionPhaseLabel(snapshot: EnvironmentSnapshot | null) {
  const phase = snapshot?.windowsDiscovery?.phase
  if (phase === 'ready') {
    return translate('install.detectionReady')
  }
  if (snapshot?.os === 'windows') {
    return translate('install.detectionPending')
  }
  return translate('common.notDetected')
}

function nativeSummary(discovery: WindowsDiscovery | null | undefined) {
  if (!discovery) {
    return translate('common.notDetected')
  }
  if (discovery.native.openclawInstalled) {
    return discovery.native.openclawVersion ?? translate('state.openclawReady')
  }
  const missing = [
    !discovery.native.sshInstalled ? 'SSH' : null,
    !discovery.native.nodeInstalled ? 'Node.js' : null,
    !discovery.native.gitInstalled ? 'Git' : null,
  ].filter(Boolean)
  return missing.length > 0
    ? translate('install.pendingDependencies', { items: missing.join(' / ') })
    : translate('overview.notInstalled')
}

function wslSummary(discovery: WindowsDiscovery | null | undefined) {
  if (!discovery) {
    return translate('common.notDetected')
  }
  if (!discovery.wsl.status?.ready) {
    return translate('install.wslWaiting')
  }
  if (discovery.wsl.openclawInstalled) {
    return discovery.wsl.openclawVersion ?? translate('state.openclawReady')
  }
  return discovery.wsl.sshInstalled
    ? translate('overview.notInstalled')
    : translate('install.pendingDependencies', { items: 'SSH' })
}
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
        <div v-if="props.state.environment.value?.os === 'windows'" class="support-tile">
          <span>{{ translate('install.detectionPhase') }}</span>
          <strong>{{ detectionPhaseLabel(props.state.environment.value) }}</strong>
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
        <div v-if="props.state.environment.value?.os === 'windows'" class="support-tile">
          <span>{{ translate('install.windowsNativeStatus') }}</span>
          <strong>{{ nativeSummary(props.state.environment.value?.windowsDiscovery) }}</strong>
        </div>
        <div v-if="props.state.environment.value?.os === 'windows'" class="support-tile">
          <span>{{ translate('install.windowsWslStatus') }}</span>
          <strong>{{ wslSummary(props.state.environment.value?.windowsDiscovery) }}</strong>
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
          v-if="props.state.environment.value?.os === 'windows'"
          class="secondary-button"
          :disabled="props.state.operationTask.value.phase === 'running' || props.state.operationTask.value.phase === 'cancelling'"
          @click="props.state.installWslCli"
        >
          {{ translate('install.installToWsl') }}
        </button>
        <button
          class="secondary-button"
          :disabled="props.state.installBusyAction.value === 'check-update' || props.state.operationTask.value.phase === 'running' || props.state.operationTask.value.phase === 'cancelling'"
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
