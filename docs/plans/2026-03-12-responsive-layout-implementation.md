# Responsive Layout Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make the BizClaw desktop UI scale cleanly across `1280x800 ~ 1440x900` laptop screens and `1920x1080` desktop screens without changing business logic.

**Architecture:** Keep the current single-page Vue layout and refactor responsiveness primarily in `src/styles.css`. Introduce a small set of desktop breakpoints plus shared sizing tokens, then tighten the shell, cards, forms, logs, and bottom status bar in stages. Use `src/App.test.ts` to protect key status-bar structure and add small regression coverage where layout-sensitive DOM behavior is affected.

**Tech Stack:** Vue 3, Vite, Vitest, CSS

---

### Task 1: Lock the responsive status-bar behavior with tests

**Files:**
- Modify: `src/App.test.ts`
- Test: `src/App.test.ts`

**Step 1: Write the failing test**

Add a focused assertion for the bottom status bar that protects the compact structure we already rely on while responsive refinements are made:
- Status bar still renders as `footer.environment-bar`
- Status items still render as `.environment-pill`
- Detail text still exposes the full value through `title`

**Step 2: Run test to verify it fails**

Run: `pnpm exec vitest run src/App.test.ts`
Expected: FAIL only if the new assertion is not yet satisfied.

**Step 3: Write minimal implementation**

If needed, make the smallest `src/App.vue` adjustment required to keep the DOM stable while the CSS refactor lands.

**Step 4: Run test to verify it passes**

Run: `pnpm exec vitest run src/App.test.ts`
Expected: PASS

**Step 5: Commit**

```bash
git add src/App.test.ts src/App.vue
git commit -m "test: lock responsive status bar structure"
```

### Task 2: Introduce shared desktop sizing tokens

**Files:**
- Modify: `src/styles.css:1-120`
- Test: `src/App.test.ts`

**Step 1: Write the failing test**

No new DOM behavior is required here, so reuse Task 1 regression coverage and treat the first CSS edit as the red/green boundary.

**Step 2: Run test to confirm baseline**

Run: `pnpm exec vitest run src/App.test.ts`
Expected: PASS before CSS refactor starts.

**Step 3: Write minimal implementation**

In `src/styles.css`, add shared size variables for:
- shell padding
- card padding
- compact card padding
- control height
- chip height
- log min-height
- environment pill min-width/min-height

Then swap existing hard-coded values over to those variables where practical.

**Step 4: Run test to verify it still passes**

Run: `pnpm exec vitest run src/App.test.ts`
Expected: PASS

**Step 5: Commit**

```bash
git add src/styles.css src/App.test.ts
git commit -m "refactor: add responsive sizing tokens"
```

### Task 3: Add desktop breakpoints for 1440px and 1280px

**Files:**
- Modify: `src/styles.css:200-720`
- Test: `src/App.test.ts`

**Step 1: Write the failing test**

If you introduce any class or structure change in `src/App.vue`, first extend `src/App.test.ts` to describe the intended DOM contract. Otherwise, proceed with CSS-only red/green based on manual viewport checks.

**Step 2: Run test to confirm baseline**

Run: `pnpm exec vitest run src/App.test.ts`
Expected: PASS

**Step 3: Write minimal implementation**

Add breakpoint layers for:
- `@media (max-width: 1440px)`
- `@media (max-width: 1280px)`

Within those ranges, tune:
- `dashboard-shell`
- `topbar`
- `workflow-strip`
- `workspace-grid`
- `workspace-card`
- `support-grid`
- `form-grid`
- `advanced-grid`
- `console-card--logs`
- `environment-bar`

Keep `1180px` and `760px` behavior, but align them with the new variables.

**Step 4: Run test to verify it passes**

Run: `pnpm exec vitest run src/App.test.ts`
Expected: PASS

**Step 5: Commit**

```bash
git add src/styles.css src/App.test.ts src/App.vue
git commit -m "feat: add desktop responsive breakpoints"
```

### Task 4: Tune density for the primary interaction areas

**Files:**
- Modify: `src/styles.css:240-720`
- Test: `src/App.test.ts`

**Step 1: Write the failing test**

If button or status-bar DOM needs to change, add the test first in `src/App.test.ts`.

**Step 2: Run test to confirm failure**

Run: `pnpm exec vitest run src/App.test.ts`
Expected: FAIL only for the new expectation, if one is added.

**Step 3: Write minimal implementation**

Tighten the dense areas most visible on `1280x800`:
- button rows
- inputs
- support tiles
- toggles
- error banners
- log rows
- compact status pills

The goal is to reclaim height without making controls feel cramped.

**Step 4: Run test to verify it passes**

Run: `pnpm exec vitest run src/App.test.ts`
Expected: PASS

**Step 5: Commit**

```bash
git add src/styles.css src/App.test.ts src/App.vue
git commit -m "feat: tune layout density for smaller desktops"
```

### Task 5: Run full verification and viewport checks

**Files:**
- Modify: none
- Test: `src/App.test.ts`, `src/lib/profile-form.test.ts`, `src/lib/runtime-view.test.ts`, production build

**Step 1: Run focused app test**

Run: `pnpm exec vitest run src/App.test.ts`
Expected: PASS

**Step 2: Run full web test suite**

Run: `pnpm test:web`
Expected: PASS

**Step 3: Run production build**

Run: `pnpm build`
Expected: PASS

**Step 4: Manual verification**

Check the UI at:
- `1280x800`
- `1440x900`
- `1920x1080`

Confirm:
- primary action area remains visible without excessive scrolling
- right-hand runtime card stays readable
- logs do not dominate the page height
- bottom status bar remains compact and aligned

**Step 5: Commit**

```bash
git add src/styles.css src/App.vue src/App.test.ts
git commit -m "feat: polish responsive desktop layout"
```
