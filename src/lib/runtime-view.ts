import type {
  EnvironmentSnapshot,
  InstallResult,
  RuntimePhase,
  RuntimeStatus,
} from '@/types'

export interface StepView {
  key: string
  title: string
  caption: string
  state: 'idle' | 'active' | 'complete' | 'error'
}

export interface InstallSummary {
  title: string
  detail: string
  tone: StepView['state']
}

export function phaseLabel(phase: RuntimePhase): string {
  switch (phase) {
    case 'checking':
      return '环境检查中'
    case 'installNeeded':
      return '待安装'
    case 'installing':
      return '安装中'
    case 'manualWait':
      return '等待手动安装'
    case 'configured':
      return '已配置'
    case 'connecting':
      return '连接中'
    case 'running':
      return '运行中'
    case 'error':
      return '异常'
  }
}

export function buildWorkflow(snapshot: EnvironmentSnapshot | null): StepView[] {
  const status = snapshot?.runtimeStatus ?? defaultStatus()
  const installReady = Boolean(snapshot?.openclawInstalled)
  const configReady = hasSavedConnection(snapshot)

  return [
    {
      key: 'install',
      title: '安装 OpenClaw',
      caption: installReady
        ? snapshot?.openclawVersion ?? 'CLI 已可用'
        : '检测并安装 OpenClaw CLI',
      state: status.phase === 'error' && !installReady
        ? 'error'
        : installReady
          ? 'complete'
          : status.phase === 'checking'
            || status.phase === 'installNeeded'
            || status.phase === 'installing'
            || status.phase === 'manualWait'
            ? 'active'
            : 'idle',
    },
    {
      key: 'configure',
      title: '保存连接配置',
      caption: configReady
        ? '显示名、SSH 与 Token 已保存'
        : snapshot?.tokenStatus === 'error'
          ? 'Token 存储异常，需要重新保存'
          : '填写显示名、SSH 参数和 Gateway Token',
      state: snapshot?.tokenStatus === 'error'
        ? 'error'
        : configReady
          ? 'complete'
          : installReady
            ? 'active'
            : 'idle',
    },
    {
      key: 'runtime',
      title: '启动托管',
      caption: status.phase === 'running'
        ? 'SSH 隧道与 OpenClaw Node 正在运行'
        : status.phase === 'connecting'
          ? '正在建立托管连接'
          : '使用已保存配置启动或停止托管',
      state: status.phase === 'error'
        ? 'error'
        : status.phase === 'running'
          ? 'complete'
          : status.phase === 'connecting'
            ? 'active'
            : configReady
              ? 'idle'
              : 'idle',
    },
  ]
}

export function tokenStatusLabel(snapshot: EnvironmentSnapshot | null): string {
  switch (snapshot?.tokenStatus) {
    case 'saved':
      return 'Token 已保存'
    case 'error':
      return 'Token 存储异常'
    default:
      return 'Token 未保存'
  }
}

export function tokenStatusTone(
  snapshot: EnvironmentSnapshot | null,
): 'neutral' | 'success' | 'error' {
  switch (snapshot?.tokenStatus) {
    case 'saved':
      return 'success'
    case 'error':
      return 'error'
    default:
      return 'neutral'
  }
}

export function buildInstallSummary(
  snapshot: EnvironmentSnapshot | null,
  installResult: InstallResult | null,
  busyAction: string | null,
): InstallSummary {
  if (busyAction === 'install') {
    return {
      title: '正在安装 OpenClaw',
      detail: '优先执行官方脚本，完成后会自动刷新检测结果。',
      tone: 'active',
    }
  }

  if (snapshot?.openclawInstalled) {
    return {
      title: 'OpenClaw 已安装',
      detail: snapshot.openclawVersion ?? '已检测到 OpenClaw CLI，可直接进入连接配置。',
      tone: 'complete',
    }
  }

  if (installResult && !installResult.success) {
    return {
      title: '安装未完成',
      detail: installResult.followUp,
      tone: 'error',
    }
  }

  if (installResult && installResult.strategy !== 'skipped') {
    return {
      title: '安装命令已执行',
      detail: installResult.followUp,
      tone: 'active',
    }
  }

  return {
    title: '尚未安装 OpenClaw',
    detail: '可直接自动安装，也可以打开官方文档手动完成安装。',
    tone: 'idle',
  }
}

export function shouldShowInstallConsole(
  installResult: InstallResult | null,
): boolean {
  return Boolean(installResult && installResult.strategy !== 'skipped')
}

export function startRuntimeDisabledReason(
  snapshot: EnvironmentSnapshot | null,
  busyAction: string | null,
): string | null {
  if (busyAction) {
    return '当前操作进行中，请稍候。'
  }

  if (!snapshot?.openclawInstalled) {
    return '请先安装 OpenClaw CLI。'
  }

  if (!snapshot.hasSavedProfile) {
    return '请先保存连接配置。'
  }

  if (snapshot.tokenStatus === 'error') {
    return snapshot.tokenStatusMessage ?? 'Token 存储异常，请重新保存。'
  }

  if (snapshot.tokenStatus !== 'saved') {
    return '请先保存 Gateway Token。'
  }

  if (snapshot.runtimeStatus.phase === 'running') {
    return '托管已在运行。'
  }

  return null
}

export function hasSavedConnection(snapshot: EnvironmentSnapshot | null): boolean {
  return Boolean(snapshot?.hasSavedProfile && snapshot.tokenStatus === 'saved')
}

function defaultStatus(): RuntimeStatus {
  return {
    phase: 'checking',
    sshConnected: false,
    nodeConnected: false,
    lastError: null,
  }
}
