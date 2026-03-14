// @vitest-environment node

import { describe, expect, it } from 'vitest'

import { normalizeTauriSigningKey } from './prepare-tauri-signing-key.mjs'

describe('normalizeTauriSigningKey', () => {
  it('keeps a canonical base64-encoded signing key unchanged', () => {
    const key = 'dW50cnVzdGVkIGNvbW1lbnQ6IHJzaWduIGVuY3J5cHRlZCBzZWNyZXQga2V5ClJXUlRZMEl5dWpodzVBQTdVajVmRTJiUDZJK2hNMkZyU0FwczU0YzlGcjBycTNMd2hxSUFBQkFBQUFBQUFBQUFBQUlBQUFBQWU2c0RIMlVIOGVlM1BkTnF1cjlDcHNTZ0dhcEFoY2hLTnE5SldidUc3SnpXdUtxVU9qdEVTMStacURKL3lZSjhFbkwrVThOVkx2Y1V0SUhxbzhuOGp1aklzdXR2QzNoS3FEbXduWm5rRHBKeUxYMDVnRGZrOWRVRG1aRjN3aW9RdkZ4cFdNRjZWTWs9Cg=='

    expect(normalizeTauriSigningKey(key)).toBe(key)
  })

  it('decodes percent-escaped newlines that were accidentally stored in GitHub secrets', () => {
    const keyWithEscapedNewline = 'dW50cnVzdGVkIGNvbW1lbnQ6IHJzaWduIGVuY3J5cHRlZCBzZWNyZXQga2V5ClJXUlRZMEl5dWpodzVBQTdVajVmRTJiUDZJK2hNMkZyU0FwczU0YzlGcjBycTNMd2hxSUFBQkFBQUFBQUFBQUFBQUlBQUFBQWU2c0RIMlVIOGVlM1BkTnF1cjlDcHNTZ0dhcEFoY2hLTnE5SldidUc3SnpXdUtxVU9qdEVTMStacURKL3lZSjhFbkwrVThOVkx2Y1V0SUhxbzhuOGp1aklzdXR2QzNoS3FEbXduWm5rRHBKeUxYMDVnRGZrOWRVRG1aRjN3aW9RdkZ4cFdNRjZWTWs9Cg==%0A'

    expect(normalizeTauriSigningKey(keyWithEscapedNewline)).toBe(
      'dW50cnVzdGVkIGNvbW1lbnQ6IHJzaWduIGVuY3J5cHRlZCBzZWNyZXQga2V5ClJXUlRZMEl5dWpodzVBQTdVajVmRTJiUDZJK2hNMkZyU0FwczU0YzlGcjBycTNMd2hxSUFBQkFBQUFBQUFBQUFBQUlBQUFBQWU2c0RIMlVIOGVlM1BkTnF1cjlDcHNTZ0dhcEFoY2hLTnE5SldidUc3SnpXdUtxVU9qdEVTMStacURKL3lZSjhFbkwrVThOVkx2Y1V0SUhxbzhuOGp1aklzdXR2QzNoS3FEbXduWm5rRHBKeUxYMDVnRGZrOWRVRG1aRjN3aW9RdkZ4cFdNRjZWTWs9Cg==',
    )
  })

  it('converts the raw minisign key file content into the canonical single-line format', () => {
    const rawKey = 'untrusted comment: rsign encrypted secret key\nRWRTY0Iyujhw5AA7Uj5fE2bP6I+hM2FrSAps54c9Fr0rq3LwhqIAABAAAAAAAAAAAAIAAAAAe6sDH2UH8ee3PdNqur9CpsSgGapAhchKNq9JWbuG7JzWuKqUOjtES1+ZqDJ/yYJ8EnL+U8NVLvcUtIHqo8n8jujIsutvC3hKqDmwnZnkDpJyLX05gDfk9dUDmZF3wioQvFxpWMF6VMk=\n'

    expect(normalizeTauriSigningKey(rawKey)).toBe(
      'dW50cnVzdGVkIGNvbW1lbnQ6IHJzaWduIGVuY3J5cHRlZCBzZWNyZXQga2V5ClJXUlRZMEl5dWpodzVBQTdVajVmRTJiUDZJK2hNMkZyU0FwczU0YzlGcjBycTNMd2hxSUFBQkFBQUFBQUFBQUFBQUlBQUFBQWU2c0RIMlVIOGVlM1BkTnF1cjlDcHNTZ0dhcEFoY2hLTnE5SldidUc3SnpXdUtxVU9qdEVTMStacURKL3lZSjhFbkwrVThOVkx2Y1V0SUhxbzhuOGp1aklzdXR2QzNoS3FEbXduWm5rRHBKeUxYMDVnRGZrOWRVRG1aRjN3aW9RdkZ4cFdNRjZWTWs9Cg==',
    )
  })

  it('fails fast with a helpful error when the secret still contains invalid characters', () => {
    expect(() => normalizeTauriSigningKey('abc%zz')).toThrowError(/TAURI_SIGNING_PRIVATE_KEY/u)
  })

  it('extracts the canonical key when the secret was pasted as an env assignment', () => {
    const pasted = 'TAURI_SIGNING_PRIVATE_KEY=dW50cnVzdGVkIGNvbW1lbnQ6IHJzaWduIGVuY3J5cHRlZCBzZWNyZXQga2V5ClJXUlRZMEl5dWpodzVBQTdVajVmRTJiUDZJK2hNMkZyU0FwczU0YzlGcjBycTNMd2hxSUFBQkFBQUFBQUFBQUFBQUlBQUFBQWU2c0RIMlVIOGVlM1BkTnF1cjlDcHNTZ0dhcEFoY2hLTnE5SldidUc3SnpXdUtxVU9qdEVTMStacURKL3lZSjhFbkwrVThOVkx2Y1V0SUhxbzhuOGp1aklzdXR2QzNoS3FEbXduWm5rRHBKeUxYMDVnRGZrOWRVRG1aRjN3aW9RdkZ4cFdNRjZWTWs9Cg=='

    expect(normalizeTauriSigningKey(pasted)).toBe(
      'dW50cnVzdGVkIGNvbW1lbnQ6IHJzaWduIGVuY3J5cHRlZCBzZWNyZXQga2V5ClJXUlRZMEl5dWpodzVBQTdVajVmRTJiUDZJK2hNMkZyU0FwczU0YzlGcjBycTNMd2hxSUFBQkFBQUFBQUFBQUFBQUlBQUFBQWU2c0RIMlVIOGVlM1BkTnF1cjlDcHNTZ0dhcEFoY2hLTnE5SldidUc3SnpXdUtxVU9qdEVTMStacURKL3lZSjhFbkwrVThOVkx2Y1V0SUhxbzhuOGp1aklzdXR2QzNoS3FEbXduWm5rRHBKeUxYMDVnRGZrOWRVRG1aRjN3aW9RdkZ4cFdNRjZWTWs9Cg==',
    )
  })
})
