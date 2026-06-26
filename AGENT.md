# AGENTS.md

Behavioral guidelines to reduce common LLM coding mistakes. Merge with project-specific instructions as needed.

**Tradeoff:** These guidelines bias toward caution over speed. For trivial tasks, use judgment.

## 1. Think Before Coding

**Don't assume. Don't hide confusion. Surface tradeoffs.**

Before implementing:
- State your assumptions explicitly. If uncertain, ask.
- If multiple interpretations exist, present them - don't pick silently.
- If a simpler approach exists, say so. Push back when warranted.
- If something is unclear, stop. Name what's confusing. Ask.

## 2. Simplicity First

**Minimum code that solves the problem. Nothing speculative.**

- No features beyond what was asked.
- No abstractions for single-use code.
- No "flexibility" or "configurability" that wasn't requested.
- No error handling for impossible scenarios.
- If you write 200 lines and it could be 50, rewrite it.

Ask yourself: "Would a senior engineer say this is overcomplicated?" If yes, simplify.

## 3. Surgical Changes

**Touch only what you must. Clean up only your own mess.**

When editing existing code:
- Don't "improve" adjacent code, comments, or formatting.
- Don't refactor things that aren't broken.
- Match existing style, even if you'd do it differently.
- If you notice unrelated dead code, mention it - don't delete it.

When your changes create orphans:
- Remove imports/variables/functions that YOUR changes made unused.
- Don't remove pre-existing dead code unless asked.

The test: Every changed line should trace directly to the user's request.

## 4. Goal-Driven Execution

**Define success criteria. Loop until verified.**

Transform tasks into verifiable goals:
- "Add validation" -> "Write tests for invalid inputs, then make them pass"
- "Fix the bug" -> "Write a test that reproduces it, then make it pass"
- "Refactor X" -> "Ensure tests pass before and after"

For multi-step tasks, state a brief plan:
```
1. [Step] -> verify: [check]
2. [Step] -> verify: [check]
3. [Step] -> verify: [check]
```

Strong success criteria let you loop independently. Weak criteria ("make it work") require constant clarification.

---

**These guidelines are working if:** fewer unnecessary changes in diffs, fewer rewrites due to overcomplication, and clarifying questions come before implementation rather than after mistakes.

# Agent Instructions

These instructions apply to the whole repository. If a deeper `AGENT.md` is added later, follow the most specific file for that subtree while keeping these rules in mind.

## Project Shape

- This is a Tauri 2 desktop app named `radianite`.
- Frontend code lives in `src-ui/` and uses Vite, React 19, TypeScript, Tailwind CSS 4, and shadcn-style UI components.
- Rust/Tauri code lives in `src-rs/`.
- Static web assets live in `public/`.
- Tauri icons, capabilities, config, and generated Rust artifacts live under `src-rs/`.
- CI lives in `.github/workflows/`.
- Local agent/skill configuration lives in `.agents/` and `.codex/`.
- Build output and dependencies such as `dist/`, `node_modules/`, and `src-rs/target/` are generated and should not be edited directly.

## Directory Structure Rules

- Keep React application code under `src-ui/`.
- Put reusable React UI primitives in `src-ui/components/`.
- Put shared frontend helpers in `src-ui/lib/`.
- Keep frontend assets that are imported by React under `src-ui/assets/`; keep browser-served static assets under `public/`.
- Keep Tauri commands, app state, integrations, and platform code under `src-rs/src/`.
- Keep Riot client integration code under `src-rs/src/riot/`.
- Do not create new top-level source directories unless the change genuinely needs a new ownership boundary.
- Prefer the existing `@/` alias for imports from `src-ui`.

## Package And Tooling

- Use Bun for JavaScript package management and scripts. The lockfile is `bun.lock`.
- Do not add `package-lock.json`, `yarn.lock`, or `pnpm-lock.yaml`.
- Use Cargo for Rust dependencies and keep `src-rs/Cargo.lock` committed.
- Use the existing Vite and Tauri configuration unless the task specifically requires changing it.

## Build, Test, And Verify

- Frontend typecheck/build: `bun run build`.
- Tauri CLI entrypoint: `bun run tauri`.
- Rust formatting check from `src-rs/`: `cargo fmt --check`.
- Rust lint from `src-rs/`: `cargo clippy -- -D warnings`.
- Rust tests from `src-rs/`: `cargo test`.
- For changes that touch both frontend and backend behavior, run the relevant frontend and Rust checks when practical.
- If a check cannot be run locally, mention that clearly in the final response.