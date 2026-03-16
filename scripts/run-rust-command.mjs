import { pathToFileURL } from 'node:url'

export function buildRustCommandEnv(platform = process.platform, env = process.env) {
  if (platform !== 'darwin') {
    return {}
  }

  return {
    CC: env.CC ?? 'clang',
    CXX: env.CXX ?? 'clang++',
    CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER:
      env.CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER ?? 'clang',
    CARGO_TARGET_X86_64_APPLE_DARWIN_LINKER:
      env.CARGO_TARGET_X86_64_APPLE_DARWIN_LINKER ?? 'clang',
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const [, , command, ...args] = process.argv

  if (!command) {
    throw new Error('Usage: node ./scripts/run-rust-command.mjs <command> [...args]')
  }

  const { spawn } = await import('node:child_process')
  const child = spawn(command, args, {
    stdio: 'inherit',
    shell: false,
    env: {
      ...process.env,
      ...buildRustCommandEnv(),
    },
  })

  child.on('exit', (code, signal) => {
    if (signal) {
      process.kill(process.pid, signal)
      return
    }

    process.exit(code ?? 1)
  })

  child.on('error', (error) => {
    console.error(error)
    process.exit(1)
  })
}
