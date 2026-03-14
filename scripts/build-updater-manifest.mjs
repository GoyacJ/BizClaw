import { readdirSync, readFileSync, writeFileSync } from 'node:fs'
import { basename, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

function releaseDownloadUrl(repository, releaseTag, assetName) {
  return `https://github.com/${repository}/releases/download/${releaseTag}/${assetName}`
}

function findAsset(assetsDirectory, pattern) {
  return readdirSync(assetsDirectory).find((name) => pattern.test(name)) ?? null
}

export function buildUpdaterManifest({
  releaseTag,
  repository,
  assetsDirectory,
  outputPath,
  notes = `Automated BizClaw release for ${releaseTag}.`,
  pubDate = new Date().toISOString(),
}) {
  const resolvedAssetsDirectory = resolve(assetsDirectory)
  const macArchive = findAsset(resolvedAssetsDirectory, /\.app\.tar\.gz$/u)
  const macSignature = findAsset(resolvedAssetsDirectory, /\.app\.tar\.gz\.sig$/u)
  const windowsInstaller = findAsset(resolvedAssetsDirectory, /\.msi$/u)
  const windowsSignature = findAsset(resolvedAssetsDirectory, /\.msi\.sig$/u)

  if (!macArchive || !macSignature) {
    throw new Error('Missing macOS updater artifacts (.app.tar.gz and .app.tar.gz.sig)')
  }

  if (!windowsInstaller || !windowsSignature) {
    throw new Error('Missing Windows updater artifacts (.msi and .msi.sig)')
  }

  const manifest = {
    version: releaseTag.replace(/^v/u, ''),
    notes,
    pub_date: pubDate,
    platforms: {
      'darwin-aarch64': {
        signature: readFileSync(resolve(resolvedAssetsDirectory, macSignature), 'utf8').trim(),
        url: releaseDownloadUrl(repository, releaseTag, macArchive),
      },
      'windows-x86_64': {
        signature: readFileSync(resolve(resolvedAssetsDirectory, windowsSignature), 'utf8').trim(),
        url: releaseDownloadUrl(repository, releaseTag, windowsInstaller),
      },
    },
  }

  writeFileSync(resolve(outputPath), JSON.stringify(manifest, null, 2))
  return manifest
}

if (process.argv[1] && fileURLToPath(import.meta.url) === resolve(process.argv[1])) {
  const [, , releaseTag, repository, assetsDirectory = 'release-artifacts', outputPath = 'release-artifacts/latest.json'] = process.argv
  if (!releaseTag || !repository) {
    throw new Error('Usage: node ./scripts/build-updater-manifest.mjs <release-tag> <repository> [assets-directory] [output-path]')
  }

  buildUpdaterManifest({
    releaseTag,
    repository,
    assetsDirectory,
    outputPath,
  })
  console.log(`[INFO] latest.json generated at ${basename(outputPath)}`)
}
