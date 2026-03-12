# Windows Portable Release Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Keep the current Windows MSI release and additionally publish a portable ZIP that users can run without installing BizClaw.

**Architecture:** Reuse the existing Windows release job in GitHub Actions. After `pnpm tauri build --bundles msi` produces `src-tauri/target/release/bizclaw.exe`, create a `zip` artifact from that executable and upload it alongside the MSI so the publish job can attach both assets to the GitHub release.

**Tech Stack:** GitHub Actions, PowerShell, Tauri v2, Vitest

---

### Task 1: Lock the portable artifact requirement in tests

**Files:**
- Modify: `scripts/release-workflow.test.ts`
- Test: `scripts/release-workflow.test.ts`

**Step 1: Write the failing test**

Add a release workflow assertion that the `build-windows` job:
- Creates a portable ZIP from `src-tauri/target/release/bizclaw.exe`
- Uses `Compress-Archive`
- Uploads both `.msi` and `.zip` assets

**Step 2: Run test to verify it fails**

Run: `pnpm exec vitest run scripts/release-workflow.test.ts`
Expected: FAIL because the workflow only uploads the MSI today.

**Step 3: Write minimal implementation**

Update `.github/workflows/release.yml` to:
- Create `src-tauri/target/release/bundle/portable`
- Compress `bizclaw.exe` into a versioned ZIP
- Upload the portable ZIP together with the MSI artifact

**Step 4: Run test to verify it passes**

Run: `pnpm exec vitest run scripts/release-workflow.test.ts`
Expected: PASS

**Step 5: Commit**

```bash
git add .github/workflows/release.yml scripts/release-workflow.test.ts docs/plans/2026-03-12-windows-portable-release.md
git commit -m "feat: publish portable windows release"
```

### Task 2: Verify the repository still builds cleanly

**Files:**
- Modify: none
- Test: `src/App.test.ts`, `src/lib/*.test.ts`, workflow tests, production build

**Step 1: Run the full web test suite**

Run: `pnpm test:web`
Expected: PASS

**Step 2: Run the production build**

Run: `pnpm build`
Expected: PASS

**Step 3: Review the workflow diff**

Check that the Windows release artifact now contains both installer and portable assets without changing the macOS or publish jobs.

**Step 4: Commit**

```bash
git add .github/workflows/release.yml scripts/release-workflow.test.ts docs/plans/2026-03-12-windows-portable-release.md
git commit -m "feat: publish portable windows release"
```
