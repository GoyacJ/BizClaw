<script setup lang="ts">
import { translate } from '@/lib/i18n'
import type { OperationsSummary } from '@/lib/runtime-view'

interface OverviewCard {
  label: string
  value: string
  detail: string
  tone: string
}

const props = defineProps<{
  operationsSummary: OperationsSummary
  overviewCards: OverviewCard[]
  goInstall: () => void
  goConnection: () => void
}>()
</script>

<template>
  <section class="page-grid">
    <article class="hero-card surface-card">
      <p class="eyebrow">{{ translate('overview.eyebrow') }}</p>
      <h3>{{ props.operationsSummary.title }}</h3>
      <p class="supporting-text">{{ props.operationsSummary.detail }}</p>
      <div class="hero-actions">
        <button class="primary-button" @click="props.goInstall">{{ translate('overview.goInstall') }}</button>
        <button class="secondary-button" @click="props.goConnection">{{ translate('overview.goConnection') }}</button>
      </div>
    </article>

    <article
      v-for="card in props.overviewCards"
      :key="card.label"
      class="overview-card surface-card"
      :data-tone="card.tone"
    >
      <span class="card-label">{{ card.label }}</span>
      <strong>{{ card.value }}</strong>
      <p class="supporting-text">{{ card.detail }}</p>
    </article>
  </section>
</template>
