<script setup lang="ts">
import { computed } from 'vue'
import type { Ref } from 'vue'

import { formatAppDate, translate } from '@/lib/i18n'
import type { BizClawUpdateState, ThemePreference, UiPreferences } from '@/types'

interface SettingsSectionState {
  uiPreferences: Ref<UiPreferences>
  setTheme: (theme: ThemePreference) => Promise<void> | void
  setLocale: (locale: 'zh-CN' | 'en-US') => Promise<void> | void
  bizclawUpdate: Ref<BizClawUpdateState>
  bizclawUpdateActionLabel: Ref<string>
  bizclawUpdateTone: Ref<string>
  bizclawUpdateDetail: Ref<string>
  bizclawUpdateBlockedReason: Ref<string | null>
  bizclawUpdatePrimaryAction: Ref<string>
  checkBizClawUpdates: () => Promise<void> | void
  installBizClawUpdate: () => Promise<void> | void
  restartBizClaw: () => Promise<void> | void
  deferBizClawRestart: () => void
}

const props = defineProps<{
  state: SettingsSectionState
}>()

const bizclawPublishedLabel = computed(() => (
  props.state.bizclawUpdate.value.publishedAt
    ? formatAppDate(props.state.bizclawUpdate.value.publishedAt)
    : translate('common.notDetected')
))

const bizclawProgressLabel = computed(() => {
  if (!props.state.bizclawUpdate.value.totalBytes && props.state.bizclawUpdate.value.downloadedBytes === 0) {
    return ''
  }

  if (props.state.bizclawUpdate.value.totalBytes) {
    return translate('install.progressKnown', {
      downloaded: props.state.bizclawUpdate.value.downloadedBytes,
      total: props.state.bizclawUpdate.value.totalBytes,
    })
  }

  return translate('install.progressUnknown', {
    downloaded: props.state.bizclawUpdate.value.downloadedBytes,
  })
})
</script>

<template>
  <section class="page-stack settings-stack">
    <article class="surface-card section-card settings-card">
      <div class="section-header settings-card-header">
        <div class="settings-title-block">
          <p class="eyebrow">{{ translate('settings.appearanceEyebrow') }}</p>
          <h3>{{ translate('settings.appearanceTitle') }}</h3>
        </div>
      </div>
      <p class="supporting-text">{{ translate('settings.appearanceDetail') }}</p>
      <div class="settings-option-row">
        <button
          class="secondary-button"
          :data-active="props.state.uiPreferences.value.theme === 'system'"
          @click="props.state.setTheme('system')"
        >
          {{ translate('settings.system') }}
        </button>
        <button
          class="secondary-button"
          :data-active="props.state.uiPreferences.value.theme === 'light'"
          @click="props.state.setTheme('light')"
        >
          {{ translate('settings.light') }}
        </button>
        <button
          class="secondary-button"
          :data-active="props.state.uiPreferences.value.theme === 'dark'"
          @click="props.state.setTheme('dark')"
        >
          {{ translate('settings.dark') }}
        </button>
      </div>
    </article>

    <article class="surface-card section-card settings-card">
      <div class="section-header settings-card-header">
        <div class="settings-title-block">
          <p class="eyebrow">{{ translate('settings.languageEyebrow') }}</p>
          <h3>{{ translate('settings.languageTitle') }}</h3>
        </div>
      </div>
      <p class="supporting-text">{{ translate('settings.languageDetail') }}</p>
      <div class="settings-option-row">
        <button
          class="secondary-button"
          :data-active="props.state.uiPreferences.value.locale === 'zh-CN'"
          @click="props.state.setLocale('zh-CN')"
        >
          {{ translate('settings.chinese') }}
        </button>
        <button
          class="secondary-button"
          :data-active="props.state.uiPreferences.value.locale === 'en-US'"
          @click="props.state.setLocale('en-US')"
        >
          {{ translate('settings.english') }}
        </button>
      </div>
    </article>

    <article class="surface-card section-card settings-card">
      <div class="section-header settings-card-header">
        <div class="settings-title-block">
          <p class="eyebrow">{{ translate('settings.versionEyebrow') }}</p>
          <h3>{{ translate('settings.versionTitle') }}</h3>
        </div>
        <span class="status-chip settings-status-chip" :data-tone="props.state.bizclawUpdateTone.value">{{ props.state.bizclawUpdateActionLabel.value }}</span>
      </div>
      <p class="supporting-text">{{ translate('settings.versionDetail') }}</p>
      <div class="support-grid">
        <div class="support-tile">
          <span>{{ translate('install.currentBizClawVersion') }}</span>
          <strong>{{ props.state.bizclawUpdate.value.currentVersion ?? translate('common.checkPending') }}</strong>
        </div>
        <div class="support-tile">
          <span>{{ translate('install.latestBizClawVersion') }}</span>
          <strong>{{ props.state.bizclawUpdate.value.latestVersion ?? translate('common.notDetected') }}</strong>
        </div>
        <div class="support-tile">
          <span>{{ translate('install.publishedAt') }}</span>
          <strong>{{ bizclawPublishedLabel }}</strong>
        </div>
        <div class="support-tile">
          <span>{{ translate('install.restartStatus') }}</span>
          <strong>{{ props.state.bizclawUpdate.value.phase === 'readyToRestart' ? translate('install.restartPending') : translate('install.restartIdle') }}</strong>
        </div>
      </div>
      <p class="helper-text">{{ props.state.bizclawUpdateDetail.value }}</p>
      <div class="button-row version-actions">
        <button
          class="secondary-button"
          :disabled="props.state.bizclawUpdate.value.phase === 'checking' || props.state.bizclawUpdate.value.phase === 'downloading' || props.state.bizclawUpdate.value.phase === 'installing'"
          @click="props.state.checkBizClawUpdates"
        >
          {{ translate('install.checkUpdate') }}
        </button>
        <button
          v-if="props.state.bizclawUpdate.value.phase !== 'readyToRestart'"
          class="primary-button"
          :disabled="props.state.bizclawUpdate.value.phase !== 'available' || !!props.state.bizclawUpdateBlockedReason.value"
          @click="props.state.installBizClawUpdate"
        >
          {{ props.state.bizclawUpdatePrimaryAction.value }}
        </button>
        <template v-else>
          <button class="primary-button" @click="props.state.restartBizClaw">
            {{ props.state.bizclawUpdatePrimaryAction.value }}
          </button>
          <button class="secondary-button" @click="props.state.deferBizClawRestart">
            {{ translate('common.later') }}
          </button>
        </template>
      </div>
      <div
        v-if="props.state.bizclawUpdate.value.releaseNotes || bizclawProgressLabel || (props.state.bizclawUpdateBlockedReason.value && props.state.bizclawUpdate.value.phase === 'available') || props.state.bizclawUpdate.value.errorMessage"
        class="version-notes"
      >
        <p v-if="props.state.bizclawUpdate.value.releaseNotes" class="helper-text version-note version-note--release">
          {{ translate('install.releaseNotes', { notes: props.state.bizclawUpdate.value.releaseNotes }) }}
        </p>
        <p v-if="bizclawProgressLabel" class="helper-text version-note">
          {{ bizclawProgressLabel }}
        </p>
        <p v-if="props.state.bizclawUpdateBlockedReason.value && props.state.bizclawUpdate.value.phase === 'available'" class="helper-text version-note">
          {{ props.state.bizclawUpdateBlockedReason.value }}
        </p>
        <p v-if="props.state.bizclawUpdate.value.errorMessage" class="error-banner">
          {{ props.state.bizclawUpdate.value.errorMessage }}
        </p>
      </div>
    </article>
  </section>
</template>
