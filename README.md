# x-poster

**Local-first desktop app** (Tauri v2 + React 19 + TypeScript) that:

- Discovers high-signal, non-political updates strictly about Elon Musk’s companies (Tesla/TSLA + Cybertruck/FSD/Optimus/Robotaxi/Megapack, SpaceX, xAI/Grok, Neuralink, Boring Company) via curated RSS feeds + Grok-powered X search (using the native `x_search` tool — **no X Developer API keys are used for research**).
- Generates draft posts with xAI Grok (multiple styles: Insight/default, Informative, Funny, Witty, Meme; grok-4.3 recommended).
- Lets you **review every single draft**, edit the text, choose/resolve images, and explicitly approve before anything is posted.
- Posts only after your approval using your own X OAuth 1.0a credentials.

Everything is stored locally in a SQLite database (`x-poster.db`). No cloud sync. Human stays in full control at every step.

## Current Status

MVP is functionally complete for the core loop:

- Research tab (Current + Historical sub-tabs, RSS / X / Both modes, per-source "Generate Post" buttons, bulk generation, search/pagination on history, "Reset All Research Data").
- Draft generation from research sources (avoids re-using sources), from individual sources, or from arbitrary pasted URLs / X posts / free-text topics.
- Posts/Queue tab with full editing, image support, Grok rationale display, approve & post.
- Settings: persisted xAI key + selectable Grok model, X OAuth 1.0a credentials (4 fields), test buttons for both.
- Nice dark synthwave-themed UI (daisyUI + custom purple/cyan accents).

See [PLAN.md](./PLAN.md) for detailed history, design decisions, and the living task list.

## Prerequisites (macOS)

