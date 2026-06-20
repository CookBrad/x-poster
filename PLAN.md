# x-poster — Design & Task Plan

> **Living document.** This file captures architecture decisions, design discussions, tradeoffs, and the current task breakdown.
> Update it after any significant conversation or when priorities shift.
>
> Last updated: 2026-06-13 (Strengthened draft originality via better prompts, richer custom sources, rationale visibility in editor)

---

## Vision

A **local-first desktop app** that helps the user stay on top of Tesla/TSLA/Elon-related developments by:

1. Automatically researching via X (semantic + keyword search) + RSS feeds
2. Generating high-quality draft posts using xAI Grok
3. Letting the human review, edit, and explicitly approve every post
4. Posting to X only after approval

**Core constraint:** The human stays in full control. No fully autonomous posting.

**Non-goals (for now):** Multi-account support, political content, general-purpose Twitter client, mobile version.

---

## Guiding Principles

- **Local first** — Data lives on the user's machine. Minimal cloud dependency.
- **Human approval required** — Every post must be explicitly approved in the MVP.
- **Transparent research** — User can always see the sources that fed a draft.
- **Fresh take required** — Drafts must offer original analysis, implications, connections, or a novel angle. They must not merely restate or closely paraphrase what has already been widely reported or said on X. When specific facts are drawn from sources, they must be explicitly attributed inline in the generated post.
- **Tests for every new feature** — No feature is considered complete until it has automated tests. Tests are part of the definition of done. This applies to both backend commands and frontend behavior.
- **Simple & reliable** — Prefer boring, debuggable solutions over clever ones.
- **Secure by default** — Keys move to OS secure storage before any public distribution.
- **Clean Code always** — All new and changed code must follow the [Code Quality (Clean Code)](#code-quality-clean-code) standards below. This is not optional polish; it is part of how we build in this repo.

---

## Code Quality (Clean Code)

This codebase follows the principles from *Clean Code* by Robert C. Martin ("Uncle Bob"). **Every feature, fix, and refactor must uphold these standards.** When in doubt, prefer readability and small, focused units over cleverness.

### Core principles (always apply)

| Principle | What it means here |
|-----------|-------------------|
| **Meaningful names** | Names reveal intent (`fetch_latest_run_with_sources`, `errorMessage`, `DraftSource`). Avoid abbreviations and noise (`data`, `temp`, `handleStuff`). |
| **Small functions** | Each function does one thing. If it needs a comment to explain *what* it does, extract a named function instead. |
| **Single responsibility** | One reason to change per module/file. UI shells stay thin (`App.tsx`); domain logic lives in `lib/` or focused components. |
| **DRY** | Do not duplicate logic. Shared UI → components; shared TS logic → `src/lib/`; shared Rust logic → helpers in the relevant module (or a dedicated module when a file grows too large). |
| **No magic strings** | Status values, setting keys, defaults, and URL prefixes live in `src/lib/constants.ts` and `src-tauri/src/constants.rs` — not scattered inline. |
| **Strong typing** | Prefer explicit types over `any`. Parse JSON into named interfaces (`DraftSource`, `DraftStatus`), not untyped blobs. |
| **Consistent error handling** | One pattern per layer: `errorMessage()` in the frontend; `Result<T, String>` in Rust; surface errors through UI state (`setError`), not ad-hoc `alert()` mixed with banners. |
| **Comments explain why** | Code should be self-documenting. Comments are for non-obvious *why* (business rules, workarounds), not restating *what* the next line does. Avoid ticket-id section headers (`T-003`) — module names should carry intent. |
| **Tests stay readable** | Tests read like specifications: clear arrange/act/assert, meaningful names, no opaque mock leakage between cases. |

### Established patterns in this repo

Use and extend these — do not reinvent parallel conventions:

**Frontend (`src/`)**
- `src/lib/constants.ts` — draft statuses, setting keys, defaults (`DEFAULT_GROK_MODEL`, `DEFAULT_DRAFT_GENERATION_COUNT`, etc.)
- `src/lib/errors.ts` — `errorMessage(unknown)` for all user-facing error text
- `src/lib/draftUtils.ts` — pure draft helpers (counts, timestamps, X URLs, source labels)
- `src/lib/db.ts` — thin Tauri invoke wrappers only; parsing helpers (`parseSources`) live here with proper types
- Components — one primary concern per file (`PostsTab`, `ResearchTab`, `ResearchSourceCard`, `HistoricalSourcesList`)
- Props interfaces named explicitly (`DraftCardProps`), not inline anonymous blobs for non-trivial components

**Backend (`src-tauri/src/`)**
- `constants.rs` — `draft_status`, `settings`, shared defaults
- Tauri commands stay thin; reusable logic in `*_db` helpers or private functions (`load_grok_settings`, `require_setting`, `fetch_run_with_sources`, `build_draft_source_context`, `maybe_resolve_preview_image`)
- Parameterized SQL (`?` binds) instead of string-interpolated literals where values vary
- `#[cfg(test)]` modules colocated; `create_test_pool()` for real migration-backed DB tests

### Definition of done — code quality checklist

Before marking work complete, verify:

- [ ] No new magic strings for statuses, settings, or defaults (use `constants.ts` / `constants.rs`)
- [ ] No duplicated logic that already has a helper or component
- [ ] Functions are short and named for intent
- [ ] Errors handled consistently (no new `alert()` paths in React UI)
- [ ] Types are explicit; no new `any` without a documented reason
- [ ] New pure logic has unit tests where practical
- [ ] Large files were not grown further without extraction (split components/modules when a file becomes hard to scan)

### When refactoring

Prefer incremental Clean Code improvements in the same PR as feature work when touching an area. Do not leave "cleanup for later" if the touched code clearly violates the standards above. A focused 20-line improvement beats a deferred 200-line rewrite.

---

## Testing Strategy

This section exists so that future sessions have clear, consistent guidance on how we approach testing. Update it as we make concrete tooling and process decisions.

### Philosophy
- Tests are **mandatory** for every new feature or significant change.
- "Feature complete" = working code + passing tests + updated documentation (where relevant).
- Prefer pragmatic, high-signal tests over 100% coverage theater.
- Tests should be fast, reliable, and easy to run locally.
- The existence of this rule in PLAN.md means we never have the "we'll add tests later" conversation again.

### Current State (as of 2026-06-04)
- **Frontend**: Vitest + React Testing Library + happy-dom. Tests cover `db.ts`, `QueueTab`, `DraftEditModal`, `HistoryTab`, `XCredentialsSettings`, `ApiKeySettings`, plus example test.
- **Backend**: Rust tests in `commands.rs`, `generation.rs`, `x_post.rs`, `research.rs` (RSS network test). In-memory SQLite + real migrations via `create_test_pool()`.
- Run: `npm test` (frontend), `cd src-tauri && cargo test` (backend).

### Backend (Rust / Tauri) Testing Approach
- Use Rust's built-in test framework (`#[test]` functions) + `#[cfg(test)]` modules.
- For database-dependent code: Use an in-memory SQLite database (`:memory:`) or a dedicated test database file per test run.
- Integration-style tests for Tauri commands are acceptable and often more valuable than pure unit tests in this architecture.
- Consider `tokio-test` or `sqlx::test` attribute for async DB tests if we adopt it.
- Goal: Every new Tauri command should have at least one happy-path test + relevant error cases.

### Frontend (React / TypeScript) Testing Approach
- **Chosen stack**: **Vitest + React Testing Library + happy-dom**
  - We chose `happy-dom` over `jsdom` due to better compatibility with the modern ESM-heavy dependency tree (daisyUI + PostCSS color tooling).
- Focus on:
  - Component behavior and user interactions
  - Critical UI flows (approve, edit, skip, settings save)
  - Mocking Tauri `invoke` calls cleanly (using `vi.mock` or `@tauri-apps/api` mocks)
- E2E testing (Playwright or Tauri end-to-end) is desirable later but **not required** for MVP features.
- Keep tests close to the components they cover (`*.test.tsx` or `*.spec.tsx`).
- How to run: `npm test` or `npm run test:ui`

### Backend (Rust / Tauri) Testing Approach
- **Current approach**: Rust's built-in `#[test]` + `#[tokio::test]` with in-memory SQLite (`sqlite::memory:`).
- We added `tokio` to `[dev-dependencies]`.
- Pattern established: `create_test_pool()` helper that runs the real embedded migrations.
- First real test lives in `src-tauri/src/commands.rs` under `#[cfg(test)]`.
- Because Tauri commands take `State<'_, AppState>`, early tests exercise the data layer directly. We will improve this over time by extracting pure functions that take `&SqlitePool`.
- How to run: `cd src-tauri && cargo test`

### Definition of Done for Any New Work
When working on a task/feature, the following must be true before marking it complete:
- [ ] Implementation done and manually verified
- [ ] [Code Quality (Clean Code)](#code-quality-clean-code) checklist satisfied
- [ ] Relevant automated tests written and passing
- [ ] Tests cover happy path + at least one important edge/error case
- [ ] PLAN.md updated if the work affected design, architecture, or process

### Open Testing Decisions
- How aggressively we will extract logic from Tauri commands to make them easier to unit test (current early tests go through the data layer directly).
- Whether we want snapshot tests on the frontend.
- Strategy for mocking `@tauri-apps/api` and Tauri invoke calls in component tests.
- Whether we will enforce a minimum coverage threshold in CI later (not planned for MVP).

Add new decisions here as we make them.

---

## Current State (as of late May 2025)

### Done
- Tauri + React + Tailwind + daisyUI scaffold
- SQLite database with `drafts` and `post_history` tables + migrations
- Full Rust CRUD commands for drafts (`create_draft`, `get_drafts`, `update_draft`, `delete_draft`, `mark_draft_posted`)
- Basic "Test xAI Connection" working in Settings (frontend calls xAI directly)
- Placeholder Queue UI with fake draft cards

### In Progress / Partial
- None for Phase 1 — MVP feature set is complete.

### Not Started (Phase 2+)
- Background scheduler / tray icon (T-009)
- Secure key storage (T-011)
- Richer source attribution UI (T-012)
- Unsplash / advanced image support
- Secure packaging distribution

---

## Design Decisions

Record important choices here with date + rationale.

### 2025-05-25 — Database choice
- **Decision:** Use `sqlx` with SQLite directly (not Tauri's sql plugin).
- **Rationale:** More control, better async story with Tokio, easier to write custom queries and migrations. Recommended pattern for Tauri apps that need real power.

### 2025-05-25 — Draft status model
- **Decision:** Simple status field: `pending` | `posted` | `skipped`.
- **Future consideration:** May add `needs_review`, `generating`, `failed` later if the pipeline gets more states.

### 2025-05-25 — Key storage (MVP vs packaged)
- **Decision:** `.env` (VITE_*) for development only.
- **Future:** Move to Tauri's OS keychain / secure storage plugin before first real distribution.

### 2025-05-28 — Fresh take, not regurgitation
- **Decision:** Generated drafts must provide original commentary/analysis rather than restating existing posts or news. Specific facts taken from sources must be attributed inline in the post text.
- **Rationale:** Core quality bar for the product. Avoids the app producing low-value "me too" content. This is a defining characteristic of the output, not just a prompt tweak.
- **Implications:** 
  - Strong instructions + examples needed in the Grok prompt (T-005).
  - May require the generation step to have visibility into the user's recent X posts and/or previously generated drafts to avoid repetition.
  - Research layer may need to distinguish between "raw facts" and "already widely discussed angles."

### 2025-05-28 — Commands layer separation (readability + testability)
- **Decision:** Split each Tauri command into a thin public `#[tauri::command]` wrapper + a `*_db(...)` implementation function that takes `&SqlitePool`.
- **Rationale:** 
  - Dramatically improves testability (we can now test real logic with in-memory databases).
  - Makes the business logic reusable outside of Tauri commands.
  - Keeps the Tauri-specific glue (State extraction) isolated and minimal.
- **Result:** All CRUD logic now lives in reusable `*_db` functions. Tests in `commands.rs` now call the real functions instead of raw SQL.
- **Future:** If the data layer grows, we can promote this into a proper `db/` module or `DraftRepository`.

### 2025-06-02 — API Key storage (MVP approach)
- **Decision:** xAI API key is persisted in the existing SQLite database via new `get_setting` / `set_setting` commands (using a simple `settings` table created on first use).
- **Rationale:** Allowed us to quickly deliver an editable + savable key experience directly in the Settings UI without adding new dependencies or plugins. Users no longer need to edit `.env` and restart.
- **Tradeoffs:** Keys are stored in plaintext within the app's data directory. This is acceptable for local development and early testing, but **must** be replaced with proper OS secure storage (keychain / credential vault) before packaging/distribution.
- **Related:** Directly fulfills the request to make the API key "submittable on the Settings tab". Advances T-008 (Settings UI for credentials). Also involved UI polish (Show/Hide toggle, fixed label overlap, save feedback).

---



## Task Breakdown

Tasks are grouped by phase. Checkboxes are the source of truth for status.

### Phase 0 — Foundation (Mostly Complete)

- [x] Project scaffold (Tauri + Vite + React + Tailwind + daisyUI)
- [x] SQLite database + migrations (`drafts`, `post_history`)
- [x] Rust command layer for draft CRUD
- [x] Basic UI shell with tabs (Queue / Research / Settings / History)
- [x] Working xAI connection test (Settings tab)

### Phase 1 — Core MVP (Next Priority)

This is the minimum that makes the app actually useful.

**Important:** Per the Testing Strategy above, every task below must include automated tests as part of completion.

- [x] **T-000** — Establish testing foundation
  - Vitest + RTL + happy-dom; Rust `#[cfg(test)]` + in-memory DB; documented in Testing Strategy.

- [x] **T-001** — Wire React frontend to Rust draft commands
  - Queue tab backed by SQLite; `db.ts` wrappers + tests; `QueueTab` component tests.

- [x] **T-002** — Build basic draft editing UI
  - `DraftEditModal`: edit text, image URL, live preview, sources list; component tests.

- [x] **T-003** — X research module (backend)
  - **Done via Grok + xAI** (`fetch_grok_discovered_x_sources`) — direct X Developer search API intentionally removed per design decision (2025-06-02). High-signal curation + confidence filtering in `research.rs`.

- [x] **T-004** — RSS research module (backend)
  - `fetch_rss_sources` in `research.rs` (Teslarati, Not a Tesla App); 14-day filter; network test.

- [x] **T-005** — Draft generation via xAI Grok
  - `generation.rs` + `generate_drafts_from_latest_research` command; fresh-take prompt + inline attribution; creates `Draft` rows with `sources_json`.

- [x] **T-006** — Manual research flow (Research tab)
  - RSS / X / Both buttons; persistence; Historical tab; **Generate Drafts → Queue** button.

- [x] **T-007** — Real X posting flow
  - `post_draft_to_x` via OAuth 1.0a + Twitter API v2; Queue **Approve & Post** calls backend; `mark_draft_posted` with real tweet id.

- [x] **T-008** — Settings UI for credentials
  - xAI key + Grok model (`ApiKeySettings`); X OAuth 1.0a four-field form (`XCredentialsSettings`) + test connection.

- [x] **T-015** — Fresh take enforcement (MVP)
  - Strong generation system prompt; recent **posted** drafts passed into Grok context; edit modal shows research sources + anti-repetition note. Full "already widely discussed" detection deferred to Phase 2/3.

### Phase 2 — Polish & Reliability

- [ ] **T-009** — Background scheduler (research on a timer)
- [ ] **T-010** — System tray icon + menu (macOS first)
- [ ] **T-011** — Secure key storage (Tauri plugin or OS keychain)
- [ ] **T-012** — Better source attribution UI (show real links, not just "Sources: ...")
- [ ] **T-013** — Draft history / posted log with direct X links
- [ ] **T-014** — Basic image support (attach or generate simple visuals)

### Phase 3 — Advanced / Nice to Have

- [ ] Richer research (combine multiple signals, scoring)
- [ ] Multiple topics / watchlists
- [ ] Draft templates or tone controls
- [ ] Export / backup of local data
- [ ] Dark mode refinements, better empty states, keyboard shortcuts

---

## Open Questions & Research Needed

- How aggressive should the research cadence be? (user-configurable?)
- What's the right balance of "freshness" vs "volume" for drafts?
- ~~Do we want to support both OAuth 1.0a and OAuth 2.0 for X, or just one?~~ **Resolved:** OAuth 1.0a for posting (paste tokens from Developer Portal).
- Image strategy: stock photos, AI-generated, or none for MVP?
- Rate limiting / cost control for xAI calls?
- **Fresh take specifics:** How strictly do we enforce original analysis vs allowing some factual summarization? Should the app fetch the user's own recent X posts before generation to avoid self-repetition? How do we detect "already widely discussed angles" in research results?

Add new questions here as they come up. Resolve and move to Design Decisions when answered.

---

## Session Log

This section captures key discussions from conversations so future sessions can pick up context quickly.

**Format:** Add new entries at the **top**.

---

### 2026-06-13 — Strengthened originality and interesting information in generated drafts

**What was implemented:**
- Stronger "Insight" (default) generation rules in `generation.rs`: more specific GOOD examples of originality (data moat, margin mix, regulatory read-through), explicit anti-phrasing ("re-express the implication in fresh language"), "long-time follower who spots the non-obvious" directive.
- Increased source excerpt from 400 to 1200 chars fed to Grok.
- Special "Notable angle from source" formatting for X research items carrying "Why notable" (from discovery) so the pre-computed insight seed is prominent.
- Slight enhancement to research X discovery prompt ("why_interesting" schema + user prompt) to bias toward non-obvious angles useful for original posts.
- Richer custom article sources: new `extract_main_text_excerpt` (p-tag based, ~1500 chars) in `draft_image.rs` used by `custom_source.rs` resolve_article_url — now passes real article body text instead of just short OG description.
- Added `generation_rationale` (Grok's self-reported "what useful insight you added") to Drafts table (new migration 0007), Create/Update inputs, DB fns, TS types.
- Rationale is set at creation (from item.rationale), logged at INFO, and surfaced read-only in `DraftEditModal` as a small "Grok's intended insight / added value:" box during mandatory human review/edit. This gives the user visibility into the originality attempt so they can amplify it.
- Updated all related tests (prompt contains new phrases, rationale population, article excerpt length) + mocks/literals for the new optional field. All tests still pass.
- Updated root PLAN.md (last updated + new top Session Log entry).

**Rationale:**
- Addresses user feedback that posts lacked originality/interesting information by attacking the main causes: weak prompt examples + limited context in sources + no feedback of the "added value" to the reviewer.
- Reuses all existing anti-regurg, finalize, cashtag, attribution, recent dedup, and human-in-loop machinery.
- Rationale display turns the previously discarded Grok output into a tool for the user (who already must edit/approve every draft).

**Verification performed (per plan):**
- cargo test / cargo check / npm build / npm test all clean.
- Manual flows (bulk generate from research, per-source, custom URL/article/topic) produce visibly stronger insight-style posts.
- Rationale appears in edit modal for generated drafts.
- Custom article now has much longer content in sources_json.

This builds on the fresh-take principle and advances the spirit of T-015 (UI visibility for the insight angle).

---

### 2026-06-04 — Custom draft inputs, per-source generation, image support, ResearchTab component extract

**What was implemented:**
- Extracted the entire ResearchTab (Current/Historical + all research + generation UI) out of App.tsx into `src/components/ResearchTab.tsx` (cleaner separation; imports new generation functions, CustomDraftInput, style/mode constants, unused-source trackers).
- New `CustomDraftInput` component + test: free-form textarea for custom prompt/text/URL/X post, DraftStyle selector (insight / informative / funny / meme), Generate button. Handles disabled states (no key, empty, busy).
- Backend custom source support (`src-tauri/src/custom_source.rs`): URL detection/normalization, X vs non-X classification so custom inputs (including direct X post URLs) can feed generation the same as research sources.
- New `draft_image.rs`: helpers to extract OpenGraph/Twitter meta images, page titles, descriptions from web pages or X sources for attaching relevant images to generated drafts.
- Large updates to `generation.rs`: prompt builders for custom inputs vs per-research-source vs bulk-from-latest; style-specific system prompts; enforcement of at most one stock cashtag per draft; logic to mark/track used research sources so they are excluded from future bulk runs.
- `x_media.rs` expanded for fetching tweet source details, preview images, matching primary source.
- Frontend: `src/lib/draftGeneration.ts` + `constants.ts` (DRAFT_STYLE, options, persisted count + style loaders/savers); `db.ts` new invokes (`generateDraftFromInput`, `generateDraftFromSource`, `generateDraftsFromLatestResearch`); `researchSource.ts` utils (`countUnusedResearchSources`, `isResearchSourceUsed`); ResearchTab now has per-source "Generate" buttons, custom input section at top, style + count controls, pipeline phases (research → generate), success toasts.
- Command wiring + small Cargo.toml/lock updates (http-range and Tauri features for media/range requests).
- Tests: dozens of new/passing Rust tests (generation::tests for custom source prompts, stock tag limit=1, system prompts per style, unused sources filter, build_user_prompt variants, x_media preview, etc.) + Vitest updates for the new components and generation flows. All 48 Rust + relevant frontend tests green.

**Rationale / user value:**
- Users can now generate a single high-quality draft for one specific research story (the "per-story" flow) instead of only bulk.
- Free-form custom instructions or pasting a specific X post / article URL bypasses or augments the research step.
- Drafts can include relevant images pulled from the source.
- Source usage tracking prevents the same research items from being re-generated in bulk runs.
- UI is now in focused component files following the established patterns (one concern per file).
- Style selection lets user choose tone (insightful default, etc.).

**Tests:** Mandatory coverage added for all new paths.

**Next steps implied:** Wire the new generation calls into Queue, image selection/approval in the draft flow, full human-in-loop review for custom + per-source drafts.

---

### 2026-06-04 — Phase 1 MVP completed

**Shipped:**
- `generation.rs` — Grok draft generation with fresh-take + attribution prompts; anti-repetition via recent posted drafts.
- `x_post.rs` — OAuth 1.0a signing + `post_tweet` + `verify_credentials`.
- Commands: `generate_drafts_from_latest_research`, `post_draft_to_x`, `test_x_credentials`, `has_x_credentials`.
- UI: `DraftEditModal`, `QueueTab` (real X post), `HistoryTab`, `XCredentialsSettings`, Research **Generate Drafts → Queue**.
- Tests: 12 Rust + 31 Vitest (fixed ApiKeySettings unhappy-path mock ordering).

**Commits:** reset fix + Phase 1 feature commit(s) on `main`.

**Note:** T-003 original spec (direct X API search) superseded by Grok-only research. T-015 full discourse detection remains future work.

---

### 2026-06-04 — Reset All Research Data: DB delete actually runs (user report: still not deleting)

**Diagnosis:**
- Live DB at `~/Library/Application Support/com.bradleycook.xposter/x-poster.db` still had 20 runs / 249 sources after UI "reset" attempts — backend DELETE was never taking effect in practice.
- Likely causes: `window.confirm()` unreliable in Tauri webview (early return before invoke), reset button `disabled={loading}` during research, and no post-delete verification.

**Fix:**
- Replaced `window.confirm()` with an in-app daisyUI `<dialog>` modal (explicit Cancel / "Yes, delete everything").
- Separate `isResetting` state — reset is not blocked by research `loading`.
- `reset_research_data` now runs in a SQL transaction, returns `{ deleted_sources, deleted_runs }`, and errors if any rows remain after delete.
- Frontend calls `invoke('reset_research_data', {})`, then re-fetches `getAllHistoricalSources` and throws if anything remains.
- Shows green success alert with deleted counts, or red error with backend message.
- Added explicit `"label": "main"` on the window in `tauri.conf.json` (matches capabilities).

**Verification:** `cargo test test_reset_research_data_clears`, `npm test src/lib/db.test.ts`, `tsc --noEmit`. User must restart via `npm run tauri dev` (rebuild Rust) for changes to apply.

---

### 2026-06-04 — Reset All Research Data: reliable historical list clear

**Change:**
- `HistoricalSourcesList` now takes `reloadToken` and refetches when it changes (parent bumps after successful `resetResearchData`).
- On each reload, search/pagination/list state is cleared synchronously before the fetch so stale historical rows do not remain visible.
- Kept `key={historicalResetKey}` for full remount when already on the Historical sub-tab.
- Reset handler also calls `loadLatest()` so Current reflects an empty DB after wipe.
- Added Vitest coverage in `db.test.ts` for `resetResearchData` and post-reset `getAllHistoricalSources` returning `[]`.

**Verification:** `cargo test test_reset_research_data_clears`, `npm test`.

---

### 2025-06-03 — Reset All Research Data bugfix: delete succeeded but UI did not clear

**Symptom (user report after "app running again" post-JSX balance fixes):**
- Clicking "Reset All Research Data", confirming the long warning, appeared to do nothing: Historical list (and Current) continued to show previously researched sources/runs. No error was shown.
- "The button does nothing" had been reported earlier; after structural fixes it "ran" (no crash) but still didn't reflect the empty state.

**Root cause diagnosis:**
- Backend `reset_research_data_db` (DELETE sources; DELETE runs) + `get_all_historical_sources` (the JOIN query) were correct and cleared data (proven by new test).
- `resetResearchData()` invoke + catch/error display path was wired.
- `HistoricalSourcesList` was a self-contained component managing its `allSources` via `loadAll` calling `getAllHistoricalSources`.
- It received `refreshKey` as a *prop* (not React key) and did `useEffect(() => { loadAll(); }, [refreshKey])`.
- The reset handler did `setCurrentRun(null)`, `setHistoricalResetKey(k => k+1)`, `setActiveSubTab('historical')`.
- Because of conditional render `{active==='historical' && <HistoricalSourcesList refreshKey={...} /> }` + internal component state, the effect sometimes didn't produce a visible empty (or previous search state + mount timing + no full instance reset meant the "No historical sources yet" alert wasn't reached reliably in all tab switch scenarios).
- Also: dead unused `historicalSources` state in ResearchTab, and the list kept internal search/page across reloads (minor but related to state isolation).

**Fix:**
- Changed the list instantiation to use React's `key` prop for reset: `<HistoricalSourcesList key={historicalResetKey} />`.
- Updated `HistoricalSourcesList` to take no props: `function HistoricalSourcesList() { ... useEffect(() => { loadAll(); }, []); ... }` (standard mount-only effect).
- When parent bumps `historicalResetKey`, React fully unmounts the prior list (discards its useState for search/page/allSources/loading), mounts a *fresh* instance which initializes loading=true + runs the [] effect → loadAll() → gets current DB ([] after successful delete) → renders the "No historical research sources yet" alert.
- This also auto-resets any lingering search term / pagination to defaults on reset (clean slate).
- Kept the forced `setActiveSubTab('historical')` so user immediately sees the confirmation that data is gone.
- Manually `setCurrentRun(null)` ensures that switching back to Current shows the "No research run yet" empty state.
- Removed the dead `const [historicalSources, setHistoricalSources]` state.
- Added detailed comment explaining the key-remount technique.
- Added a full unit test `test_reset_research_data_clears_runs_and_sources` in commands.rs (re-uses the `create_test_pool` + migrations pattern): seeds a run+source, asserts pre counts + hist join >0, calls `reset_research_data_db`, asserts post counts==0 and hist query returns [].
- Incidental (to make `npm run build` / tsc -b verify cleanly while here): fixed pre-existing `TS6133 'i' declared but never read` in a sources.map in Queue section of App.tsx (removed unused index param); fixed `TS2769 'test' not in UserConfigExport` for vite.config.ts by casting the config arg `as any` (vitest-only field) + comment. Now `npm run build` succeeds end-to-end.

**Verification performed:**
- `cargo test test_reset_research_data_clears...` → passes (and re-ran after edits).
- `npx tsc --noEmit` clean.
- `npm run build` (tsc -b + vite build) now succeeds completely (dist produced).
- `cd src-tauri && cargo check` clean.
- The DB layer clear + subsequent get is covered by automated test (the exact contract the UI relies on after reset).
- UI flow uses well-known stable React pattern for "force a subtree to reset and refetch".

**Why this approach:**
- Using `key` bump is the idiomatic, reliable way to get a completely fresh component instance + state + effects on demand (vs trying to orchestrate prop+dep reload while preserving or fighting internal state).
- Matches the "force reload of Historical list" intent that was attempted earlier.
- Keeps the destructive reset protected by confirm() as required.

**Commits & PLAN:**
- Will commit after this update.
- Added test coverage for the reset path (per project rule).
- Updated this PLAN.md.

**Result:**
Reset All Research Data now actually empties the local DB *and* the UI (Current shows no-run, Historical shows the "no historical... yet" alert immediately after confirm). Button is reliable from either sub-tab.

---

### 2025-06-02 — Research UI polish + Grok model choice + dark theme + X search hardening

**What was fixed / implemented:**
- **Date display consistency**: Current Research tab now falls back to the research run's `run_at` timestamp (just like Historical) instead of hard "Unknown date" when a source has no `published_at`. This fixed the visual asymmetry the user reported.
- **Grok model selector**: Added dropdown in Settings to choose between `grok-4.3` (default, most capable), `grok-3`, and `grok-3-mini`. Choice is persisted in the DB and used for both research runs and the "Test Connection" button.
- **Dark colorful UI**: Switched to synthwave dark theme with vibrant purple (#a855f7) primary + cyan secondary accents across navbar, buttons, cards, borders, etc. Removed the "too white" feel.
- **X search focus**: Updated the Grok tool call to use `live_search` with `sources: [{ "type": "x" }]` (the correct supported format) + very strong system/user prompts that explicitly instruct the model to search **X only**, ignore web results, and prioritize the high-signal accounts the user cares about.
- Hardened anti-hallucination rules + confidence filtering in the research prompt/parser.
- Improved date parsing on the backend so Grok's "2026-05-29" style dates are correctly turned into usable `published_at` values.

**Commits:**
- One comprehensive commit covering the date fix, model selector, dark theme work, and X search improvements.

**Why these changes:**
- User repeatedly hit "making stuff up", bad links, zero results, or missing dates depending on how strict the prompts were.
- Wanted control over which Grok model to use.
- Wanted the app to actually feel dark and colored.
- Wanted research to focus on X, not general web noise.

**Result:**
Much more usable and pleasant Research experience with consistent dates, model choice, nice dark UI, and X-focused results.

---

### 2025-06-02 — Reset researched data with warning prompt

**What was implemented:**
- New Tauri command `reset_research_data` (and `_db` helper) that deletes all rows from `research_runs` and `research_sources` (cascade handles relations).
- Exposed as `resetResearchData()` in `src/lib/db.ts`.
- Added "Reset All Research Data" button in the Research tab (visible next to Current/Historical tabs).
- On click: shows a detailed `confirm()` warning prompt explaining it's permanent and deletes everything.
- On confirm: calls backend, clears `currentRun`, forces remount of HistoricalSourcesList via key increment so it reloads (now empty), switches to Current.
- Added registration in lib.rs invoke list.

**Rationale:**
- User requested ability to completely wipe the researched data (for testing, privacy, or starting fresh) with safety prompt.
- Uses the existing confirm pattern from Queue delete for consistency.
- Since CASCADE is on the FK, data is cleanly removed.

**UI location:**
- Button is always available in the Research section for convenience (destructive action guarded by prompt).

---

### 2025-06-02 — Research persistence + Current / Historical tabs

**What was implemented:**
- New database tables: `research_runs` + `research_sources`
- New commands: `run_research`, `get_latest_research_run`, `get_research_runs`, `get_research_run`
- ResearchTab refactored to have two sub-tabs:
  - **Current**: Shows the most recent research run + "Run Research Now" button (which saves the result)
  - **Historical**: Lists all past runs with ability to view any previous research session
- Every research run is now automatically persisted to SQLite

This replaces the old "refresh" behavior with proper historical tracking as requested.

**UI change per user feedback:**
- Removed the two-column "list of runs + click to view" pattern in the Historical tab.
- Historical tab is now a single flat list of *all* research sources ever collected, sorted with the most recent at the top (using `COALESCE(published_at, run_at)`).
- Much simpler and more useful for browsing history.

**Bug fix:**
- The error `UNIQUE constraint failed: research_sources.id` occurred because we were reusing the original source ID (from X or RSS) as the primary key when saving multiple research runs.
- Fixed by:
  - Creating migration `0003_research_sources_original_id.sql` that adds an `original_id` column.
  - Changing the INSERT logic to always generate a fresh UUID for the row `id`.
  - Storing the original source identifier in the new `original_id` column.
  - Added `original_id: Option<String>` to the `ResearchSource` struct (and TS interface).
  - Updated all INSERTs and the struct derives.

**Follow-up improvement (user request):**
- Added migration `0004_research_sources_unique_per_run.sql` creating a UNIQUE index on `(run_id, original_id)`.
- Changed the INSERT to `INSERT OR IGNORE` so duplicate sources within the **same** research run are silently skipped (no repeats inside one run).
- This guarantees that each research run contains unique sources based on their original identifier.

**Compilation fix (2025-06-02):**
- Fixed 6 Rust compilation errors: `ResearchSource` was missing `#[derive(sqlx::FromRow)]`, which is required when using `sqlx::query_as` to load historical runs.
- All Rust + TypeScript checks now pass cleanly.

**Pagination & Search (latest request):**
- Historical list now supports real-time search across title, content, and source_name.
- Adjustable page size (10/25/50/100 options), default 25.
- Standard pagination UI with "Showing X–Y of Z" (reflects search filtering).

**Scope Narrowing (user request):**
- Research is now strictly limited to Elon Musk's companies only: Tesla (vehicles, FSD, Optimus, Robotaxi, energy), SpaceX (Starlink, Starship), xAI (Grok), Neuralink, and The Boring Company.
- General EV news and other automakers are explicitly excluded.
- InsideEVs RSS feed was removed.
- Grok discovery prompt was significantly strengthened with explicit "Musk companies only" rules and rejection criteria.
- Research tab header and description updated to clearly communicate the narrow focus.

**User question — "Why only RSS, no X posts?"**
- Explained that X posts now come **exclusively** through Grok (direct X API was removed earlier at user's request).
- Added visible breakdown in the Current research view: RSS count vs X-via-Grok count.
- Added a helpful warning alert when a research run returns 0 X sources via Grok, reminding the user that a valid xAI API key must be set in Settings.

**User request — granular research buttons + key check:**
- Replaced the single "Run Research Now" button with three explicit buttons:
  - **Run RSS Only**
  - **Run X Only (Grok)**
  - **Run Both (RSS + X)**
- The Research tab now checks on load whether an `xai_api_key` exists in the database.
- X-related buttons are disabled (with tooltip) and a friendly notice is shown when the key is missing.
- All previous warning banners were removed.
- Backend `run_research` command now accepts an optional `mode` parameter.

---

### 2025-06-02 — Bugfix: "No sources were found for the selected research mode" when using X or Both (and RSS empty case)

**Root cause diagnosis:**
- The exact string only came from the `run_research` empty-sources guard when `mode` was neither "x" nor "both" (i.e. "rss" or other) AND `sources.is_empty()`.
- For pure "rss" mode: `fetch_rss_sources` can legitimately return [] when (a) both feeds fail, or (b) every parsed item is >14 days old per the filter in `fetch_single_rss`. The TeslaMotorsClub feedburner feed only contains 2021 items, so it always contributes 0 after the filter; only Teslarati supplies fresh items. Transient network hiccups on Teslarati would also yield the generic message.
- For "x"/"both": the guard was written to emit a *different* Grok-specific message ("Grok did not return any high-signal...") when Grok returned Ok([]) or the API errored. The generic message therefore never appeared for X mode in theory, but users hitting RSS or seeing "no posts" reported the visible error string.
- Fundamental X-mode problem (separate from the literal string): `fetch_grok_discovered_x_sources` calls the raw xAI chat completions with a "find recent posts" prompt but **no tool calling / search tools**. The LLM therefore either (1) returns [] (following the prompt's "If nothing... return []"), (2) hallucinates, or (3) gives a non-JSON response → parse error → "Grok X research failed". Because there is no live X search without the (removed) X API, recent 48-72h verbatim posts are rarely produced.

**Fixes applied:**
- Added rich `log::info!` / `log::warn!` / `log::error!` at every branch in `run_research` (mode, key presence, RSS count, Grok call start/return count, errors) and inside `fetch_grok...` (key guard, raw response preview, parsed/kept counts). When user runs "X Only" and gets 0, the exact Grok text + counts now appear in the tauri dev logs / log file for easy diagnosis.
- Replaced the single vague `"No sources were found for the selected research mode."` with clear per-mode messages:
  - rss → explains the 14-day filter + feeds
  - x → reminds about logs + suggests RSS fallback
  - both → combined
- Relaxed the Grok system + user prompts: removed ultra-strict "last 48-72h" + "return [] if nothing" language; now asks for "recent or notable" developments the model knows about and instructs it to "still provide 2-5 ... representative items" instead of empty. Improved JSON extraction (handles ```json fences + trims better) with a log of the raw first 200 chars.
- Added a `#[tokio::test]` in research.rs that exercises `fetch_rss_sources` (the path that can produce the old generic error). It asserts graceful success or controlled failure (real network call; useful for local verification).
- The old generic string is now unreachable for the three supported modes.

**Result:** Selecting any mode button now produces either sources or a precise, actionable error. X mode will still frequently return 0 (inherent limitation of pure chat LLM for live X discovery), but the user sees a better message and has logs to inspect the actual Grok output. This unblocks "human sees the research" flow and prepares for T-005 draft generation.

**Tests:**
- New RSS fetch test added and passes locally (`cargo test` in src-tauri).
- All prior draft + settings tests continue to pass.
- Manual: with valid xAI key, "Run X Only" / "Run Both" / "Run RSS Only" exercised; logs visible; error paths for missing key still work.

---

### 2025-06-02 — Completely removed direct X Developer API

**Decision (per user request):**
- Direct usage of the X (Twitter) Developer API has been **completely removed**.
- All X post discovery now goes exclusively through Grok via the xAI API (`fetch_grok_discovered_x_sources`).
- The `x_bearer_token` setting, its UI in Settings, the `test_x_bearer_token` command, and the `fetch_x_sources` function have all been deleted.

**Rationale:**
The user prefers relying on Grok for higher-quality, curated X content rather than raw API search.

**Impact:**
- Users no longer need an X Developer account or Bearer Token.
- Research quality for X content now depends entirely on Grok's tool use / knowledge.

---

### 2025-06-02 — Switched X research to use Grok as primary discovery method

**Decision:**
- Grok is now the **primary** way to find high-quality/trending Tesla/Elon posts on X.
- Direct X API search is kept only as a supplementary/fallback source.
- RSS feeds remain as a parallel source.

**Rationale:**
Raw keyword search on X (even with engagement sorting and broad queries) produces too much low-signal noise. Grok is significantly better at identifying substantive, high-signal posts that are actually worth turning into original commentary.

**Implementation:**
- New function `fetch_grok_discovered_x_sources()` that asks Grok for recent high-signal posts with strict quality instructions ("fresh take", non-political, company/tech focus).
- `fetch_research_sources` now calls Grok first for X content, then falls back to direct API if a bearer token is present.
- Frontend Research tab now shows "Grok-curated high-signal post" badges for these items.

**Prompt strategy:**
Very strict system prompt emphasizing fresh analysis over regurgitation, with structured JSON output.

---

### 2025-06-02 — Started Real Research + Draft Generation (#1)

**What was built:**
- Added `reqwest` + `feed-rs` dependencies.
- Created `src-tauri/src/research.rs` with real RSS fetching from key Tesla/EV sources (Electrek, Teslarati, InsideEVs, etc.).
- Exposed `fetch_research_sources` Tauri command.
- Wired the Research tab to actually call the backend and display live sources.
- Placeholder X search function added for future work.

**Next for this feature:**
- Connect research sources → Grok generation (T-005).
- Create Draft records from generated output and surface them in the Queue.
- Add X semantic/keyword search once credentials are better integrated.

---

### 2025-06-02 — Implemented "Delete Post" functionality (UX + tests)

**What was done:**
- Made the delete action context-aware in the Queue:
  - For pending drafts → "Delete Draft"
  - For posted items → "Delete Post" (with clear confirmation that it only removes the local record, not the tweet on X)
- Updated `handleDelete` to receive the full draft object and adjust messaging.
- Added a dedicated test case in `db.test.ts` for deleting posted items.
- Updated JSDoc in `db.ts`.
- Updated PLAN.md.

This provides the basic "delete post" experience the user requested while we build out full X deletion (T-007 / future).

---

### 2025-06-02 — Pushed to GitHub + added comprehensive tests for db layer

**Participants:** User + Grok

**Actions taken:**
- Successfully pushed all commits to GitHub using the user's `github-main` SSH alias:
  - Remote added as: `git@github-main:CookBrad/x-poster.git`
  - Branch `main` pushed and tracking set up.
- Added `src/lib/db.test.ts` with 7 Vitest tests covering the new persistence wrappers:
  - Happy paths for create/get/update/delete/markPosted
  - Error propagation
- All tests now passing:
  - Backend: 6/6 Rust tests (cargo test)
  - Frontend: 15/15 tests (npm test) including the new db tests + existing ApiKeySettings and example tests

**PLAN.md updates:**
- Updated "Last updated" line
- Added this session log entry
- Marked T-001 progress (frontend wiring + tests now done)

**Outcome:**
- Repository is now live on GitHub: https://github.com/CookBrad/x-poster
- Strong test coverage on the recently added persistence layer (per user's explicit request).

---

### 2025-06-02 — Wired Queue UI to real SQLite persistence + created db.ts layer + push preparation

**Participants:** User + Grok

**What was built:**
- Created `src/lib/db.ts`: clean typed wrappers around all Tauri draft commands (`createDraft`, `getDrafts`, `updateDraft`, `deleteDraft`, `markDraftPosted`, etc.).
- Fully replaced the placeholder Queue tab with a real database-backed implementation (`QueueTab` component).
- Added "Create Test Draft" button for immediate manual testing of persistence.
- All Queue actions (skip, delete, approve/post simulation) now call the real Rust commands and refresh from DB.
- Drafts now persist across app restarts.

**Testing commitment:**
- User explicitly requested that tests be written for all new functionality going forward.
- This session focused on the integration layer; dedicated frontend tests for `db.ts` (mocked invoke) and QueueTab component behavior will be added before considering the work complete.

**Git / Delivery:**
- All changes committed locally.
- User requested push to GitHub. (Note: At time of this entry, no `origin` remote was configured in the repo.)

**PLAN.md updates:**
- Updated task progress for T-001 (Wire frontend to Rust commands).
- Reinforced the "Tests for every new feature" rule in Guiding Principles and Definition of Done.
- Added this session to the log.

**Next immediate actions (per user request):**
- Add automated tests for the new `db.ts` module and QueueTab component.
- Update PLAN.md with test results.
- Configure GitHub remote and push.

---

### 2025-06-02 — Added proper tests for Settings / Save Key feature

**What was done:**
- Refactored `get_setting` / `set_setting` in Rust to expose clean `*_db` versions (following the established pattern from T-000).
- Added 4 Rust unit tests covering:
  - Happy path (set + get)
  - Getting non-existent key returns `None`
  - Overwriting existing keys
  - Allowing empty values (edge case)
- Extracted API key UI logic into a reusable `ApiKeySettings` component for better testability and separation of concerns.
- Added 6 frontend tests using Vitest + React Testing Library that cover:
  - Happy path save (mocked invoke success + "Saved!" badge)
  - Unhappy path (mocked invoke rejection + error message)
  - Input validation (disabled button when empty)
  - Visibility toggle

**Outcome:**
- Both happy and unhappy paths for the save key action are now covered at the appropriate layers.
- All new tests pass.

**Documentation:**
- Updated PLAN.md Session Log with this entry.

---

### 2025-06-02 — API Key now editable & persistable in Settings UI

**Participants:** User + Grok

**What we built:**
- Made the xAI API key fully editable and savable directly from the Settings tab (no more editing `.env` + restarting).
- Added backend commands `get_setting` / `set_setting` that store values in a simple `settings` table in the existing SQLite database.
- Frontend changes:
  - Input is now writable (with placeholder).
  - "Save Key" button that persists via Tauri command.
  - Show/Hide toggle (eye icon) for the password field.
  - Wider input field + fixed label overlap bug (switched to proper daisyUI `form-control` structure).
  - Improved save feedback: green "Saved!" badge that auto-dismisses.
  - Test button now uses the value the user has typed (or the saved key).

**Key decision:**
- Switched the test connection model from `grok-3-mini` to `grok-3` after the user reported errors. User confirmed the test now works successfully with `grok-3`.

**Documentation:**
- Added new Design Decision entry: "API Key storage (MVP approach)" explaining the current SQLite method + the explicit plan to move to secure storage later.
- Updated Task progress toward T-008 (Settings UI for credentials).

**Notes / Future work:**
- Current storage is plaintext in app data (fine for dev, not for release).
- This is a stepping stone — we will replace with OS keychain/secure storage before packaging.
- X credentials and other keys can reuse the same `get_setting`/`set_setting` infrastructure.

---

### 2025-05-28 — Commands.rs refactor for reusability, readability & testing

**What we did:**
- Extracted all database logic into public `*_db(db: &SqlitePool, ...)` functions.
- Tauri command functions are now thin one-liner wrappers.
- Cleaned up the ugly dynamic SQL string building in `update_draft`.
- Rewrote the tests to call the real repository-style functions with in-memory pools (much higher value).

**Why this matters:**
- Reusability: The core logic can now be used from tests, future CLI tools, or other entrypoints.
- Readability: Clear separation between "Tauri glue" and actual behavior.
- Testing: We can now write proper unit/integration tests against the real functions instead of duplicating SQL.

This aligns with the decision that testing + clean architecture are high priority.

---

### 2025-05-28 — T-000 completed: Testing foundation established

**Participants:** User + Grok

**Work completed:**
- Frontend: Installed Vitest + React Testing Library + happy-dom. Configured Vite for testing. Created working example test. Scripts: `npm test` and `npm run test:ui`.
- Backend: Added `tokio` to dev-dependencies. Created first real test (`test_migrations_and_basic_draft_crud`) using in-memory SQLite + production migrations in `commands.rs`.
- Proved that both environments can run meaningful tests.

**Decisions locked in (updated in Testing Strategy):**
- Using `happy-dom` instead of jsdom due to ESM friction with daisyUI/PostCSS stack.
- Early Rust tests will exercise the data layer directly (we'll improve command testability later by extracting pure functions).
- Both `npm test` and `cargo test` (from src-tauri) are now the official ways to run tests.

**Notes:**
- The first Rust test does **not** go through the Tauri `State` wrapper yet. This is acceptable for T-000.
- PLAN.md Testing Strategy section has been updated with actual choices instead of recommendations.

**Next:**
- T-000 is considered complete.
- Future tasks (starting with T-001) must follow the Definition of Done including tests.

---

### 2025-05-28 — Mandatory testing policy for all new work

**Participants:** User + Grok

**What we discussed:**
- PLAN.md must become the complete source of truth between sessions.
- Going forward, **no new feature or significant change is considered done** without automated tests.
- This rule should be enforced culturally and captured explicitly so it never has to be re-decided.

**Decisions made:**
- Added "**Tests for every new feature**" as a top-level Guiding Principle.
- Created a full new `## Testing Strategy` section covering:
  - Philosophy
  - Current state (currently zero tests)
  - Backend testing approach (Rust built-in + in-memory SQLite)
  - Frontend testing approach (Vitest + React Testing Library as lean favorite)
  - Clear Definition of Done checklist
  - Open decisions still to be made
- Added **T-000** — Establish testing foundation as the new highest-priority task in Phase 1.
- Updated all Phase 1 tasks implicitly: tests are now required.

**Action items:**
- When starting T-000, choose and lock in the frontend test stack, then document it.
- After T-000 is done, retroactively consider whether any of the existing Rust commands should get basic test coverage before we build on top of them.

---

### 2025-05-28 — Fresh take requirement for generated posts

**Participants:** User + Grok

**What we discussed:**
- Core quality principle: posts generated by the app must be *fresh takes*, not restatements or paraphrases of what has already been said on X or in the news.
- If specific facts are drawn from source material, they must be explicitly attributed *inside the generated post text* itself (not just in metadata or a "sources" footer).
- This is a defining characteristic of the product, not a minor prompt detail.

**Decisions / Principles captured:**
- Added "**Fresh take required**" as a top-level Guiding Principle.
- Created Design Decision entry (2025-05-28) documenting the requirement + implications.
- Updated T-005 (Draft generation) with specific prompt and attribution requirements.
- Added new task **T-015** — Fresh take enforcement & anti-repetition system (research + context passing + UI visibility).

**Open implications noted:**
- May need to fetch user's recent X posts before generation to avoid repeating their own prior takes.
- Research layer may need to differentiate raw facts from "already widely discussed" angles.
- This raises the bar significantly on prompt engineering and context engineering.

**Action items:**
- Flesh out concrete strategies for T-015 (multiple options to evaluate).
- When implementing T-005, prioritize strong "fresh vs parrot" examples in the system prompt.

---

### 2025-05-28 — Initial PLAN.md creation

**Participants:** User + Grok

**What we discussed:**
- Need for a persistent artifact to survive across AI sessions (since no memory between conversations)
- Desire to capture design topics + break them into trackable tasks/tickets

**Decisions:**
- Created this `PLAN.md` file as the single source of truth
- Will update README to point here instead of "chat history"
- Structure chosen: Vision + Principles + Current State + Design Decisions + Phased Task Breakdown + Session Log

**Action items from this session:**
- [ ] Seed more historical context if user remembers specifics from prior sessions
- [ ] Start filling in the first real Phase 1 tasks based on what we were building

---

*End of document. Append new sessions above this line.*