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

  it.each(releaseJobs)('%s installs pnpm before enabling setup-node cache', (jobName) => {
    const jobBlock = readJobBlock(jobName)

    expect(jobBlock).toMatch(/uses:\s*pnpm\/action-setup@v4/u)
    expect(jobBlock).toMatch(/uses:\s*actions\/setup-node@v6/u)
    expect(jobBlock).toMatch(/cache:\s*pnpm/u)

    const pnpmSetupIndex = jobBlock.indexOf('uses: pnpm/action-setup@v4')
    const setupNodeIndex = jobBlock.indexOf('uses: actions/setup-node@v6')

    expect(pnpmSetupIndex).toBeGreaterThanOrEqual(0)
    expect(setupNodeIndex).toBeGreaterThan(pnpmSetupIndex)
  })

  it.each(releaseJobs)('%s uses a node24-compatible checkout action', (jobName) => {
    expect(readJobBlock(jobName)).toMatch(/uses:\s*actions\/checkout@v6/u)
  })

  it('uses node24-compatible artifact actions', () => {
    expect(readJobBlock('build-macos')).toMatch(/uses:\s*actions\/upload-artifact@v6/u)
    expect(readJobBlock('build-windows')).toMatch(/uses:\s*actions\/upload-artifact@v6/u)
    expect(readJobBlock('publish-release')).toMatch(/uses:\s*actions\/download-artifact@v7/u)
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
