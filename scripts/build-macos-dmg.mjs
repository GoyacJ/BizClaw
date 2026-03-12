import { cpSync, existsSync, mkdirSync, renameSync, rmSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { execFileSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'

const rootDir = dirname(dirname(fileURLToPath(import.meta.url)))
const appName = 'BizClaw.app'
const stagedName = 'BizClaw.app'
const appSource = join(
  rootDir,
  'src-tauri',
  'target',
  'release',
  'bundle',
  'macos',
  appName,
)
const dmgOutputDir = join(
  rootDir,
  'src-tauri',
  'target',
  'release',
  'bundle',
  'dmg',
)
const dmgOutput = join(dmgOutputDir, 'BizClaw-macos.dmg')
const stagingDir = '/tmp/bizclaw-dmg-test'
const tmpDmg = '/tmp/bizclaw-ascii.dmg'

if (!existsSync(appSource)) {
  console.error(`[ERROR] 未找到应用包: ${appSource}`)
  console.error('[INFO] 请先执行: pnpm tauri build --bundles app')
  process.exit(1)
}

rmSync(stagingDir, { force: true, recursive: true })
mkdirSync(stagingDir, { recursive: true })
mkdirSync(dmgOutputDir, { recursive: true })
cpSync(appSource, join(stagingDir, stagedName), { recursive: true })
rmSync(tmpDmg, { force: true })
rmSync(dmgOutput, { force: true })

execFileSync(
  'hdiutil',
  [
    'create',
    '-fs',
    'HFS+',
    '-volname',
    'BizClaw',
    '-srcfolder',
    stagingDir,
    '-ov',
    '-format',
    'UDZO',
    tmpDmg,
  ],
  { stdio: 'inherit' },
)

renameSync(tmpDmg, dmgOutput)
rmSync(stagingDir, { force: true, recursive: true })
console.log(`[INFO] DMG 已生成: ${dmgOutput}`)
