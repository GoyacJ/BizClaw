import { appendFileSync, existsSync, writeFileSync } from 'node:fs'
import { join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

export function normalizeTauriSigningKey(value) {
  const trimmed = value.trim()
  if (!trimmed) {
    throw new Error('TAURI_SIGNING_PRIVATE_KEY is empty.')
  }

  if (existsSync(trimmed)) {
    return resolve(trimmed)
  }

  let normalized = trimmed
  if (/%[0-9A-Fa-f]{2}/u.test(normalized)) {
    try {
      normalized = decodeURIComponent(normalized)
    } catch (error) {
      throw new Error(`TAURI_SIGNING_PRIVATE_KEY contains invalid URL-encoded content: ${error instanceof Error ? error.message : String(error)}`)
    }
  }

  normalized = normalized.replace(/\s+/gu, '')
  if (!/^[A-Za-z0-9+/=]+$/u.test(normalized)) {
    throw new Error('TAURI_SIGNING_PRIVATE_KEY contains unexpected characters. Store the exact key content without extra encoding.')
  }

  return normalized
}

if (process.argv[1] && fileURLToPath(import.meta.url) === resolve(process.argv[1])) {
  const rawKey = process.env.TAURI_SIGNING_PRIVATE_KEY ?? ''
  const runnerTemp = process.env.RUNNER_TEMP
  const githubEnv = process.env.GITHUB_ENV

  if (!runnerTemp || !githubEnv) {
    throw new Error('RUNNER_TEMP and GITHUB_ENV must be set when preparing the Tauri signing key.')
  }

  const normalized = normalizeTauriSigningKey(rawKey)
  const keyPath = existsSync(normalized)
    ? normalized
    : join(runnerTemp, 'tauri-updater.key')

  if (!existsSync(normalized)) {
    writeFileSync(keyPath, `${normalized}\n`)
  }

  appendFileSync(githubEnv, `TAURI_SIGNING_PRIVATE_KEY=${keyPath}\n`)
  console.log(`[INFO] Prepared Tauri signing key at ${keyPath}`)
}
