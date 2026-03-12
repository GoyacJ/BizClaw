<script setup lang="ts">
import { computed } from 'vue'

import { phaseLabel } from '@/lib/runtime-view'
import { useAppModel } from '@/lib/use-app-model'

const {
  advancedOpen,
  busyAction,
  canSaveAndConnect,
  canSaveProfile,
  canStartHostedRuntime,
  companyProfile,
  connectDisabledReason,
  environment,
  installCli,
  installResult,
  installSummary,
  launchManualInstall,
  logs,
  profileError,
  refreshEnvironment,
  runtimeError,
  runtimeStatus,
  saveAndConnect,
  saveOnly,
  showInstallConsole,
  sshPasswordInput,
  startHostedRuntime,
  stopHostedRuntime,
  tokenInput,
  tokenStateLabel,
  tokenStateToneValue,
  userProfile,
  workflow,
} = useAppModel()

const busyLabel = computed(() => {
  switch (busyAction.value) {
    case 'install':
      return '正在执行安装'
    case 'manual-install':
      return '正在打开文档'
    case 'save':
      return '正在保存配置'
    case 'connect':
      return '正在保存并连接'
    case 'start':
      return '正在启动托管'
    case 'stop':
      return '正在停止托管'
    default:
      return ''
  }
})

const logPreview = computed(() => [...logs.value].reverse().slice(0, 12))

const installConsoleOutput = computed(() => {
  const result = installResult.value
  if (!result) {
    return ''
  }

  return result.stdout.trim() || result.stderr.trim() || '安装命令已执行，等待更多输出。'
})

function hostedRuntimeTone() {
  const status = runtimeStatus.value

  switch (status.phase) {
    case 'running':
      return 'success'
    case 'connecting':
      return 'active'
    case 'error':
      return 'error'
    default:
      return 'neutral'
  }
}

function hostedRuntimeDetail() {
  const status = runtimeStatus.value

  if (status.lastError) {
    return status.lastError
  }

  if (status.phase === 'running') {
    return `${status.sshConnected ? 'SSH 已连接' : 'SSH 未连接'} · ${status.nodeConnected ? 'Gateway 已连接' : 'Gateway 未连接'}`
  }

  if (status.phase === 'connecting') {
    return '正在建立 SSH 与 Gateway 连接'
  }

  return '可使用已保存配置启动或停止托管'
}

const environmentCards = computed(() => {
  const snapshot = environment.value
  if (!snapshot) {
    return []
  }

  return [
    {
      label: 'SSH',
      status: snapshot.sshInstalled ? '可用' : '缺失',
      detail: snapshot.sshInstalled ? '已检测到系统命令' : '需要系统 SSH',
      tone: snapshot.sshInstalled ? 'success' : 'warning',
    },
    {
      label: 'OpenClaw CLI',
      status: snapshot.openclawInstalled ? '已安装' : '未安装',
      detail: snapshot.openclawInstalled
        ? (snapshot.openclawVersion ?? '已检测到 CLI')
        : '安装后可启动托管',
      tone: snapshot.openclawInstalled ? 'success' : 'warning',
    },
    {
      label: '托管',
      status: phaseLabel(runtimeStatus.value.phase),
      detail: hostedRuntimeDetail(),
      tone: hostedRuntimeTone(),
    },
    {
      label: 'npm',
      status: snapshot.npmInstalled ? '可用' : '缺失',
      detail: snapshot.npmInstalled ? '可作为回退安装源' : '缺少 npm',
      tone: snapshot.npmInstalled ? 'success' : 'neutral',
    },
    {
      label: 'pnpm',
      status: snapshot.pnpmInstalled ? '可用' : '缺失',
      detail: snapshot.pnpmInstalled ? '可作为回退安装源' : '缺少 pnpm',
      tone: snapshot.pnpmInstalled ? 'success' : 'neutral',
    },
  ]
})

