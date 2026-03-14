import { getVersion } from '@tauri-apps/api/app'
import { relaunch } from '@tauri-apps/plugin-process'
import { check } from '@tauri-apps/plugin-updater'

const missingReleaseManifestPattern = /Could not fetch a valid release JSON from the remote/iu

function canUseTauriPlugins() {
  return typeof (globalThis as { __TAURI_INTERNALS__?: { invoke?: unknown } }).__TAURI_INTERNALS__?.invoke === 'function'
}

export interface BizClawUpdateDownloadStartedEvent {
  event: 'Started'
  data: {
    contentLength?: number
  }
}

export interface BizClawUpdateDownloadProgressEvent {
  event: 'Progress'
  data: {
    chunkLength?: number
  }
}

export interface BizClawUpdateDownloadFinishedEvent {
  event: 'Finished'
}

export type BizClawUpdateDownloadEvent =
  | BizClawUpdateDownloadStartedEvent
  | BizClawUpdateDownloadProgressEvent
  | BizClawUpdateDownloadFinishedEvent

export interface PendingBizClawUpdate {
  version: string
  body: string | null
  date: string | null
  downloadAndInstall: (
    onEvent?: (event: BizClawUpdateDownloadEvent) => void,
  ) => Promise<void>
}

export async function getCurrentBizClawVersion() {
  if (!canUseTauriPlugins()) {
    return null
  }
  return getVersion()
}

export function describeBizClawUpdaterError(error: unknown) {
  const message = error instanceof Error ? error.message : String(error)

  if (missingReleaseManifestPattern.test(message)) {
    return '无法读取 GitHub Release 的更新清单，请确认最新 Release 已上传 latest.json、签名文件和对应安装包。'
  }

  return message
}

export async function checkForBizClawUpdate(): Promise<PendingBizClawUpdate | null> {
  if (!canUseTauriPlugins()) {
    return null
  }
  const update = await check()
  if (!update) {
    return null
  }

  return {
    version: update.version,
    body: update.body ?? null,
    date: update.date ?? null,
    downloadAndInstall: update.downloadAndInstall.bind(update),
  }
}

export async function relaunchBizClaw() {
  if (!canUseTauriPlugins()) {
    return
  }
  await relaunch()
}
