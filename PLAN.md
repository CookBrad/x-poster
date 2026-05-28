# x-poster — Design & Task Plan

> **Living document.** This file captures architecture decisions, design discussions, tradeoffs, and the current task breakdown.
> Update it after any significant conversation or when priorities shift.
>
> Last updated: 2025-05-28 (Fresh take principle added)

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
- **Simple & reliable** — Prefer boring, debuggable solutions over clever ones.
- **Secure by default** — Keys move to OS secure storage before any public distribution.

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

- [ ] **T-001** — Wire React frontend to Rust draft commands
  - Replace placeholder cards with real data from `get_drafts`
  - Implement approve / skip / delete actions calling the backend
  - Add loading + error states

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