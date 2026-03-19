<script setup lang="ts">
import type { Ref } from 'vue'

import { translate } from '@/lib/i18n'
import { phaseLabel } from '@/lib/runtime-view'
import type { CompanyProfileDraft, ConnectionTestResult, RuntimeStatus, TargetProfile, UserProfile } from '@/types'

interface ConnectionSectionState {
  advancedOpen: Ref<boolean>
  canSaveProfile: Ref<boolean>
  canTestConnection: Ref<boolean>
  canStartHostedRuntime: Ref<boolean>
  companyProfile: CompanyProfileDraft
  userProfile: UserProfile
  targetProfile: TargetProfile
  tokenInput: Ref<string>
  sshPasswordInput: Ref<string>
  tokenStateToneValue: Ref<string>
  tokenStateLabel: Ref<string>
  profileError: Ref<string | null>
  connectionTestDisabledReason: Ref<string | null>
  connectionTestInlinePhase: Ref<'idle' | 'running' | 'success' | 'error'>
  connectionTestInlineSummary: Ref<string>
  connectionTestInlineResult: Ref<ConnectionTestResult | null>
  connectionTestInlineVisible: Ref<boolean>
  saveOnly: () => Promise<void> | void
  saveAndTest: () => Promise<void> | void
  runtimeStatus: Ref<RuntimeStatus>
  platformLabel: Ref<string>
  runtimeError: Ref<string | null>
  runtimeStartBusy: Ref<boolean>
  runtimeStopBusy: Ref<boolean>
  connectDisabledReason: Ref<string | null>
  startHostedRuntime: () => Promise<void> | void
  stopHostedRuntime: () => Promise<void> | void
}

const props = defineProps<{
  state: ConnectionSectionState
}>()
</script>

