import type { CompanyProfile, CompanyProfileDraft } from '@/types'

export function createEmptyCompanyProfileDraft(): CompanyProfileDraft {
  return {
    sshHost: '',
    sshUser: '',
    localPort: '18889',
    remoteBindHost: '127.0.0.1',
    remoteBindPort: '18789',
  }
}

export function draftFromCompanyProfile(
  profile: CompanyProfile,
): CompanyProfileDraft {
  return {
    sshHost: profile.sshHost,
    sshUser: profile.sshUser,
    localPort: String(profile.localPort),
    remoteBindHost: profile.remoteBindHost,
    remoteBindPort: String(profile.remoteBindPort),
  }
}

export function isCompanyProfileDraftComplete(
  draft: CompanyProfileDraft,
): boolean {
  return [
    draft.sshHost,
    draft.sshUser,
    draft.localPort,
    draft.remoteBindHost,
    draft.remoteBindPort,
  ].every((value) => value.trim().length > 0)
}

export function normalizeCompanyProfileDraft(
  draft: CompanyProfileDraft,
): CompanyProfile {
  return {
    sshHost: draft.sshHost.trim(),
    sshUser: draft.sshUser.trim(),
    localPort: parsePort(draft.localPort),
    remoteBindHost: draft.remoteBindHost.trim(),
    remoteBindPort: parsePort(draft.remoteBindPort),
  }
}

function parsePort(value: string): number {
  const parsed = Number.parseInt(value.trim(), 10)
  if (!Number.isInteger(parsed) || parsed < 1 || parsed > 65535) {
    throw new Error('端口必须在 1-65535 之间')
  }

  return parsed
}
