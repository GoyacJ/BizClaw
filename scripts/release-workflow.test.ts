// @vitest-environment node

import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

import { describe, expect, it } from 'vitest'

const rootDir = resolve(fileURLToPath(new URL('..', import.meta.url)))
const workflowPath = resolve(rootDir, '.github', 'workflows', 'release.yml')
const packageJsonPath = resolve(rootDir, 'package.json')

const workflow = readFileSync(workflowPath, 'utf8')
const packageJson = JSON.parse(readFileSync(packageJsonPath, 'utf8')) as {
  packageManager?: string
}

const releaseJobs = ['verify-version', 'build-macos', 'build-windows']
const workflowJobs = [...releaseJobs, 'publish-release']

describe('release workflow', () => {
  it('pins pnpm through package.json', () => {
    expect(packageJson.packageManager).toBe('pnpm@10.11.0')
  })

  it.each(releaseJobs)('%s installs pnpm via corepack instead of the deprecated pnpm action', (jobName) => {
    const jobBlock = readJobBlock(jobName)

    expect(jobBlock).toMatch(/uses:\s*actions\/setup-node@v6/u)
    expect(jobBlock).toMatch(/run:\s*corepack enable/u)
    expect(jobBlock).toMatch(/corepack prepare pnpm@10\.11\.0 --activate/u)
    expect(jobBlock).not.toMatch(/pnpm\/action-setup@/u)
    expect(jobBlock).not.toMatch(/cache:\s*pnpm/u)

    const setupNodeIndex = jobBlock.indexOf('uses: actions/setup-node@v6')
    const corepackEnableIndex = jobBlock.indexOf('run: corepack enable')
    const corepackPrepareIndex = jobBlock.indexOf('corepack prepare pnpm@10.11.0 --activate')

    expect(setupNodeIndex).toBeGreaterThanOrEqual(0)
    expect(corepackEnableIndex).toBeGreaterThan(setupNodeIndex)
    expect(corepackPrepareIndex).toBeGreaterThan(corepackEnableIndex)
  })

  it.each(releaseJobs)('%s uses a node24-compatible checkout action', (jobName) => {
    expect(readJobBlock(jobName)).toMatch(/uses:\s*actions\/checkout@v6/u)
  })

  it('uses node24-compatible artifact actions', () => {
    expect(readJobBlock('build-macos')).toMatch(/uses:\s*actions\/upload-artifact@v6/u)
    expect(readJobBlock('build-windows')).toMatch(/uses:\s*actions\/upload-artifact@v6/u)
    expect(readJobBlock('publish-release')).toMatch(/uses:\s*actions\/download-artifact@v7/u)
  })

  it('publishes releases without relying on a checked out git repository', () => {
    const jobBlock = readJobBlock('publish-release')

    expect(jobBlock).toMatch(/gh release view "\$\{GITHUB_REF_NAME\}" --repo "\$\{GITHUB_REPOSITORY\}"/u)
    expect(jobBlock).toMatch(/gh release create "\$\{GITHUB_REF_NAME\}"[\s\S]*--repo "\$\{GITHUB_REPOSITORY\}"/u)
    expect(jobBlock).toMatch(/gh release upload "\$\{GITHUB_REF_NAME\}" --repo "\$\{GITHUB_REPOSITORY\}"/u)
    expect(jobBlock).not.toMatch(/uses:\s*actions\/checkout@/u)
  })

  it('uploads only downloaded release files, not artifact directories', () => {
    const jobBlock = readJobBlock('publish-release')

    expect(jobBlock).toMatch(/find release-artifacts -type f/u)
    expect(jobBlock).toMatch(/mapfile -d '' assets/u)
    expect(jobBlock).toMatch(/gh release upload "\$\{GITHUB_REF_NAME\}" --repo "\$\{GITHUB_REPOSITORY\}" "\$\{assets\[@\]\}" --clobber/u)
    expect(jobBlock).not.toMatch(/release-artifacts\/\*/u)
  })

  it('preinstalls WiX before building the Windows MSI', () => {
    const jobBlock = readJobBlock('build-windows')

    expect(jobBlock).toMatch(/choco install wixtoolset/u)
    expect(jobBlock).toMatch(/WiX Toolset v3\.14\\bin/u)
    expect(jobBlock).toMatch(/"WIX=\$wixBin"\s*\|\s*Out-File\s+-FilePath \$env:GITHUB_ENV/u)
    expect(jobBlock).toMatch(/pnpm tauri build --bundles msi/u)

    const wixInstallIndex = jobBlock.indexOf('choco install wixtoolset')
    const msiBuildIndex = jobBlock.indexOf('pnpm tauri build --bundles msi')

    expect(wixInstallIndex).toBeGreaterThanOrEqual(0)
    expect(msiBuildIndex).toBeGreaterThan(wixInstallIndex)
  })

  it('packages a portable Windows zip alongside the MSI', () => {
    const jobBlock = readJobBlock('build-windows')

    expect(jobBlock).toMatch(/src-tauri\/target\/release\/bizclaw\.exe/u)
    expect(jobBlock).toMatch(/Compress-Archive/u)
    expect(jobBlock).toMatch(/bundle\/portable/u)
    expect(jobBlock).toMatch(/\.zip/u)
    expect(jobBlock).toMatch(/bundle\/msi\/\*\.msi[\s\S]*bundle\/portable\/\*\.zip/u)
  })
})

function readJobBlock(jobName: string) {
  const marker = `  ${jobName}:\n`
  const startIndex = workflow.indexOf(marker)

  if (startIndex < 0) {
    throw new Error(`Could not find job block for ${jobName}`)
  }

  const nextJobStartIndex = workflowJobs
    .map((name) => (name === jobName ? -1 : workflow.indexOf(`  ${name}:\n`, startIndex + marker.length)))
    .filter((index) => index > startIndex)
    .sort((left, right) => left - right)[0]

  return workflow.slice(startIndex, nextJobStartIndex === undefined ? workflow.length : nextJobStartIndex)
}
