<script setup lang="ts">
import { translate } from '@/lib/i18n'

interface SidebarSection {
  key: string
  label: string
}

const props = defineProps<{
  sections: readonly SidebarSection[]
  activeSection: string
  collapsed: boolean
  pinnedCollapsed: boolean
}>()

const emit = defineEmits<{
  selectSection: [sectionKey: string]
  toggleCollapse: []
  hoverChange: [hovering: boolean]
}>()

function sidebarToggleLabel() {
  return translate(props.pinnedCollapsed ? 'nav.expandSidebar' : 'nav.collapseSidebar')
}
</script>

<template>
  <aside
    class="sidebar surface-card"
    :data-collapsed="String(props.collapsed)"
    :data-hover-expanded="String(props.pinnedCollapsed && !props.collapsed)"
    @mouseenter="emit('hoverChange', true)"
    @mouseleave="emit('hoverChange', false)"
  >
    <div class="brand-panel brand-panel--minimal">
      <button
        class="sidebar-toggle"
        type="button"
        :aria-label="sidebarToggleLabel()"
        :title="sidebarToggleLabel()"
        @click="emit('toggleCollapse')"
      >
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path d="M5 5H9V19H5V5Z" />
          <path
            v-if="props.collapsed"
            d="M12 8L16 12L12 16"
            fill="none"
            stroke="currentColor"
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="1.8"
          />
          <path
            v-else
            d="M16 8L12 12L16 16"
            fill="none"
            stroke="currentColor"
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="1.8"
          />
        </svg>
      </button>

      <div class="brand-lockup">
        <span class="brand-glyph" aria-hidden="true">
          <svg viewBox="0 0 24 24">
            <path d="M6 5H10V19H6V5Z" />
            <path d="M11 7H16C18.2091 7 20 8.79086 20 11V13C20 16.866 16.866 20 13 20H11V16H13C14.6569 16 16 14.6569 16 13V11C16 9.89543 15.1046 9 14 9H11V7Z" />
          </svg>
        </span>
        <h1 v-if="!props.collapsed">BIZCLAW</h1>
      </div>
    </div>

    <nav class="sidebar-nav" :aria-label="translate('workspace.eyebrow')">
      <button
        v-for="item in props.sections"
        :key="item.key"
        class="nav-button"
        :data-active="props.activeSection === item.key"
        :aria-current="props.activeSection === item.key ? 'page' : undefined"
        :aria-label="item.label"
        :title="item.label"
        @click="emit('selectSection', item.key)"
      >
        <span class="nav-button-icon" aria-hidden="true">
          <svg v-if="item.key === 'overview'" viewBox="0 0 24 24">
            <path d="M5 5H11V11H5V5Z" />
            <path d="M13 5H19V11H13V5Z" />
            <path d="M5 13H11V19H5V13Z" />
            <path d="M13 13H19V19H13V13Z" />
          </svg>
          <svg v-else-if="item.key === 'agent'" viewBox="0 0 24 24">
            <path d="M12 12.5A3.5 3.5 0 1 0 12 5.5A3.5 3.5 0 0 0 12 12.5Z" fill="none" stroke="currentColor" stroke-width="1.8" />
            <path d="M6.5 18C7.4 15.7 9.45 14.2 12 14.2C14.55 14.2 16.6 15.7 17.5 18" fill="none" stroke="currentColor" stroke-linecap="round" stroke-width="1.8" />
            <path d="M18.5 8.5H21.5" fill="none" stroke="currentColor" stroke-linecap="round" stroke-width="1.8" />
            <path d="M20 7V10" fill="none" stroke="currentColor" stroke-linecap="round" stroke-width="1.8" />
          </svg>
          <svg v-else-if="item.key === 'install'" viewBox="0 0 24 24">
            <path d="M12 4V14" fill="none" stroke="currentColor" stroke-linecap="round" stroke-width="1.8" />
            <path d="M8.5 10.5L12 14L15.5 10.5" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" />
            <path d="M6 18H18" fill="none" stroke="currentColor" stroke-linecap="round" stroke-width="1.8" />
          </svg>
          <svg v-else-if="item.key === 'connection'" viewBox="0 0 24 24">
            <path d="M8 9C8 6.79086 9.79086 5 12 5C14.2091 5 16 6.79086 16 9V10.5H17C17.5523 10.5 18 10.9477 18 11.5V18C18 18.5523 17.5523 19 17 19H7C6.44772 19 6 18.5523 6 18V11.5C6 10.9477 6.44772 10.5 7 10.5H8V9Z" fill="none" stroke="currentColor" stroke-width="1.8" />
            <path d="M12 13V16" fill="none" stroke="currentColor" stroke-linecap="round" stroke-width="1.8" />
          </svg>
          <svg v-else-if="item.key === 'runtime'" viewBox="0 0 24 24">
            <path d="M7 6H17C18.1046 6 19 6.89543 19 8V16C19 17.1046 18.1046 18 17 18H7C5.89543 18 5 17.1046 5 16V8C5 6.89543 5.89543 6 7 6Z" fill="none" stroke="currentColor" stroke-width="1.8" />
            <path d="M9 10L11.5 12L9 14" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" />
            <path d="M13.5 14H15.5" fill="none" stroke="currentColor" stroke-linecap="round" stroke-width="1.8" />
          </svg>
          <svg v-else-if="item.key === 'skill'" viewBox="0 0 24 24">
            <path d="M7 7H13L17 11V17H7V7Z" fill="none" stroke="currentColor" stroke-linejoin="round" stroke-width="1.8" />
            <path d="M13 7V11H17" fill="none" stroke="currentColor" stroke-linejoin="round" stroke-width="1.8" />
            <path d="M10 14H14" fill="none" stroke="currentColor" stroke-linecap="round" stroke-width="1.8" />
          </svg>
          <svg v-else-if="item.key === 'chat'" viewBox="0 0 24 24">
            <path d="M7 7.5H17C18.1046 7.5 19 8.39543 19 9.5V14.5C19 15.6046 18.1046 16.5 17 16.5H11L7.5 19V16.5H7C5.89543 16.5 5 15.6046 5 14.5V9.5C5 8.39543 5.89543 7.5 7 7.5Z" fill="none" stroke="currentColor" stroke-linejoin="round" stroke-width="1.8" />
            <path d="M8.8 11H15.2" fill="none" stroke="currentColor" stroke-linecap="round" stroke-width="1.8" />
            <path d="M8.8 13.5H12.8" fill="none" stroke="currentColor" stroke-linecap="round" stroke-width="1.8" />
          </svg>
          <svg v-else viewBox="0 0 24 24">
            <path d="M12 4.5L13.5645 6.11498L15.8076 5.6527L16.4084 7.86553L18.5862 8.56283L17.889 10.7406L19.5 12L17.889 13.2594L18.5862 15.4372L16.4084 16.1345L15.8076 18.3473L13.5645 17.885L12 19.5L10.4355 17.885L8.19239 18.3473L7.59161 16.1345L5.41378 15.4372L6.11098 13.2594L4.5 12L6.11098 10.7406L5.41378 8.56283L7.59161 7.86553L8.19239 5.6527L10.4355 6.11498L12 4.5Z" fill="none" stroke="currentColor" stroke-width="1.6" />
            <path d="M12 9.2A2.8 2.8 0 1 1 12 14.8A2.8 2.8 0 0 1 12 9.2Z" fill="none" stroke="currentColor" stroke-width="1.6" />
          </svg>
        </span>
        <span v-if="!props.collapsed" class="nav-button-label">{{ item.label }}</span>
      </button>
    </nav>
  </aside>
</template>
