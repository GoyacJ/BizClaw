// @vitest-environment node

import { describe, expect, it } from 'vitest'

import { buildRustCommandEnv } from './run-rust-command.mjs'

describe('buildRustCommandEnv', () => {
  it('adds macOS linker overrides when the host platform is darwin', () => {
    expect(buildRustCommandEnv('darwin', {})).toEqual({
      CC: 'clang',
      CXX: 'clang++',
      CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER: 'clang',
      CARGO_TARGET_X86_64_APPLE_DARWIN_LINKER: 'clang',
    })
  })

  it('keeps explicit compiler overrides that are already set', () => {
    expect(buildRustCommandEnv('darwin', {
      CC: 'custom-cc',
      CXX: 'custom-cxx',
      CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER: 'custom-aarch64-linker',
      CARGO_TARGET_X86_64_APPLE_DARWIN_LINKER: 'custom-x64-linker',
    })).toEqual({
      CC: 'custom-cc',
      CXX: 'custom-cxx',
      CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER: 'custom-aarch64-linker',
      CARGO_TARGET_X86_64_APPLE_DARWIN_LINKER: 'custom-x64-linker',
    })
  })

  it('does not inject overrides on non-macOS platforms', () => {
    expect(buildRustCommandEnv('win32', {})).toEqual({})
    expect(buildRustCommandEnv('linux', {})).toEqual({})
  })
})
