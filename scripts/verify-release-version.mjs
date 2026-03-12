import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'

const rootDir = process.cwd()
const rawTag = process.argv[2] ?? ''
const expectedVersion = rawTag
  ? rawTag.replace(/^refs\/tags\//u, '').replace(/^v/u, '')
  : null

const packageJsonPath = resolve(rootDir, 'package.json')
const tauriConfigPath = resolve(rootDir, 'src-tauri', 'tauri.conf.json')
const cargoTomlPath = resolve(rootDir, 'src-tauri', 'Cargo.toml')

const packageVersion = JSON.parse(readFileSync(packageJsonPath, 'utf8')).version
const tauriVersion = JSON.parse(readFileSync(tauriConfigPath, 'utf8')).version
const cargoVersion = readCargoPackageVersion(readFileSync(cargoTomlPath, 'utf8'))

const mismatches = []

if (packageVersion !== tauriVersion) {
  mismatches.push(`package.json=${packageVersion} but tauri.conf.json=${tauriVersion}`)
}

if (packageVersion !== cargoVersion) {
  mismatches.push(`package.json=${packageVersion} but Cargo.toml=${cargoVersion}`)
}

if (expectedVersion && packageVersion !== expectedVersion) {
  mismatches.push(`tag=v${expectedVersion} but package.json=${packageVersion}`)
}

if (mismatches.length > 0) {
  console.error('[ERROR] 版本校验失败:')
  for (const mismatch of mismatches) {
    console.error(`- ${mismatch}`)
  }
  process.exit(1)
}

console.log(`[INFO] 版本一致: ${packageVersion}`)

function readCargoPackageVersion(tomlContent) {
  const packageSection = tomlContent.match(/\[package\][\s\S]*?(?:\n\[|$)/u)?.[0]
  const versionMatch = packageSection?.match(/^\s*version\s*=\s*"([^"]+)"/mu)
  if (!versionMatch) {
    throw new Error('无法从 src-tauri/Cargo.toml 读取 [package] version')
  }

  return versionMatch[1]
}
