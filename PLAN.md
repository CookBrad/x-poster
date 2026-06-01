# x-poster — Design & Task Plan

> **Living document.** This file captures architecture decisions, design discussions, tradeoffs, and the current task breakdown.
> Update it after any significant conversation or when priorities shift.
>
> Last updated: 2025-06-02 (Fixed UNIQUE constraint on research_sources.id by adding original_id + using surrogate PKs for saved sources)

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

---

## Testing Strategy

This section exists so that future sessions have clear, consistent guidance on how we approach testing. Update it as we make concrete tooling and process decisions.

### Philosophy
- Tests are **mandatory** for every new feature or significant change.
- "Feature complete" = working code + passing tests + updated documentation (where relevant).
- Prefer pragmatic, high-signal tests over 100% coverage theater.
- Tests should be fast, reliable, and easy to run locally.
- The existence of this rule in PLAN.md means we never have the "we'll add tests later" conversation again.

### Current State (as of 2025-05-28)
- **Frontend**: Vitest + React Testing Library + happy-dom is set up and working. One example test exists (`src/test/example.test.tsx`).
- **Backend**: First real test added in `commands.rs` (`test_migrations_and_basic_draft_crud`). Uses in-memory SQLite + real migrations. `tokio` added to dev-dependencies.
- We now have a proven pattern for both sides.

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
- None — the backend is ahead of the frontend

### Not Started (but newly prioritized)
- **T-000**: Establish testing foundation for Rust + frontend (now highest priority before more feature work)

### Not Started
- Real research pipeline (X + RSS)
- Draft generation via Grok
- Wiring React frontend to Rust commands
- Editable draft UI
- X posting flow
- Background scheduler / tray icon
- Secure key storage (for packaged builds)

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

- [ ] **T-000** — Establish testing foundation (highest priority before building more features)
  - Set up frontend test runner (Vitest + React Testing Library recommended)
  - Add basic Rust test infrastructure (test module patterns + in-memory DB helper)
  - Write at least one example test for an existing command (e.g. `create_draft` or `get_drafts`) to prove the pattern works
  - Document the testing workflow in PLAN.md under Testing Strategy

- [x] **T-001** — Wire React frontend to Rust draft commands (mostly complete)
  - [x] Replace placeholder cards with real data from `get_drafts`
  - [x] Implement approve / skip / delete actions calling the backend (via new `db.ts`)
  - [x] Add loading + error states
  - [ ] Add automated frontend tests for `db.ts` and QueueTab (in progress — user requested)
  - Note: "Create Test Draft" helper added for fast manual verification. Full "Research → Generate" flow will feed this queue later (T-006).

- [ ] **T-002** — Build basic draft editing UI
  - Click-to-edit text in a card or modal
  - Simple image URL field (or later Unsplash integration)
  - Live preview of what will be posted

- [ ] **T-003** — Implement X research module (backend)
  - X API client (start with OAuth 1.0a user context)
  - Semantic + keyword search for Tesla/TSLA/Elon topics
  - Deduplication / freshness logic

- [ ] **T-004** — Implement RSS research module (backend)
  - Fetch from key sources (Electrek, Tesla, etc.)
  - Parse and extract relevant items

- [ ] **T-005** — Draft generation via xAI Grok
  - Create a prompt template that includes research results
  - **Critical requirement:** Force "fresh take" behavior — original analysis, implications, or novel framing instead of restating facts or parroting existing commentary. Include strong instructions + few-shot examples of good vs bad output style.
  - When facts are used, require explicit inline attribution in the generated text (e.g. "According to @Tesla's Q2 delivery numbers..." or "As reported by Electrek...").
  - Call Grok, parse response, create `Draft` records
  - Store sources_json properly for citation and for UI display
  - Consider passing the user's recent posts (from X or local history) into the prompt context to reduce repetition of their own prior takes.

- [ ] **T-006** — Manual "Research Now" flow (Research tab)
  - Button that triggers research + generation cycle
  - Shows progress / results summary

- [ ] **T-007** — Real X posting flow
  - Use stored X credentials
  - Post text + optional image
  - On success: call `mark_draft_posted` with the X post ID
  - Basic error handling + retry

- [ ] **T-008** — Settings UI for all credentials
  - xAI key (already partially there)
  - X API keys (multiple formats)
  - Optional: Unsplash key

- [ ] **T-015** — Fresh take enforcement & anti-repetition system
  - Design and implement mechanisms to reduce "parroting" existing discourse.
  - Options to evaluate: (a) pass user's recent X posts into generation prompt, (b) maintain local cache of recently posted drafts, (c) research layer detects "already widely discussed" angles vs raw facts.
  - Add UI affordance (in draft card or editing view) to show what the system considered "already covered."

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
- Do we want to support both OAuth 1.0a and OAuth 2.0 for X, or just one?
- Image strategy: stock photos, AI-generated, or none for MVP?
- Rate limiting / cost control for xAI calls?
- **Fresh take specifics:** How strictly do we enforce original analysis vs allowing some factual summarization? Should the app fetch the user's own recent X posts before generation to avoid self-repetition? How do we detect "already widely discussed angles" in research results?

Add new questions here as they come up. Resolve and move to Design Decisions when answered.

---

## Session Log

This section captures key discussions from conversations so future sessions can pick up context quickly.

**Format:** Add new entries at the **top**.

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

**Bug fix:**
- The error `UNIQUE constraint failed: research_sources.id` occurred because we were reusing the original source ID (from X or RSS) as the primary key when saving multiple research runs.
- Fixed by:
  - Creating migration `0003_research_sources_original_id.sql` that adds an `original_id` column.
  - Changing the INSERT logic to always generate a fresh UUID for the row `id`.
  - Storing the original source identifier in the new `original_id` column.
  - Added `original_id: Option<String>` to the `ResearchSource` struct (and TS interface).
  - Updated all INSERTs and the struct derives.

**Compilation fix (2025-06-02):**
- Fixed 6 Rust compilation errors: `ResearchSource` was missing `#[derive(sqlx::FromRow)]`, which is required when using `sqlx::query_as` to load historical runs.
- All Rust + TypeScript checks now pass cleanly.

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