- **Node.js** 20+ LTS (or recent 18+)
- **Rust** (stable, minimum 1.77.2 — latest stable strongly recommended)
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  rustup update
  ```
  For Apple Silicon Macs (M1/M2/M3/M4):
  ```bash
  rustup target add aarch64-apple-darwin
  ```
  (Add `x86_64-apple-darwin` too if you need Intel builds.)
- **Xcode Command Line Tools**
  ```bash
  xcode-select --install
  ```

## Setup

1. Install JavaScript dependencies:
   ```bash
   npm install
   ```

2. Get the required API credentials:

   - **xAI / Grok key** (required for research + draft generation):  
     Create one at https://x.ai/api (or the xAI console). The free tier is sufficient for normal use.

   - **X (Twitter) credentials** (required only if you want to post drafts):  
     - Go to https://developer.x.com/ and create a Project + App.
     - Set the app permissions to **Read + Write** (and Direct Messages if desired).
     - Generate:
       - API Key + API Secret (consumer key/secret)
       - Access Token + Access Token Secret (with write access)
     - You need all four for OAuth 1.0a posting.

3. (Optional, development only) Create a `.env` file for a quick xAI fallback:
   ```bash
   cp .env.example .env
   ```
   Put your key in `VITE_XAI_API_KEY=sk-...`

   **Note**: The app primarily uses keys you save inside the **Settings** tab (persisted to your local SQLite DB). The `.env` VITE_ key is only a dev convenience fallback.

## Running the App (Development)

```bash
npm run tauri dev
```

- This runs the Vite dev server (`npm run dev`) and launches the native Tauri desktop window.
- First run will compile the Rust backend (can take 1–5 minutes).
- Frontend changes hot-reload.
- Rust changes require a full window restart.

The window title is "x-poster". Default size is comfortable for a laptop.

## First-Time Configuration & Core Workflow

1. Open the **Settings** tab.
2. **Research & Drafts**:
   - Paste your xAI key → Save.
   - (Optional) Change the Grok model (grok-4.3 is the current most capable default).
   - Click **Test Connection**.
3. **Posting to X** (if you want to post):
   - Fill the four credential fields (labels match the X Developer Portal exactly).
   - Save.
   - Click **Test X Credentials**.
4. Go to the **Research** tab.
   - Choose a mode (RSS only, X via Grok only, or Both).
   - Click the corresponding Run button (or "Research + Generate" for the full pipeline).
   - Browse sources in Current or Historical. Click **Generate Post** on any individual source, or use the bulk buttons.
5. Switch to the **Posts** tab (your queue).
   - Edit text + optional image URL.
   - See the original research sources + Grok’s “intended insight / added value” rationale.
   - **Approve & Post** (or Skip / Delete).
6. Posted items move to the posted view. You can still delete the local history record (it never deletes the actual tweet).

All research, drafts, settings, and credentials live in:
`xposter/x-poster.db`

Delete that folder (or use the **Reset All Research Data** button) to start fresh.

## Building a Release Version

```bash
npm run tauri build
```

Bundles appear in `src-tauri/target/release/bundle/`.

- On macOS you’ll get a `.app` and usually a `.dmg`.
- For distribution outside your machine you should sign + notarize the app (see the official Tauri macOS signing guide).
- Packaged builds do **not** read `.env` — users configure everything in the in-app Settings (persisted locally).

## Troubleshooting

- **First run takes forever / Rust errors**: Make sure you ran the `rustup target add` commands for your architecture and that Xcode CLTs are installed.
- **“xAI API key is required”**: The key must be saved in Settings (or present as `VITE_XAI_API_KEY` in `.env` during dev). Research X mode and all generation require it.
- **X posting / credential test fails**: Double-check that the app in the X Developer Portal has Read+Write permissions **and** that the Access Token you are using was generated *after* you changed the permissions. Regenerate the token if needed.
- **No (or very few) X research results**: The discovery is intentionally narrow (Musk companies only + high-confidence items from known voices). Try “Both” mode or run research a couple of times. Check the terminal logs — full raw Grok responses are logged for diagnosis.
- **App feels slow on first research/generation**: The first Grok calls + Rust compilation of certain paths can be noticeable. Subsequent runs are fast.
- **Database locked or weird state**: Quit the app completely, or delete the `x-poster.db` file in the app support directory.
- Logs: In development they appear in the terminal that launched `tauri dev` (plus the tauri-plugin-log output).

## Architecture & Philosophy (Quick)

- **Frontend**: React + TypeScript + Vite + Tailwind + daisyUI (synthwave-based dark theme with purple/cyan accents).
- **Backend**: Tauri 2 (Rust) using `sqlx` + embedded SQLite migrations. No Tauri SQL plugin — direct control.
- **Research**:
  - RSS: `feed-rs` (currently Teslarati + Not a Tesla App; 14-day freshness filter).
  - X: Grok Responses API + the `x_search` (live_search) tool with `sources: [{type:"x"}]`. Strict “Musk companies only”, high-confidence, anti-hallucination rules. No X API keys ever used for discovery.
- **Draft generation**: Grok (JSON output) with rich style-specific system prompts that emphasize *fresh insight / anti-regurgitation*, one stock cashtag max (`$TSLA` or `$SPCX`), inline attribution rules, and recent-posted deduplication. Post-processing normalizes attribution and enforces cashtag limits.
- **Images**: Meta extraction for custom sources + optional Grok image generation for Meme style.
- **Posting**: Pure manual OAuth 1.0a request signing (no official X SDK). Human approval gate is non-negotiable in the MVP.
- **Everything local**. Settings, research history, drafts, and X credentials are all in the local SQLite file.

See the source (especially `src-tauri/src/{research,generation,commands}.rs`, `src/components/{ResearchTab,PostsTab,SettingsTab}.tsx`, and `src/lib/db.ts`) for the real details.

## License & Notes

Personal project. All posts you make are your own responsibility.

The guiding principle is **“fresh take required”** — drafts must add original analysis, implications, or a novel angle. They must not merely restate the source.

Happy posting (after you review it)! 🚀

If you’re hacking on it: run the tests with `npm test` and `cd src-tauri && cargo test`. Full task history lives in [PLAN.md](./PLAN.md).
