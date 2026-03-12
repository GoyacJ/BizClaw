<script setup lang="ts">
import { computed } from 'vue'

import { phaseLabel } from '@/lib/runtime-view'
import { useAppModel } from '@/lib/use-app-model'

const {
  advancedOpen,
  busyAction,
  canConnect,
  canSaveProfile,
  companyProfile,
  environment,
  installCli,
  installResult,
  lastError,
  launchManualInstall,
  logs,
  refreshEnvironment,
  runtimeStatus,
  saveAndConnect,
  saveOnly,
  stopHostedRuntime,
  tokenInput,
  userProfile,
  workflow,
} = useAppModel()

const commandAvailability = computed(() => {
  const env = environment.value
  if (!env) {
    return []
  }

  return [
    { label: 'SSH', online: env.sshInstalled },
    { label: 'OpenClaw CLI', online: env.openclawInstalled },
    { label: 'npm', online: env.npmInstalled },
    { label: 'pnpm', online: env.pnpmInstalled },
  ]
})

const busyLabel = computed(() => {
  switch (busyAction.value) {
    case 'install':
      return '正在执行安装...'
    case 'manual-install':
      return '正在打开官方安装页...'
    case 'save':
      return '正在保存配置...'
    case 'connect':
      return '正在保存并连接...'
    case 'stop':
      return '正在停止托管运行时...'
    default:
      return ''
  }
})

const logPreview = computed(() => [...logs.value].reverse().slice(0, 16))
</script>

