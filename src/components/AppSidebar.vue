<script setup lang="ts">
import { translate } from '@/lib/i18n'

interface SidebarSection {
  key: string
  label: string
}

const props = defineProps<{
  sections: readonly SidebarSection[]
  activeSection: string
}>()

const emit = defineEmits<{
  selectSection: [sectionKey: string]
}>()
</script>

<template>
  <aside class="sidebar surface-card">
    <div class="brand-panel brand-panel--minimal">
      <h1>BIZCLAW</h1>
    </div>

    <nav class="sidebar-nav" :aria-label="translate('workspace.eyebrow')">
      <button
        v-for="item in props.sections"
        :key="item.key"
        class="nav-button"
        :data-active="props.activeSection === item.key"
        @click="emit('selectSection', item.key)"
      >
        <span>{{ item.label }}</span>
      </button>
    </nav>
  </aside>
</template>
