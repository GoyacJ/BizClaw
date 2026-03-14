# Shell Background Refresh Design

## Goal

Remove the square grid background from the BizClaw shell and replace it with a calmer, more modern canvas that feels closer to current Google productivity surfaces while staying compatible with the existing rounded card system and light/dark themes.

## Chosen Direction

We will use the approved `B` direction: a light canvas with restrained regional color washes instead of a technical grid. The visual tone should feel clean, desktop-native, and professional rather than decorative or experimental.

## Visual Approach

- Remove the fixed `body::before` grid overlay entirely.
- Keep the page background atmospheric, but simplify it into soft layered gradients.
- Let the shell hierarchy come from card contrast, border definition, and subtle ambient highlights instead of a patterned backdrop.
- Preserve the current shell layout and component structure so the change is aesthetic rather than architectural.

## Theme Strategy

- Light theme becomes a paper-like blue-gray canvas with gentle blue and mint accents near the top edges.
- Dark theme keeps the existing deep palette, but loses any grid-like texture and moves closer to a smooth editorial backdrop.
- Sidebar, workspace header, and cards remain distinct surfaces, but with more restrained glow treatment so the page feels calmer.

## Scope

In scope:

- `src/styles.css` shell and theme tokens
- `src/styles.test.ts` style-contract assertions

Out of scope:

- Vue component structure changes
- Navigation behavior changes
- Copy or interaction changes

## Acceptance Criteria

- No square grid is visible in the shell background.
- The light theme feels cleaner and less “engineering dashboard”.
- Cards still stand out clearly against the background.
- Dark mode remains visually balanced after the background simplification.