<template>
  <main class="app-shell">
    <aside class="control-rail">
      <section class="rail-panel rail-panel--hero">
        <p class="eyebrow">BizClaw</p>
        <h1>把安装、配置和托管运行压缩成一次入站流程。</h1>
        <p class="hero-copy">
          BizClaw 用一个桌面向导串起环境检查、OpenClaw 安装、连接配置保存与托管运行，避免脚本化接入里的重复手工操作。
        </p>
        <div class="status-strip">
          <span class="phase-pill">{{ phaseLabel(runtimeStatus.phase) }}</span>
          <span v-if="busyLabel" class="busy-pill">{{ busyLabel }}</span>
        </div>
      </section>

      <section class="rail-panel">
        <div class="section-heading">
          <span>当前环境</span>
          <button class="ghost-button" @click="refreshEnvironment">
            重新检测
          </button>
        </div>
        <div class="command-grid">
          <article
            v-for="command in commandAvailability"
            :key="command.label"
            class="command-card"
            :class="{ 'command-card--ok': command.online }"
          >
            <span>{{ command.label }}</span>
            <strong>{{ command.online ? '可用' : '缺失' }}</strong>
          </article>
        </div>
        <p v-if="environment" class="footnote">
          {{ environment.installRecommendation }}
        </p>
      </section>

      <section class="rail-panel">
        <div class="section-heading">
          <span>流程概览</span>
          <span class="meta-text">{{ workflow.length }} steps</span>
        </div>
        <ol class="workflow-list">
          <li
            v-for="step in workflow"
            :key="step.key"
            class="workflow-item"
            :data-state="step.state"
          >
            <strong>{{ step.title }}</strong>
            <span>{{ step.caption }}</span>
          </li>
        </ol>
      </section>
    </aside>

    <section class="workspace">
      <article class="panel">
        <div class="panel-header">
          <div>
            <p class="eyebrow">Step 1</p>
            <h2>安装 OpenClaw CLI</h2>
          </div>
          <span class="meta-text">自动安装优先官方脚本</span>
        </div>
        <p class="section-copy">
          如果当前机器还没有 `openclaw`，可以直接触发自动安装，也可以跳转到官方文档按原始流程手动执行。
        </p>
        <div class="button-row">
          <button class="primary-button" @click="installCli">
            自动安装
          </button>
          <button class="secondary-button" @click="launchManualInstall">
            手动安装
          </button>
        </div>
        <div v-if="installResult" class="result-console">
          <div class="result-meta">
            <strong>策略: {{ installResult.strategy }}</strong>
            <span>{{ installResult.followUp }}</span>
          </div>
          <pre>{{ installResult.stdout || installResult.stderr || '等待安装输出...' }}</pre>
        </div>
      </article>

      <article class="panel panel--split">
        <div class="panel-header">
          <div>
            <p class="eyebrow">Step 2</p>
            <h2>录入 BizClaw 连接配置</h2>
          </div>
          <span class="meta-text">
            Token {{ environment?.hasSavedToken ? '已保存到系统钥匙串' : '尚未保存' }}
          </span>
        </div>

        <div class="form-grid">
          <label class="field">
            <span>显示名称</span>
            <input v-model="userProfile.displayName" placeholder="填写当前设备显示名称" />
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
          <label class="toggle">
            <input v-model="userProfile.autoConnect" type="checkbox" />
            <span>保存后倾向立即连接</span>
          </label>
          <label class="toggle">
            <input v-model="userProfile.runInBackground" type="checkbox" />
            <span>关闭窗口后继续后台运行</span>
          </label>
        </div>

        <button class="ghost-button" @click="advancedOpen = !advancedOpen">
          {{ advancedOpen ? '收起高级参数' : '展开高级参数' }}
        </button>

        <div v-if="advancedOpen" class="advanced-grid">
          <label class="field">
            <span>SSH Host</span>
            <input v-model="companyProfile.sshHost" placeholder="填写 SSH 主机地址" />
          </label>
          <label class="field">
            <span>SSH User</span>
            <input v-model="companyProfile.sshUser" placeholder="填写 SSH 用户名" />
          </label>
          <label class="field">
            <span>Local Port</span>
            <input
              v-model="companyProfile.localPort"
              inputmode="numeric"
              placeholder="填写本地转发端口"
            />
          </label>
          <label class="field">
            <span>Remote Bind Host</span>
            <input v-model="companyProfile.remoteBindHost" placeholder="填写远端绑定地址" />
          </label>
          <label class="field">
            <span>Remote Bind Port</span>
            <input
              v-model="companyProfile.remoteBindPort"
              inputmode="numeric"
              placeholder="填写远端绑定端口"
            />
          </label>
        </div>

        <p v-if="!canSaveProfile" class="footnote">
          需要填写完整的显示名称、SSH 参数和 token，才可以保存或启动托管连接。
        </p>

        <div class="button-row">
          <button class="secondary-button" :disabled="!canSaveProfile" @click="saveOnly">
            仅保存配置
          </button>
          <button class="primary-button" :disabled="!canConnect" @click="saveAndConnect">
            保存并立即连接
          </button>
        </div>
      </article>

      <article class="panel panel--runtime">
        <div class="panel-header">
          <div>
            <p class="eyebrow">Step 3</p>
            <h2>托管运行时</h2>
          </div>
          <span class="meta-text">
            SSH {{ runtimeStatus.sshConnected ? 'Up' : 'Idle' }} · Node
            {{ runtimeStatus.nodeConnected ? 'Up' : 'Idle' }}
          </span>
        </div>

        <div class="runtime-grid">
          <div class="runtime-metric">
            <span>当前阶段</span>
            <strong>{{ phaseLabel(runtimeStatus.phase) }}</strong>
          </div>
          <div class="runtime-metric">
            <span>后台模式</span>
            <strong>{{ userProfile.runInBackground ? '已启用' : '未启用' }}</strong>
          </div>
        </div>

        <div class="button-row">
          <button class="primary-button" :disabled="!canConnect" @click="saveAndConnect">
            启动托管
          </button>
          <button class="secondary-button" @click="stopHostedRuntime">
            停止托管
          </button>
        </div>

        <p v-if="runtimeStatus.lastError" class="error-banner">
          {{ runtimeStatus.lastError }}
        </p>
        <p v-if="lastError" class="error-banner">
          {{ lastError }}
        </p>

        <div class="log-console">
          <div class="log-console__header">
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
  </main>
</template>
