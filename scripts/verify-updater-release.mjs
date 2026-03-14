import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const requiredAssetPatterns = [
  { label: 'latest.json', pattern: /^latest\.json$/u },
  { label: 'macOS updater archive', pattern: /\.app\.tar\.gz$/u },
  { label: 'macOS updater signature', pattern: /\.app\.tar\.gz\.sig$/u },
  { label: 'Windows installer', pattern: /\.msi$/u },
  { label: 'Windows installer signature', pattern: /\.msi\.sig$/u },
]

export function assertUpdaterReleaseAssets(release) {
  const assets = Array.isArray(release?.assets) ? release.assets : []

  for (const requirement of requiredAssetPatterns) {
    const found = assets.some((asset) => (
      typeof asset?.name === 'string' && requirement.pattern.test(asset.name)
    ))

    if (!found) {
      throw new Error(`Published GitHub Release is missing ${requirement.label}.`)
    }
  }
}

if (process.argv[1] && fileURLToPath(import.meta.url) === resolve(process.argv[1])) {
  const [, , releaseJsonPath] = process.argv

  if (!releaseJsonPath) {
    throw new Error('Usage: node ./scripts/verify-updater-release.mjs <release-json-path>')
  }

  const release = JSON.parse(readFileSync(resolve(releaseJsonPath), 'utf8'))
  assertUpdaterReleaseAssets(release)
  console.log('[INFO] updater release assets verified')
}