const saveHint = computed(() => {
  if (!environment.value?.openclawInstalled) {
    return canSaveProfile.value
      ? '可以先保存配置，安装 OpenClaw 后再立即连接。'
      : '安装 OpenClaw 前也可以先填写并保存配置。'
  }

  if (!userProfile.displayName.trim()) {
    return '请填写显示名称。'
  }

  const missingNetworkField = [
    companyProfile.sshHost,
    companyProfile.sshUser,
    companyProfile.localPort,
    companyProfile.remoteBindHost,
    companyProfile.remoteBindPort,
  ].some((value) => value.trim().length === 0)

  if (missingNetworkField) {
    return '请补全 SSH 与端口参数。'
  }

  if (!tokenInput.value.trim() && environment.value?.tokenStatus !== 'saved') {
    return '请填写 Gateway Token。'
  }

  return '配置已完整，可以保存到本机并启动托管。'
})
</script>

<template>
  <main class="dashboard-shell">
    <header class="topbar surface-card">
      <div class="brand-lockup">
        <div class="brand-mark">B</div>
        <div>
          <p class="eyebrow">BizClaw</p>
          <h1>接入工作台</h1>
        </div>
      </div>

      <div class="topbar-actions">
        <span class="status-chip" data-tone="success">{{ phaseLabel(runtimeStatus.phase) }}</span>
        <span v-if="busyLabel" class="status-chip" data-tone="accent">{{ busyLabel }}</span>
        <button class="secondary-button" @click="refreshEnvironment">
          重新检测
        </button>
      </div>
    </header>

    <section class="workflow-strip surface-card">
      <ol class="workflow-strip__list">
        <li
          v-for="(step, index) in workflow"
          :key="step.key"
          class="workflow-step"
          :data-state="step.state"
        >
          <span class="workflow-step__index">{{ `0${index + 1}` }}</span>
          <div class="workflow-step__body">
            <strong>{{ step.title }}</strong>
            <span>{{ step.caption }}</span>
          </div>
        </li>
      </ol>
    </section>

    <section class="workspace-grid">
      <article class="workspace-card workspace-card--wide surface-card">
        <div class="card-header">
          <div>
            <p class="eyebrow">Step 1</p>
            <h2>安装 OpenClaw</h2>
          </div>
          <span class="status-chip" :data-tone="installSummary.tone">{{ installSummary.title }}</span>
        </div>

        <p class="supporting-text">{{ installSummary.detail }}</p>

        <div v-if="environment?.openclawInstalled" class="support-grid">
          <div class="support-tile">
            <span>当前版本</span>
            <strong>{{ environment.openclawVersion ?? '已检测到 OpenClaw CLI' }}</strong>
          </div>
          <div class="support-tile">
            <span>安装建议</span>
            <strong>{{ environment.installRecommendation }}</strong>
          </div>
        </div>

        <div class="button-row">
          <button
            class="primary-button"
            :disabled="Boolean(environment?.openclawInstalled) || busyAction === 'install'"
            @click="installCli"
          >
            自动安装
          </button>
          <button class="secondary-button" @click="launchManualInstall">
            查看安装文档
          </button>
        </div>

        <div v-if="showInstallConsole" class="console-card">
          <div class="console-card__header">
            <span>安装输出</span>
            <span class="meta-text">{{ installResult?.followUp }}</span>
          </div>
          <pre>{{ installConsoleOutput }}</pre>
        </div>
      </article>

      <article class="workspace-card surface-card">
        <div class="card-header">
          <div>
            <p class="eyebrow">Step 2</p>
            <h2>保存连接配置</h2>
          </div>
          <span class="status-chip" :data-tone="tokenStateToneValue">{{ tokenStateLabel }}</span>
        </div>

        <p class="supporting-text">保存显示名称、SSH 参数和 Gateway Token，供托管运行时直接复用。</p>

        <div class="form-grid">
          <label class="field">
            <span>显示名称</span>
            <input v-model="userProfile.displayName" placeholder="例如 Goya Mac" />
          </label>

          <label class="field">
            <span>Gateway Token</span>
            <input
              v-model="tokenInput"
              type="password"
              placeholder="如已保存，可留空保持原值"
            />
          </label>
        </div>

        <div class="toggle-row">
          <label class="toggle-pill">
            <input v-model="userProfile.autoConnect" type="checkbox" />
            <span>保存后优先立即连接</span>
          </label>

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
            <span>SSH Host</span>
            <input v-model="companyProfile.sshHost" placeholder="127.0.0.1" />
          </label>

          <label class="field">
            <span>SSH User</span>
            <input v-model="companyProfile.sshUser" placeholder="root" />
          </label>

          <label class="field">
            <span>SSH Password</span>
            <input
              v-model="sshPasswordInput"
              type="password"
              placeholder="如已保存，可留空保持原值"
            />
          </label>

          <label class="field">
            <span>Local Port</span>
            <input
              v-model="companyProfile.localPort"
              inputmode="numeric"
              placeholder="18889"
            />
          </label>

          <label class="field">
            <span>Remote Bind Host</span>
            <input v-model="companyProfile.remoteBindHost" placeholder="61.150.94.14" />
          </label>

          <label class="field">
            <span>Remote Bind Port</span>
            <input
              v-model="companyProfile.remoteBindPort"
              inputmode="numeric"
              placeholder="18789"
            />
          </label>
        </div>

        <p v-if="advancedOpen" class="helper-text">
          如果服务器不支持密钥登录，可在这里保存 SSH 密码，启动托管时会自动复用。
        </p>

        <p class="helper-text" :data-tone="canSaveProfile ? 'success' : 'neutral'">
          {{ saveHint }}
        </p>
        <p v-if="profileError" class="error-banner">{{ profileError }}</p>

        <div class="button-row button-row--end">
          <button class="secondary-button" :disabled="!canSaveProfile" @click="saveOnly">
            仅保存配置
          </button>
          <button class="primary-button" :disabled="!canSaveAndConnect" @click="saveAndConnect">
            保存并立即连接
          </button>
        </div>
      </article>

      <article class="workspace-card surface-card">
        <div class="card-header">
          <div>
            <p class="eyebrow">Step 3</p>
            <h2>启动托管</h2>
          </div>
          <span class="status-chip" :data-tone="runtimeStatus.phase === 'running' ? 'success' : 'neutral'">
            {{ runtimeStatus.phase === 'running' ? '运行中' : '待命' }}
          </span>
        </div>

        <div class="support-grid support-grid--metrics">
          <div class="support-tile">
            <span>当前阶段</span>
            <strong>{{ phaseLabel(runtimeStatus.phase) }}</strong>
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
            <span>后台模式</span>
            <strong>{{ userProfile.runInBackground ? '已启用' : '未启用' }}</strong>
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
            :disabled="busyAction === 'stop' || runtimeStatus.phase !== 'running'"
            @click="stopHostedRuntime"
          >
            停止托管
          </button>
        </div>

        <p
          v-if="connectDisabledReason && runtimeStatus.phase !== 'running'"
          class="helper-text"
          data-tone="warning"
        >
          {{ connectDisabledReason }}
        </p>
        <p v-if="runtimeError" class="error-banner">{{ runtimeError }}</p>

        <div class="console-card console-card--logs">
          <div class="console-card__header">
            <span>最近日志</span>
            <span class="meta-text">{{ logs.length }} entries</span>
          </div>
          <ol class="log-list">
            <li v-for="entry in logPreview" :key="`${entry.timestampMs}-${entry.message}`">
              <time>{{ new Date(entry.timestampMs).toLocaleTimeString() }}</time>
              <strong>[{{ entry.source }}]</strong>
              <span>{{ entry.message }}</span>
            </li>
          </ol>
        </div>
      </article>
    </section>

    <footer class="environment-bar" role="status" aria-label="当前环境状态">
      <div class="environment-bar__header">
        <div class="environment-bar__title">
          <p class="eyebrow">Environment</p>
          <strong>当前环境</strong>
        </div>
        <span class="environment-bar__os">{{ environment?.os ?? '检测中' }}</span>
      </div>

      <ul class="environment-bar__list" aria-label="环境能力状态">
        <li
          v-for="item in environmentCards"
          :key="item.label"
          class="environment-pill"
          :data-tone="item.tone"
        >
          <span class="environment-pill__label">{{ item.label }}</span>
          <strong>{{ item.status }}</strong>
          <small class="environment-pill__detail" :title="item.detail">{{ item.detail }}</small>
        </li>
      </ul>
    </footer>
  </main>
</template>
