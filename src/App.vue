<script setup lang="ts">
import { computed } from 'vue'

import AppConnectionSection from './components/AppConnectionSection.vue'
import AppInstallSection from './components/AppInstallSection.vue'
import AppOverviewSection from './components/AppOverviewSection.vue'
import AppRuntimeLogsSection from './components/AppRuntimeLogsSection.vue'
import AppSidebar from './components/AppSidebar.vue'
import AppStatusBar from './components/AppStatusBar.vue'
import AppSettingsSection from './components/AppSettingsSection.vue'
import AppWorkspaceHeader from './components/AppWorkspaceHeader.vue'
import { translate } from '@/lib/i18n'
import { useAppModel } from '@/lib/use-app-model'
import type { ConnectionTestModalStep } from '@/types'

const {
  activeSection,
  advancedOpen,
  bizclawUpdate,
  bizclawUpdateActionLabel,
  bizclawUpdateBlockedReason,
  bizclawUpdateDetail,
  bizclawUpdatePrimaryAction,
  bizclawUpdateTone,
  checkBizClawUpdates,
  canSaveProfile,
  canStartHostedRuntime,
  canStopOperation,
  canTestConnection,
  checkForUpdates,
  closeConnectionTestModal,
  companyProfile,
  connectDisabledReason,
  connectionTestBusy,
  connectionTestCloseDisabled,
  connectionTestDisabledReason,
  connectionTestModal,
  deferBizClawRestart,
  environment,
  installBusyAction,
  installCli,
  installBizClawUpdate,
  launchManualInstall,
  logs,
  manualInstallBusy,
  operationError,
  operationEvents,
  operationHeadline,
  operationsSummary,
  operationTask,
  overviewCards,
  platformLabel,
  profileError,
  refreshEnvironment,
  runtimeError,
  runtimeStartBusy,
  runtimeStatus,
  runtimeStopBusy,
  restartBizClaw,
  saveAndTest,
  saveBusy,
  saveOnly,
  sshPasswordInput,
  sshStateLabel,
  startHostedRuntime,
  statusItems,
  stopOperation,
  stopHostedRuntime,
  setLocale,
  setTheme,
  targetProfile,
  tokenInput,
  tokenStateLabel,
  tokenStateToneValue,
  uiPreferences,
  updateCli,
  userProfile,
} = useAppModel()

const sections = computed(() => ([
  { key: 'overview', label: translate('nav.overview') },
  { key: 'install', label: translate('nav.install') },
  { key: 'connection', label: translate('nav.connection') },
  { key: 'runtime', label: translate('nav.runtime') },
  { key: 'settings', label: translate('nav.settings') },
] as const))

const sectionTitle = computed(() => sections.value.find((item) => item.key === activeSection.value)?.label ?? translate('nav.overview'))

const selectSection = (sectionKey: string) => {
  activeSection.value = sectionKey as typeof activeSection.value
}

const goInstall = () => {
  activeSection.value = 'install'
}

const goConnection = () => {
  activeSection.value = 'connection'
}

const installSectionState = {
  environment,
  operationsSummary,
  platformLabel,
  sshStateLabel,
  operationTask,
  installBusyAction,
  canStopOperation,
  operationHeadline,
  operationError,
  operationEvents,
  installCli,
  checkForUpdates,
  updateCli,
  stopOperation,
  launchManualInstall,
}

const connectionSectionState = {
  advancedOpen,
  canSaveProfile,
  canTestConnection,
  canStartHostedRuntime,
  companyProfile,
  userProfile,
  targetProfile,
  tokenInput,
  sshPasswordInput,
  tokenStateToneValue,
  tokenStateLabel,
  profileError,
  connectionTestDisabledReason,
  saveOnly,
  saveAndTest,
  runtimeStatus,
  platformLabel,
  runtimeError,
  runtimeStartBusy,
  runtimeStopBusy,
  connectDisabledReason,
  startHostedRuntime,
  stopHostedRuntime,
}

const settingsSectionState = {
  uiPreferences,
  setTheme,
  setLocale,
  bizclawUpdate,
  bizclawUpdateActionLabel,
  bizclawUpdateTone,
  bizclawUpdateDetail,
  bizclawUpdateBlockedReason,
  bizclawUpdatePrimaryAction,
  checkBizClawUpdates,
  installBizClawUpdate,
  restartBizClaw,
  deferBizClawRestart,
}

