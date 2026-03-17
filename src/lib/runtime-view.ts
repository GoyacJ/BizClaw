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
import { appLocaleRef, translate } from '@/lib/i18n'

export interface OperationsSummary {
  title: string
  detail: string
  tone: 'idle' | 'active' | 'complete' | 'error'
}

export function phaseLabel(phase: RuntimePhase): string {
  void appLocaleRef.value
  switch (phase) {
    case 'checking':
      return translate('runtime.phase.checking')
    case 'installNeeded':
      return translate('runtime.phase.installNeeded')
    case 'installing':
      return translate('runtime.phase.installing')
    case 'manualWait':
      return translate('runtime.phase.manualWait')
    case 'configured':
      return translate('runtime.phase.configured')
    case 'connecting':
      return translate('runtime.phase.connecting')
    case 'running':
      return translate('runtime.phase.running')
    case 'error':
      return translate('runtime.phase.error')
  }
}

export function runtimeTargetLabel(target: RuntimeTarget): string {
  void appLocaleRef.value
  return translate(`runtime.target.${target}`)
}

export function tokenStatusLabel(snapshot: EnvironmentSnapshot | null): string {
  void appLocaleRef.value
  switch (snapshot?.tokenStatus) {
    case 'saved':
      return translate('runtime.token.saved')
    case 'error':
      return translate('runtime.token.error')
    default:
      return translate('runtime.token.missing')
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
  void appLocaleRef.value
  switch (step) {
    case 'detect':
      return translate('runtime.operationStep.detect')
    case 'bootstrapWsl':
      return translate('runtime.operationStep.bootstrapWsl')
    case 'ensureSsh':
      return translate('runtime.operationStep.ensureSsh')
    case 'installOpenClaw':
      return translate('runtime.operationStep.installOpenClaw')
    case 'checkUpdate':
      return translate('runtime.operationStep.checkUpdate')
    case 'updateOpenClaw':
      return translate('runtime.operationStep.updateOpenClaw')
  }
}

export function operationTaskPhaseLabel(phase: OperationTaskPhase): string {
  void appLocaleRef.value
  switch (phase) {
    case 'idle':
      return translate('runtime.operationPhase.idle')
    case 'running':
      return translate('runtime.operationPhase.running')
    case 'cancelling':
      return translate('runtime.operationPhase.cancelling')
    case 'success':
      return translate('runtime.operationPhase.success')
    case 'error':
      return translate('runtime.operationPhase.error')
    case 'cancelled':
      return translate('runtime.operationPhase.cancelled')
  }
}

export function buildOperationsSummary(
  snapshot: EnvironmentSnapshot | null,
  operationTask: OperationTaskSnapshot | null,
  busyAction: string | null,
): OperationsSummary {
  void appLocaleRef.value
  if (operationTask?.phase === 'running' && operationTask.kind === 'install') {
    return {
      title: translate('runtime.operationsSummary.installingTitle'),
      detail: translate('runtime.operationsSummary.installingDetail'),
      tone: 'active',
    }
  }

  if (operationTask?.phase === 'running' && operationTask.kind === 'checkUpdate') {
    return {
      title: translate('runtime.operationsSummary.checkingTitle'),
      detail: translate('runtime.operationsSummary.checkingDetail'),
      tone: 'active',
    }
  }

  if (operationTask?.phase === 'running' && operationTask.kind === 'update') {
    return {
      title: translate('runtime.operationsSummary.updatingTitle'),
      detail: translate('runtime.operationsSummary.updatingDetail'),
      tone: 'active',
    }
  }

  if (operationTask?.phase === 'cancelling') {
    return {
      title: translate('runtime.operationsSummary.cancellingTitle'),
      detail: translate('runtime.operationsSummary.cancellingDetail'),
      tone: 'active',
    }
  }

  if (operationTask?.phase === 'cancelled') {
    return {
      title: translate('runtime.operationsSummary.cancelledTitle'),
      detail: operationTask.lastResult?.followUp ?? translate('runtime.operationsSummary.cancelledDetail'),
      tone: 'active',
    }
  }

  if (busyAction === 'check-update') {
    return {
      title: translate('runtime.operationsSummary.checkingTitle'),
      detail: translate('runtime.operationsSummary.checkingDetail'),
      tone: 'active',
    }
  }

  if (operationTask?.phase === 'error' && operationTask.lastResult && !operationTask.lastResult.success) {
    return {
      title: translate('runtime.operationsSummary.failedTitle'),
      detail: operationTask.lastResult.followUp,
      tone: 'error',
    }
  }

  if (operationTask?.phase === 'success' && operationTask.lastResult?.success) {
    return {
      title: operationTask.kind === 'checkUpdate'
        ? snapshot?.updateAvailable
          ? translate('runtime.operationsSummary.updateReady')
          : translate('runtime.operationsSummary.installedReady')
        : operationTask.kind === 'update'
          ? translate('runtime.operationsSummary.completedUpdate')
          : translate('runtime.operationsSummary.completedInstall'),
      detail: operationTask.lastResult.followUp,
      tone: 'complete',
    }
  }

  if (snapshot?.openclawInstalled) {
    return {
      title: snapshot.updateAvailable
        ? translate('runtime.operationsSummary.updateReady')
        : translate('runtime.operationsSummary.installedReady'),
      detail: snapshot.updateAvailable
        ? translate('bizclaw.detail.available', {
          current: snapshot.openclawVersion ?? translate('common.unknown'),
          latest: snapshot.latestOpenclawVersion ?? translate('common.latest'),
        })
        : snapshot.openclawVersion ?? translate('overview.detectedOpenClaw'),
      tone: snapshot.updateAvailable ? 'active' : 'complete',
    }
  }

  return {
    title: translate('runtime.operationsSummary.installMissing'),
    detail: snapshot?.installRecommendation ?? translate('runtime.operationsSummary.installMissingDetail'),
    tone: 'idle',
  }
}

export function latestOperationDetail(events: OperationEvent[]): string {
  void appLocaleRef.value
  return events[events.length - 1]?.message ?? translate('runtime.operationsSummary.latestOperationEmpty')
}

export function startRuntimeDisabledReason(
  snapshot: EnvironmentSnapshot | null,
  hasBlockingTask: boolean,
): string | null {
  void appLocaleRef.value
  if (hasBlockingTask) {
    return translate('runtime.startDisabled.blocking')
  }

  if (!snapshot) {
    return null
  }

  if (snapshot.runtimeTarget === 'windowsWsl' && !snapshot.wslStatus?.ready) {
    return translate('runtime.startDisabled.wsl')
  }

  if (!snapshot.targetSshInstalled) {
    return translate('runtime.startDisabled.ssh')
  }

  if (!snapshot.openclawInstalled) {
    return translate('runtime.startDisabled.openclaw')
  }

  if (!snapshot.hasSavedProfile) {
    return translate('runtime.startDisabled.profile')
  }

  if (snapshot.tokenStatus === 'error') {
    return snapshot.tokenStatusMessage ?? translate('runtime.startDisabled.tokenError')
  }

  if (snapshot.tokenStatus !== 'saved') {
    return translate('runtime.startDisabled.tokenMissing')
  }

  if (snapshot.runtimeStatus.phase === 'running') {
    return translate('runtime.startDisabled.alreadyRunning')
  }

  if (snapshot.runtimeStatus.phase === 'connecting') {
    return translate('runtime.startDisabled.connecting')
  }

  return null
}

export function runtimeDetail(status: RuntimeStatus): string {
  void appLocaleRef.value
  if (status.lastError) {
    return status.lastError
  }

  if (status.phase === 'running') {
    return `${status.sshConnected ? translate('runtime.detail.sshConnected') : translate('runtime.detail.sshDisconnected')} · ${status.nodeConnected ? translate('runtime.detail.nodeConnected') : translate('runtime.detail.nodeDisconnected')} · ${status.gatewayConnected ? translate('runtime.detail.gatewayConnected') : translate('runtime.detail.gatewayDisconnected')}`
  }

  if (status.phase === 'connecting') {
    return translate('runtime.detail.connecting')
  }

  return translate('runtime.detail.idle')
}
