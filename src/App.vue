<script setup lang="ts">
import { computed } from 'vue'

import { operationStepLabel, phaseLabel } from '@/lib/runtime-view'
import { useAppModel } from '@/lib/use-app-model'
import type { ConnectionTestModalStep } from '@/types'

const {
  activeSection,
  advancedOpen,
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
  environment,
  installBusyAction,
  installCli,
  launchManualInstall,
  logs,
  manualInstallBusy,
  operationError,
  operationEvents,
  operationHeadline,
  operationsSummary,
  operationTask,
  operationTaskPhaseLabel,
  openclawStateLabel,
  openclawStateTone,
  overviewCards,
  platformLabel,
  profileError,
  refreshEnvironment,
  runtimeError,
  runtimeStartBusy,
  runtimeStatus,
  runtimeStopBusy,
  saveAndTest,
  saveBusy,
  saveOnly,
  sshPasswordInput,
  sshStateLabel,
  startHostedRuntime,
  statusItems,
  stopOperation,
  stopHostedRuntime,
  targetProfile,
  tokenInput,
  gatewayStateLabel,
  gatewayStateTone,
  tokenStateLabel,
  tokenStateToneValue,
  updateCli,
  userProfile,
} = useAppModel()

const sections = [
  { key: 'overview', label: '概览' },
  { key: 'install', label: '安装与更新' },
  { key: 'connection', label: '连接与配置' },
  { key: 'runtime', label: '运行日志' },
] as const

const sectionTitle = computed(() => sections.find((item) => item.key === activeSection.value)?.label ?? '概览')
const logPreview = computed(() => [...logs.value].reverse().slice(0, 16))
const gatewayConnected = computed(() => (
  runtimeStatus.value.phase === 'running' && runtimeStatus.value.gatewayConnected
))