const busyLabel = computed(() => {
  if (operationTask.value.phase === 'running' && operationTask.value.kind === 'install') {
    return translate('busy.installing')
  }
  if (operationTask.value.phase === 'running' && operationTask.value.kind === 'update') {
    return translate('busy.updating')
  }
  if (operationTask.value.phase === 'cancelling') {
    return translate('busy.stopping')
  }
  if (installBusyAction.value === 'check-update') {
    return translate('busy.checkingUpdate')
  }
  if (bizclawUpdate.value.phase === 'checking') {
    return translate('busy.bizclawChecking')
  }
  if (bizclawUpdate.value.phase === 'downloading') {
    return translate('busy.bizclawDownloading')
  }
  if (bizclawUpdate.value.phase === 'installing') {
    return translate('busy.bizclawInstalling')
  }
  if (manualInstallBusy.value) {
    return translate('busy.openingDocs')
  }
  if (saveBusy.value) {
    return translate('busy.saving')
  }
  if (connectionTestBusy.value) {
    return translate('busy.testing')
  }
  if (runtimeStartBusy.value) {
    return translate('busy.starting')
  }
  if (runtimeStopBusy.value) {
    return translate('busy.stopping')
  }
  return ''
})

function connectionStepTone(status: ConnectionTestModalStep['status']) {
  switch (status) {
    case 'success':
      return 'success'
    case 'error':
      return 'error'
    case 'running':
      return 'active'
    default:
      return 'neutral'
  }
}

function connectionStepLabel(status: ConnectionTestModalStep['status']) {
  switch (status) {
    case 'success':
      return translate('common.success')
    case 'error':
      return translate('common.failure')
    case 'running':
      return translate('common.inProgress')
    default:
      return translate('common.pending')
  }
}
</script>

<template>
  <main class="ops-shell">
    <AppSidebar
      :sections="sections"
      :active-section="activeSection"
      @select-section="selectSection"
    />

    <section class="workspace">
      <AppWorkspaceHeader
        :section-title="sectionTitle"
        :runtime-phase="runtimeStatus.phase"
        :busy-label="busyLabel"
        @refresh="refreshEnvironment"
      />

      <KeepAlive>
        <AppOverviewSection
          v-if="activeSection === 'overview'"
          :operations-summary="operationsSummary"
          :overview-cards="overviewCards"
          :go-install="goInstall"
          :go-connection="goConnection"
        />

        <AppInstallSection
          v-else-if="activeSection === 'install'"
          :state="installSectionState"
        />

        <AppConnectionSection
          v-else-if="activeSection === 'connection'"
          :state="connectionSectionState"
        />

        <AppSettingsSection
          v-else-if="activeSection === 'settings'"
          :state="settingsSectionState"
        />

        <AppRuntimeLogsSection
          v-else
          :logs="logs"
        />
      </KeepAlive>
    </section>

    <AppStatusBar :status-items="statusItems" />
  </main>

  <Teleport to="body">
    <div v-if="connectionTestModal.open" class="modal-backdrop">
      <section class="modal-card surface-card" role="dialog" aria-modal="true" aria-labelledby="connection-test-title">
        <div class="section-header">
          <div>
            <p class="eyebrow">{{ translate('connectionTest.modalEyebrow') }}</p>
            <h3 id="connection-test-title">{{ translate('connectionTest.modalTitle') }}</h3>
          </div>
          <span class="status-chip" :data-tone="connectionTestModal.phase === 'success' ? 'success' : connectionTestModal.phase === 'error' ? 'error' : 'active'">
            {{
              connectionTestModal.phase === 'success'
                ? translate('connectionTest.pass')
                : connectionTestModal.phase === 'error'
                  ? translate('connectionTest.fail')
                  : translate('connectionTest.running')
            }}
          </span>
        </div>

        <p class="supporting-text">{{ connectionTestModal.summary }}</p>

        <ol class="connection-test-list">
          <li v-for="step in connectionTestModal.steps" :key="step.step" class="connection-test-item">
            <div>
              <strong>{{ step.label }}</strong>
              <p class="supporting-text">{{ step.message || translate('connectionTest.waiting') }}</p>
            </div>
            <span class="status-chip" :data-tone="connectionStepTone(step.status)">
              {{ connectionStepLabel(step.status) }}
            </span>
          </li>
        </ol>

        <div
          v-if="connectionTestModal.result && (connectionTestModal.result.stdout || connectionTestModal.result.stderr)"
          class="connection-test-output"
        >
          <div v-if="connectionTestModal.result.stdout">
            <span class="card-label">{{ translate('common.stdout') }}</span>
            <pre>{{ connectionTestModal.result.stdout }}</pre>
          </div>
          <div v-if="connectionTestModal.result.stderr">
            <span class="card-label">{{ translate('common.stderr') }}</span>
            <pre>{{ connectionTestModal.result.stderr }}</pre>
          </div>
        </div>

        <div class="button-row button-row--end">
          <button class="primary-button" :disabled="connectionTestCloseDisabled" @click="closeConnectionTestModal">
            {{ translate('common.close') }}
          </button>
        </div>
      </section>
    </div>
  </Teleport>
</template>
