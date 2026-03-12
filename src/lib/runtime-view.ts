import type { EnvironmentSnapshot, RuntimePhase, RuntimeStatus } from '@/types'

export interface StepView {
  key: string
  title: string
  caption: string
  state: 'idle' | 'active' | 'complete' | 'error'
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
  const configured = Boolean(snapshot?.hasSavedProfile)

  return [
    {
      key: 'environment',
      title: '环境检测',
      caption: snapshot ? `OS ${snapshot.os}` : '等待首次检测',
      state: snapshot ? 'complete' : 'active',
    },
    {
      key: 'install',
      title: '安装 OpenClaw CLI',
      caption: installReady ? '已检测到 CLI' : '优先官方安装，失败后回退 npm / pnpm',
      state: status.phase === 'error' && !installReady
        ? 'error'
        : installReady
          ? 'complete'
          : status.phase === 'installing' || status.phase === 'manualWait' || status.phase === 'installNeeded'
            ? 'active'
            : 'idle',
    },
    {
      key: 'configure',
      title: '录入 BizClaw 连接配置',
      caption: configured ? '连接参数与显示名称已保存' : '填写显示名称、token 与 SSH 参数',
      state: status.phase === 'error' && installReady && !configured
        ? 'error'
        : configured
          ? 'complete'
          : installReady
            ? 'active'
            : 'idle',
    },
    {
      key: 'runtime',
      title: '托管连接',
      caption: status.phase === 'running'
        ? 'SSH 隧道与 OpenClaw Node 正在运行'
        : '支持立即连接与后台驻留',
      state: status.phase === 'error'
        ? 'error'
        : status.phase === 'running'
          ? 'complete'
          : status.phase === 'connecting'
            ? 'active'
            : configured
              ? 'idle'
              : 'idle',
    },
  ]
}

function defaultStatus(): RuntimeStatus {
  return {
    phase: 'checking',
    sshConnected: false,
    nodeConnected: false,
    lastError: null,
  }
}
