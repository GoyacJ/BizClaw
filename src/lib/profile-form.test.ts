import { describe, expect, it } from 'vitest'

import {
  createEmptyCompanyProfileDraft,
  draftFromCompanyProfile,
  isCompanyProfileDraftComplete,
  normalizeCompanyProfileDraft,
} from './profile-form'

describe('company profile draft helpers', () => {
  it('starts with loopback-friendly default ports and host', () => {
    expect(createEmptyCompanyProfileDraft()).toEqual({
      sshHost: '',
      sshUser: '',
      localPort: '18889',
      remoteBindHost: '127.0.0.1',
      remoteBindPort: '18789',
    })
  })

  it('treats missing values as incomplete', () => {
    expect(isCompanyProfileDraftComplete(createEmptyCompanyProfileDraft())).toBe(false)
  })

  it('normalizes complete drafts into persisted company profiles', () => {
    expect(normalizeCompanyProfileDraft({
      sshHost: ' gateway.example.com ',
      sshUser: ' bizclaw ',
      localPort: '32001',
      remoteBindHost: ' localhost ',
      remoteBindPort: '32002',
    })).toEqual({
      sshHost: 'gateway.example.com',
      sshUser: 'bizclaw',
      localPort: 32001,
      remoteBindHost: 'localhost',
      remoteBindPort: 32002,
    })
  })

  it('hydrates saved company profiles back into string drafts', () => {
    expect(draftFromCompanyProfile({
      sshHost: 'gateway.example.com',
      sshUser: 'bizclaw',
      localPort: 32001,
      remoteBindHost: 'localhost',
      remoteBindPort: 32002,
    })).toEqual({
      sshHost: 'gateway.example.com',
      sshUser: 'bizclaw',
      localPort: '32001',
      remoteBindHost: 'localhost',
      remoteBindPort: '32002',
    })
  })
})
