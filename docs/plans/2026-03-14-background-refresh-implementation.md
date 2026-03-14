# Shell Background Refresh Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the grid-backed shell with a cleaner Google-style canvas while keeping the existing BizClaw layout and interaction model intact.

**Architecture:** This is a CSS-only shell refresh centered in `src/styles.css`. We will remove the global grid overlay, rebalance theme tokens for both light and dark modes, and update the style-contract tests so the new background treatment is protected from regression.

**Tech Stack:** Vue 3, Vite, Vitest, Tauri, plain CSS

---

### Task 1: Lock the new styling contract in tests

**Files:**
- Modify: `src/styles.test.ts`
- Reference: `src/styles.css`

**Step 1: Write the failing test**

Add assertions that:
- `body::before` no longer draws a grid
- `body` still uses layered gradients
- the shell keeps distinct background tokens for light and dark themes

**Step 2: Run test to verify it fails**

Run: `pnpm vitest run src/styles.test.ts`
Expected: FAIL because the current stylesheet still contains the grid overlay contract.

**Step 3: Write minimal implementation**

Adjust the regex expectations so they match the new canvas-based background rather than the old grid-overlay implementation.

**Step 4: Run test to verify it passes**

Run: `pnpm vitest run src/styles.test.ts`
Expected: PASS

### Task 2: Replace the shell background treatment

**Files:**
- Modify: `src/styles.css`
- Reference: `src/components/AppSidebar.vue`
- Reference: `src/components/AppWorkspaceHeader.vue`

**Step 1: Write the failing test**

Use the test from Task 1 as the red case for the visual contract.

**Step 2: Run test to verify it fails**

Run: `pnpm vitest run src/styles.test.ts`
Expected: FAIL before CSS updates.

**Step 3: Write minimal implementation**

- Remove the `body::before` grid overlay.
- Rework light-theme background tokens toward a clean blue-gray canvas.
- Simplify dark-theme background treatment to matching smooth gradients.
- Tune shell accent/glow tokens so sidebar/header/cards feel integrated with the new backdrop.

**Step 4: Run test to verify it passes**

Run: `pnpm vitest run src/styles.test.ts`
Expected: PASS

### Task 3: Run full verification

**Files:**
- Modify: none

**Step 1: Run targeted tests**

Run: `pnpm vitest run src/styles.test.ts`
Expected: PASS

**Step 2: Run the web test suite**

Run: `pnpm test:web`
Expected: PASS

**Step 3: Run production build**

Run: `pnpm build`
Expected: PASS
