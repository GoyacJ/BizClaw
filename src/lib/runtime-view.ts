import type {
  EnvironmentSnapshot,
  OperationEvent,
  OperationTaskPhase,
  OperationTaskSnapshot,
  OperationStep,
  RuntimePhase,
  RuntimeStatus,
  RuntimeTarget,
} from '@/types'

export interface OperationsSummary {
  title: string
  detail: string
  tone: 'idle' | 'active' | 'complete' | 'error'
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

export function runtimeTargetLabel(target: RuntimeTarget): string {
  return target === 'windowsWsl' ? 'Windows WSL' : 'macOS 本机'
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

export function operationStepLabel(step: OperationStep): string {
  switch (step) {
    case 'detect':
      return '环境检测'
    case 'bootstrapWsl':
      return 'WSL 初始化'
    case 'ensureSsh':
      return 'OpenSSH 检查'
    case 'installOpenClaw':
      return '安装 OpenClaw'
    case 'checkUpdate':
      return '检查更新'
    case 'updateOpenClaw':
      return '更新 OpenClaw'
  }
}

export function operationTaskPhaseLabel(phase: OperationTaskPhase): string {
  switch (phase) {
    case 'idle':
      return '待命'
    case 'running':
      return '执行中'
    case 'cancelling':
      return '停止中'
    case 'success':
      return '已完成'
    case 'error':
      return '失败'
    case 'cancelled':
      return '已停止'
  }
}

export function buildOperationsSummary(
  snapshot: EnvironmentSnapshot | null,
  operationTask: OperationTaskSnapshot | null,
  busyAction: string | null,
): OperationsSummary {
  if (operationTask?.phase === 'running' && operationTask.kind === 'install') {
    return {
      title: '正在安装 OpenClaw',
      detail: 'BizClaw 正在后台执行官方安装器，页面可以继续操作。',
      tone: 'active',
    }
  }

  if (operationTask?.phase === 'running' && operationTask.kind === 'update') {
    return {
      title: '正在更新 OpenClaw',
      detail: '当前正在后台执行官方更新流程，完成后会重新检测版本信息。',
      tone: 'active',
    }
  }

  if (operationTask?.phase === 'cancelling') {
    return {
      title: '正在停止任务',
      detail: 'BizClaw 正在结束安装或更新进程，请稍候。',
      tone: 'active',
    }
  }

  if (operationTask?.phase === 'cancelled') {
    return {
      title: '任务已停止',
      detail: operationTask.lastResult?.followUp ?? '当前安装或更新任务已停止。',
      tone: 'active',
    }
  }

  if (busyAction === 'check-update') {
    return {
      title: '正在检查更新',
      detail: '正在获取当前版本与远端最新版本。',
      tone: 'active',
    }
  }

  if (operationTask?.phase === 'error' && operationTask.lastResult && !operationTask.lastResult.success) {
    return {
      title: '操作未完成',
      detail: operationTask.lastResult.followUp,
      tone: 'error',
    }
  }

  if (operationTask?.phase === 'success' && operationTask.lastResult?.success) {
    return {
      title: operationTask.kind === 'update' ? '更新已完成' : '安装已完成',
      detail: operationTask.lastResult.followUp,
      tone: 'complete',
    }
  }

  if (snapshot?.openclawInstalled) {
    return {
      title: snapshot.updateAvailable ? '检测到可用更新' : 'OpenClaw 已就绪',
      detail: snapshot.updateAvailable
        ? `当前 ${snapshot.openclawVersion ?? '未知版本'}，可更新到 ${snapshot.latestOpenclawVersion ?? '最新版本'}`
        : snapshot.openclawVersion ?? '已检测到 OpenClaw CLI',
      tone: snapshot.updateAvailable ? 'active' : 'complete',
    }
  }

  return {
    title: '尚未安装 OpenClaw',
    detail: snapshot?.installRecommendation ?? '请先检测环境并安装 OpenClaw。',
    tone: 'idle',
  }
}

export function latestOperationDetail(events: OperationEvent[]): string {
  return events[events.length - 1]?.message ?? '尚未执行安装或更新操作。'
}

export function startRuntimeDisabledReason(
  snapshot: EnvironmentSnapshot | null,
  hasBlockingTask: boolean,
): string | null {
  if (hasBlockingTask) {
    return '当前操作进行中，请稍候。'
  }

  if (!snapshot) {
    return '正在检测当前环境。'
  }

  if (snapshot.runtimeTarget === 'windowsWsl' && !snapshot.wslStatus?.ready) {
    return '请先完成 WSL / Ubuntu 初始化。'
  }

  if (!snapshot.targetSshInstalled) {
    return '请先补齐目标运行环境中的 OpenSSH。'
  }

  if (!snapshot.openclawInstalled) {
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

  if (snapshot.runtimeStatus.phase === 'connecting') {
    return '托管正在连接中。'
  }

  return null
}

export function runtimeDetail(status: RuntimeStatus): string {
  if (status.lastError) {
    return status.lastError
  }

  if (status.phase === 'running') {
    return `${status.sshConnected ? 'SSH 已连接' : 'SSH 未连接'} · ${status.nodeConnected ? 'Node 已连接' : 'Node 未连接'} · ${status.gatewayConnected ? 'Gateway 已连接' : 'Gateway 未连接'}`
  }

  if (status.phase === 'connecting') {
    return '正在建立 SSH 隧道并启动 OpenClaw Node。'
  }

  return '可使用已保存配置启动或停止托管。'
}
