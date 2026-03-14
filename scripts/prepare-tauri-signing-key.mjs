import { appendFileSync, existsSync, readFileSync, writeFileSync } from 'node:fs'
import { join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

export function normalizeTauriSigningKey(value) {
  let normalized = value.trim()
  if (!normalized) {
    throw new Error('TAURI_SIGNING_PRIVATE_KEY is empty.')
  }

  if (existsSync(normalized)) {
    normalized = readFileSync(resolve(normalized), 'utf8').trim()
  }

  if (/%[0-9A-Fa-f]{2}/u.test(normalized)) {
    try {
      normalized = decodeURIComponent(normalized)
    } catch (error) {
      throw new Error(`TAURI_SIGNING_PRIVATE_KEY contains invalid URL-encoded content: ${error instanceof Error ? error.message : String(error)}`)
    }
  }

  normalized = normalized.replace(/^['"]|['"]$/gu, '')
  normalized = normalized.replace(/\\r\\n/gu, '\n').replace(/\\n/gu, '\n').replace(/\\r/gu, '\r')
  normalized = normalized.trim()
  if (normalized.startsWith('untrusted comment:')) {
    return Buffer.from(normalized.endsWith('\n') ? normalized : `${normalized}\n`).toString('base64')
  }

  const compact = normalized.replace(/\s+/gu, '')
  if (!/^[A-Za-z0-9+/=]+$/u.test(compact)) {
    throw new Error('TAURI_SIGNING_PRIVATE_KEY contains unexpected characters. Store the exact key content without extra encoding.')
  }

  const decoded = Buffer.from(compact, 'base64').toString('utf8')
  if (!decoded.startsWith('untrusted comment:')) {
    throw new Error('TAURI_SIGNING_PRIVATE_KEY did not decode to a valid minisign key file or canonical key string.')
  }

  return compact
}

if (process.argv[1] && fileURLToPath(import.meta.url) === resolve(process.argv[1])) {
  try {
    const rawKey = process.env.TAURI_SIGNING_PRIVATE_KEY ?? ''
    const runnerTemp = process.env.RUNNER_TEMP
    const githubEnv = process.env.GITHUB_ENV

    if (!runnerTemp || !githubEnv) {
      throw new Error('RUNNER_TEMP and GITHUB_ENV must be set when preparing the Tauri signing key.')
    }

    const normalized = normalizeTauriSigningKey(rawKey)
    const keyPath = join(runnerTemp, 'tauri-updater.key')

    writeFileSync(keyPath, normalized)
    appendFileSync(githubEnv, `TAURI_SIGNING_PRIVATE_KEY=${keyPath}\n`)
    console.log(`[INFO] Prepared Tauri signing key at ${keyPath}`)
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error)
    console.error(`::error title=Prepare updater signing key::${message}`)
    throw error
  }
}