const busyLabel = computed(() => {
  if (operationTask.value.phase === 'running' && operationTask.value.kind === 'install') {
    return '安装中'
  }
  if (operationTask.value.phase === 'running' && operationTask.value.kind === 'update') {
    return '更新中'
  }
  if (operationTask.value.phase === 'cancelling') {
    return '停止中'
  }
  if (installBusyAction.value === 'check-update') {
    return '检查更新中'
  }
  if (manualInstallBusy.value) {
    return '打开文档中'
  }
  if (saveBusy.value) {
    return '保存中'
  }
  if (connectionTestBusy.value) {
    return '测试中'
  }
  if (runtimeStartBusy.value) {
    return '启动中'
  }
  if (runtimeStopBusy.value) {
    return '停止中'
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
      return '完成'
    case 'error':
      return '失败'
    case 'running':
      return '进行中'
    default:
      return '待执行'
  }
}
</script>

<template>
  <main class="ops-shell">
    <aside class="sidebar surface-card">
      <div class="brand-panel brand-panel--minimal">
        <h1>BIZCLAW</h1>
      </div>

      <nav class="sidebar-nav" aria-label="主导航">
        <button
          v-for="item in sections"
          :key="item.key"
          class="nav-button"
          :data-active="activeSection === item.key"
          @click="activeSection = item.key"
        >
          <span>{{ item.label }}</span>
        </button>
      </nav>
    </aside>

    <section class="workspace">
      <header class="workspace-header surface-card">
        <div>
          <p class="eyebrow">Workspace</p>
          <h2>{{ sectionTitle }}</h2>
        </div>
        <div class="workspace-actions">
          <span
            class="status-chip"
            :data-tone="runtimeStatus.phase === 'running' ? 'success' : runtimeStatus.phase === 'error' ? 'error' : 'neutral'"
          >
            {{ phaseLabel(runtimeStatus.phase) }}
          </span>
          <span v-if="busyLabel" class="status-chip" data-tone="active">{{ busyLabel }}</span>
          <button class="secondary-button" @click="refreshEnvironment">重新检测</button>
        </div>
      </header>

      <section v-if="activeSection === 'overview'" class="page-grid">
        <article class="hero-card surface-card">
          <p class="eyebrow">Snapshot</p>
          <h3>{{ operationsSummary.title }}</h3>
          <p class="supporting-text">{{ operationsSummary.detail }}</p>
          <div class="hero-actions">
            <button class="primary-button" @click="activeSection = 'install'">前往安装与更新</button>
            <button class="secondary-button" @click="activeSection = 'connection'">前往连接与配置</button>
          </div>
        </article>

        <article
          v-for="card in overviewCards"
          :key="card.label"
          class="overview-card surface-card"
          :data-tone="card.tone"
        >
          <span class="card-label">{{ card.label }}</span>
          <strong>{{ card.value }}</strong>
          <p class="supporting-text">{{ card.detail }}</p>
        </article>
      </section>

      <section v-else-if="activeSection === 'install'" class="page-stack">
        <article class="surface-card section-card">
          <div class="section-header">
            <div>
              <p class="eyebrow">Install & Update</p>
              <h3>目标环境准备</h3>
            </div>
            <span class="status-chip" :data-tone="operationsSummary.tone">{{ operationsSummary.title }}</span>
          </div>
          <p class="supporting-text">{{ operationsSummary.detail }}</p>
          <div class="support-grid">
            <div class="support-tile">
              <span>目标运行时</span>
              <strong>{{ platformLabel }}</strong>
            </div>
            <div class="support-tile">
              <span>当前版本</span>
              <strong>{{ environment?.openclawVersion ?? '未安装' }}</strong>
            </div>
            <div class="support-tile">
              <span>最新版本</span>
              <strong>{{ environment?.latestOpenclawVersion ?? '未检测' }}</strong>
            </div>
            <div class="support-tile">
              <span>OpenSSH</span>
              <strong>{{ sshStateLabel }}</strong>
            </div>
          </div>
          <div class="button-row">
            <button
              class="primary-button"
              :disabled="operationTask.phase === 'running' || operationTask.phase === 'cancelling'"
              @click="installCli"
            >
              安装 OpenClaw
            </button>
            <button class="secondary-button" :disabled="installBusyAction === 'check-update'" @click="checkForUpdates">
              检查更新
            </button>
            <button
              class="secondary-button"
              :disabled="!environment?.updateAvailable || operationTask.phase === 'running' || operationTask.phase === 'cancelling'"
              @click="updateCli"
            >
              更新 OpenClaw
            </button>
            <button
              v-if="canStopOperation"
              class="secondary-button"
              @click="stopOperation"
            >
              停止
            </button>
            <button class="ghost-button" @click="launchManualInstall">官方文档</button>
          </div>
          <p v-if="operationTask.step" class="helper-text">
            当前步骤：{{ operationStepLabel(operationTask.step) }} · {{ operationTaskPhaseLabel(operationTask.phase) }}
          </p>
          <p v-if="environment?.wslStatus?.message" class="helper-text">
            {{ environment.wslStatus.message }}
          </p>
          <p class="helper-text">{{ operationHeadline }}</p>
          <p v-if="operationError" class="error-banner">{{ operationError }}</p>
        </article>

        <article class="surface-card section-card">
          <div class="section-header">
            <div>
              <p class="eyebrow">Live Console</p>
              <h3>安装 / 更新输出</h3>
            </div>
            <span
              class="status-chip"
              :data-tone="operationTask.phase === 'error' ? 'error' : operationTask.phase === 'success' ? 'success' : operationTask.phase === 'cancelled' ? 'neutral' : 'active'"
            >
              {{ operationTaskPhaseLabel(operationTask.phase) }}
            </span>
          </div>
          <ol class="operation-list">
            <li v-for="entry in operationEvents" :key="`${entry.timestampMs}-${entry.message}`">
              <time>{{ new Date(entry.timestampMs).toLocaleTimeString() }}</time>
              <strong>{{ operationStepLabel(entry.step) }}</strong>
              <span>{{ entry.message }}</span>
            </li>
          </ol>
        </article>
      </section>

      <section v-else-if="activeSection === 'connection'" class="page-stack">
        <article class="surface-card section-card">
          <div class="section-header">
            <div>
              <p class="eyebrow">Connection</p>
              <h3>保存连接配置</h3>
            </div>
            <span class="status-chip" :data-tone="tokenStateToneValue">{{ tokenStateLabel }}</span>
          </div>
          <div class="form-grid form-grid--compact">
            <label class="field">
              <span>显示名称</span>
              <input v-model="userProfile.displayName" placeholder="名称" />
            </label>
            <label class="field">
              <span>Gateway Token</span>
              <input
                v-model="tokenInput"
                type="password"
                placeholder="如已保存，可留空保持原值"
              />
            </label>
            <label class="field">
              <span>SSH Host</span>
              <input v-model="companyProfile.sshHost" placeholder="gateway.example.com" />
            </label>
            <label class="field">
              <span>SSH User</span>
              <input v-model="companyProfile.sshUser" placeholder="bizclaw" />
            </label>
          </div>

          <div class="toggle-row">
            <label class="toggle-pill">
              <input v-model="userProfile.runInBackground" type="checkbox" />
              <span>关闭窗口后继续后台运行</span>
            </label>
          </div>

          <button class="inline-button" @click="advancedOpen = !advancedOpen">
            {{ advancedOpen ? '收起高级参数' : '展开高级参数' }}
          </button>

          <div v-if="advancedOpen" class="advanced-grid">
            <label class="field">
              <span>WSL Distro</span>
              <input v-model="targetProfile.wslDistro" placeholder="Ubuntu" />
            </label>
            <label class="field">
              <span>Local Port</span>
              <input v-model="companyProfile.localPort" inputmode="numeric" />
            </label>
            <label class="field">
              <span>Remote Bind Host</span>
              <input v-model="companyProfile.remoteBindHost" />
            </label>
            <label class="field">
              <span>Remote Bind Port</span>
              <input v-model="companyProfile.remoteBindPort" inputmode="numeric" />
            </label>
            <label class="field field--span">
              <span>SSH Password</span>
              <input
                v-model="sshPasswordInput"
                type="password"
                placeholder="如已保存，可留空保持原值"
              />
            </label>
          </div>

          <p class="helper-text">
            默认使用 `127.0.0.1` loopback 转发；只有在网关部署要求不同的时候才需要改高级参数。
          </p>
          <p v-if="profileError" class="error-banner">{{ profileError }}</p>
          <p v-if="connectionTestDisabledReason" class="helper-text">{{ connectionTestDisabledReason }}</p>
          <div class="button-row button-row--end">
            <button class="secondary-button" :disabled="!canSaveProfile" @click="saveOnly">
              仅保存
            </button>
            <button class="primary-button" :disabled="!canTestConnection" @click="saveAndTest">
              保存并测试
            </button>
          </div>
        </article>

        <article class="surface-card section-card">
          <div class="section-header">
            <div>
              <p class="eyebrow">Runtime</p>
              <h3>托管运行时</h3>
            </div>
            <span class="status-chip" :data-tone="runtimeStatus.phase === 'running' ? 'success' : 'neutral'">
              {{ phaseLabel(runtimeStatus.phase) }}
            </span>
          </div>
          <div class="support-grid support-grid--metrics">
            <div class="support-tile">
              <span>Gateway</span>
              <strong>{{ gatewayConnected ? '已连接' : '未连接' }}</strong>
            </div>
            <div class="support-tile">
              <span>SSH</span>
              <strong>{{ runtimeStatus.sshConnected ? '已连接' : '未连接' }}</strong>
            </div>
            <div class="support-tile">
              <span>Node</span>
              <strong>{{ runtimeStatus.nodeConnected ? '已连接' : '未连接' }}</strong>
            </div>
            <div class="support-tile">
              <span>目标环境</span>
              <strong>{{ platformLabel }}</strong>
            </div>
          </div>
          <div class="button-row">
            <button
              class="primary-button"
              :disabled="!canStartHostedRuntime"
              @click="startHostedRuntime"
            >
              启动托管
            </button>
            <button
              class="secondary-button"
              :disabled="runtimeStopBusy || runtimeStatus.phase !== 'running'"
              @click="stopHostedRuntime"
            >
              停止托管
            </button>
          </div>
          <p v-if="connectDisabledReason && runtimeStatus.phase !== 'running'" class="helper-text">
            {{ connectDisabledReason }}
          </p>
          <p v-if="runtimeError" class="error-banner">{{ runtimeError }}</p>
        </article>
      </section>

      <section v-else class="page-stack">
        <article class="surface-card section-card">
          <div class="section-header">
            <div>
              <p class="eyebrow">Runtime Log</p>
              <h3>最近日志</h3>
            </div>
            <span class="status-chip" data-tone="neutral">{{ logs.length }} 条</span>
          </div>
          <ol class="log-list">
            <li v-for="entry in logPreview" :key="`${entry.timestampMs}-${entry.message}`">
              <time>{{ new Date(entry.timestampMs).toLocaleTimeString() }}</time>
              <strong>[{{ entry.source }}]</strong>
              <span>{{ entry.message }}</span>
            </li>
          </ol>
        </article>
      </section>
    </section>

    <footer class="status-bar surface-card">
      <div v-for="item in statusItems" :key="item.label" class="status-bar-item">
        <span>{{ item.label }}</span>
        <strong :data-tone="item.tone">{{ item.value }}</strong>
      </div>
    </footer>
  </main>

  <Teleport to="body">
    <div v-if="connectionTestModal.open" class="modal-backdrop">
      <section class="modal-card surface-card" role="dialog" aria-modal="true" aria-labelledby="connection-test-title">
        <div class="section-header">
          <div>
            <p class="eyebrow">Connection Test</p>
            <h3 id="connection-test-title">连接测试结果</h3>
          </div>
          <span class="status-chip" :data-tone="connectionTestModal.phase === 'success' ? 'success' : connectionTestModal.phase === 'error' ? 'error' : 'active'">
            {{
              connectionTestModal.phase === 'success'
                ? '测试通过'
                : connectionTestModal.phase === 'error'
                  ? '测试失败'
                  : '测试中'
            }}
          </span>
        </div>

        <p class="supporting-text">{{ connectionTestModal.summary }}</p>

        <ol class="connection-test-list">
          <li v-for="step in connectionTestModal.steps" :key="step.step" class="connection-test-item">
            <div>
              <strong>{{ step.label }}</strong>
              <p class="supporting-text">{{ step.message || '等待执行。' }}</p>
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
            <span class="card-label">stdout</span>
            <pre>{{ connectionTestModal.result.stdout }}</pre>
          </div>
          <div v-if="connectionTestModal.result.stderr">
            <span class="card-label">stderr</span>
            <pre>{{ connectionTestModal.result.stderr }}</pre>
          </div>
        </div>

        <div class="button-row button-row--end">
          <button class="primary-button" :disabled="connectionTestCloseDisabled" @click="closeConnectionTestModal">
            关闭
          </button>
        </div>
      </section>
    </div>
  </Teleport>
</template>