<template>
  <section class="page-stack">
    <article class="surface-card section-card">
      <div class="section-header">
        <div>
          <p class="eyebrow">{{ translate('connection.eyebrow') }}</p>
          <h3>{{ translate('connection.title') }}</h3>
        </div>
        <span class="status-chip" :data-tone="props.state.tokenStateToneValue.value">{{ props.state.tokenStateLabel.value }}</span>
      </div>
      <div class="form-grid form-grid--compact">
        <label class="field">
          <span>{{ translate('connection.displayName') }}</span>
          <input v-model="props.state.userProfile.displayName" :placeholder="translate('connection.placeholderName')" />
        </label>
        <label class="field">
          <span>{{ translate('connection.gatewayToken') }}</span>
          <input
            v-model="props.state.tokenInput.value"
            type="password"
            :placeholder="translate('connection.placeholderToken')"
          />
        </label>
        <label class="field">
          <span>{{ translate('connection.sshHost') }}</span>
          <input v-model="props.state.companyProfile.sshHost" :placeholder="translate('connection.placeholderHost')" />
        </label>
        <label class="field">
          <span>{{ translate('connection.sshUser') }}</span>
          <input v-model="props.state.companyProfile.sshUser" :placeholder="translate('connection.placeholderUser')" />
        </label>
      </div>

      <div class="toggle-row">
        <label class="toggle-pill">
          <input v-model="props.state.userProfile.runInBackground" type="checkbox" />
          <span>{{ translate('connection.keepRunning') }}</span>
        </label>
      </div>

      <button class="inline-button" @click="props.state.advancedOpen.value = !props.state.advancedOpen.value">
        {{ props.state.advancedOpen.value ? translate('connection.collapseAdvanced') : translate('connection.expandAdvanced') }}
      </button>

      <div v-if="props.state.advancedOpen.value" class="advanced-grid">
        <label class="field">
          <span>{{ translate('connection.wslDistro') }}</span>
          <input v-model="props.state.targetProfile.wslDistro" placeholder="Ubuntu" />
        </label>
        <label class="field">
          <span>{{ translate('connection.localPort') }}</span>
          <input v-model="props.state.companyProfile.localPort" inputmode="numeric" />
        </label>
        <label class="field">
          <span>{{ translate('connection.remoteBindHost') }}</span>
          <input v-model="props.state.companyProfile.remoteBindHost" />
        </label>
        <label class="field">
          <span>{{ translate('connection.remoteBindPort') }}</span>
          <input v-model="props.state.companyProfile.remoteBindPort" inputmode="numeric" />
        </label>
        <label class="field field--span">
          <span>{{ translate('connection.sshPassword') }}</span>
          <input
            v-model="props.state.sshPasswordInput.value"
            type="password"
            :placeholder="translate('connection.placeholderPassword')"
          />
        </label>
      </div>

      <p class="helper-text">
        {{ translate('connection.helper') }}
      </p>
      <p v-if="props.state.profileError.value" class="error-banner">{{ props.state.profileError.value }}</p>
      <p v-if="props.state.connectionTestDisabledReason.value" class="helper-text">{{ props.state.connectionTestDisabledReason.value }}</p>
      <div class="button-row button-row--end">
        <button class="secondary-button" :disabled="!props.state.canSaveProfile.value" @click="props.state.saveOnly">
          {{ translate('connection.saveOnly') }}
        </button>
        <button class="primary-button" :disabled="!props.state.canTestConnection.value" @click="props.state.saveAndTest">
          {{ translate('connection.saveAndTest') }}
        </button>
      </div>
      <div v-if="props.state.connectionTestInlineVisible.value" class="connection-inline-feedback">
        <div class="section-header">
          <div>
            <p class="eyebrow">{{ translate('connectionTest.modalEyebrow') }}</p>
            <h4>{{ translate('connectionTest.modalTitle') }}</h4>
          </div>
          <span
            class="status-chip"
            :data-tone="props.state.connectionTestInlinePhase.value === 'success' ? 'success' : props.state.connectionTestInlinePhase.value === 'error' ? 'error' : 'active'"
          >
            {{
              props.state.connectionTestInlinePhase.value === 'success'
                ? translate('connectionTest.pass')
                : props.state.connectionTestInlinePhase.value === 'error'
                  ? translate('connectionTest.fail')
                  : translate('connectionTest.running')
            }}
          </span>
        </div>
        <p class="supporting-text">{{ props.state.connectionTestInlineSummary.value }}</p>
        <div
          v-if="props.state.connectionTestInlineResult.value && (props.state.connectionTestInlineResult.value.stdout || props.state.connectionTestInlineResult.value.stderr)"
          class="connection-test-output"
        >
          <div v-if="props.state.connectionTestInlineResult.value.stdout">
            <span class="card-label">{{ translate('common.stdout') }}</span>
            <pre>{{ props.state.connectionTestInlineResult.value.stdout }}</pre>
          </div>
          <div v-if="props.state.connectionTestInlineResult.value.stderr">
            <span class="card-label">{{ translate('common.stderr') }}</span>
            <pre>{{ props.state.connectionTestInlineResult.value.stderr }}</pre>
          </div>
        </div>
      </div>
    </article>

    <article class="surface-card section-card">
      <div class="section-header">
        <div>
          <p class="eyebrow">{{ translate('connection.runtimeEyebrow') }}</p>
          <h3>{{ translate('connection.runtimeTitle') }}</h3>
        </div>
        <span class="status-chip" :data-tone="props.state.runtimeStatus.value.phase === 'running' ? 'success' : 'neutral'">
          {{ phaseLabel(props.state.runtimeStatus.value.phase) }}
        </span>
      </div>
      <div class="support-grid support-grid--metrics">
        <div class="support-tile">
          <span>{{ translate('connection.gateway') }}</span>
          <strong>{{ props.state.runtimeStatus.value.phase === 'running' && props.state.runtimeStatus.value.gatewayConnected ? translate('common.connected') : translate('common.notConnected') }}</strong>
        </div>
        <div class="support-tile">
          <span>{{ translate('connection.ssh') }}</span>
          <strong>{{ props.state.runtimeStatus.value.sshConnected ? translate('common.connected') : translate('common.notConnected') }}</strong>
        </div>
        <div class="support-tile">
          <span>{{ translate('connection.node') }}</span>
          <strong>{{ props.state.runtimeStatus.value.nodeConnected ? translate('common.connected') : translate('common.notConnected') }}</strong>
        </div>
        <div class="support-tile">
          <span>{{ translate('connection.target') }}</span>
          <strong>{{ props.state.platformLabel.value }}</strong>
        </div>
      </div>
      <div class="button-row">
        <button
          class="primary-button"
          :disabled="!props.state.canStartHostedRuntime.value"
          @click="props.state.startHostedRuntime"
        >
          {{ translate('connection.startHosted') }}
        </button>
        <button
          class="secondary-button"
          :disabled="props.state.runtimeStopBusy.value || props.state.runtimeStatus.value.phase !== 'running'"
          @click="props.state.stopHostedRuntime"
        >
          {{ translate('connection.stopHosted') }}
        </button>
      </div>
      <p v-if="props.state.connectDisabledReason.value && props.state.runtimeStatus.value.phase !== 'running'" class="helper-text">
        {{ props.state.connectDisabledReason.value }}
      </p>
      <p v-if="props.state.runtimeError.value" class="error-banner">{{ props.state.runtimeError.value }}</p>
    </article>
  </section>
</template>
