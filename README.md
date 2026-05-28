# x-poster

Local-first desktop app (Tauri + React) that researches Tesla/TSLA/Elon company topics via X + RSS, generates drafts with xAI Grok, lets you review/approve in a nice UI, and posts to X.

**Current status (MVP early shell):** Tauri + React + Tailwind + daisyUI scaffold complete. Working "Test xAI Connection" button in Settings. Placeholder queue UI.

## Quick Start (macOS)

1. **Paste your keys**
   - Open `.env` in the project root
   - Paste your xAI key: `VITE_XAI_API_KEY=sk-...`
   - (Later also add your X API credentials in the same file)

2. **Install deps** (already done in this workspace)
   ```bash
   npm install
   ```

3. **Run the app (with hot reload)**
   ```bash
   npm run tauri dev
   ```

   This starts the Vite dev server + the native Tauri window.

4. Go to the **Settings** tab → click **Test xAI Connection**.  
   You should see a successful reply from Grok if the key is valid.

## Project Structure (key files)
- `.env` + `.env.example` — your API keys (gitignored)
- `src/App.tsx` — main UI (tabs + draft queue shell)
- `src-tauri/tauri.conf.json` — native window, bundle id, etc.
- `src-tauri/src/main.rs` — Rust entry (we'll extend for tray, scheduler, secure storage)

## Next steps we're building
- Real hybrid research (X search + RSS)
- Editable draft cards with image support
- Proper X posting flow
- Background scheduler + tray
- Secure (non-.env) key storage for packaged builds

See [PLAN.md](./PLAN.md) for the current task breakdown, design decisions, and session notes.

## Important Notes
- This is **local dev only** right now. Keys in `.env` are for development.
- The final packaged app will use secure OS storage so you can distribute the binary without leaking keys.
- All posts require explicit human approval in the MVP (you stay in control).

Currently, two official plugins are available:

- [@vitejs/plugin-react](https://github.com/vitejs/vite-plugin-react/blob/main/packages/plugin-react) uses [Oxc](https://oxc.rs)
- [@vitejs/plugin-react-swc](https://github.com/vitejs/vite-plugin-react/blob/main/packages/plugin-react-swc) uses [SWC](https://swc.rs/)

## React Compiler

The React Compiler is not enabled on this template because of its impact on dev & build performances. To add it, see [this documentation](https://react.dev/learn/react-compiler/installation).

## Expanding the ESLint configuration

If you are developing a production application, we recommend updating the configuration to enable type-aware lint rules:

```js
export default defineConfig([
  globalIgnores(['dist']),
  {
    files: ['**/*.{ts,tsx}'],
    extends: [
      // Other configs...

      // Remove tseslint.configs.recommended and replace with this
      tseslint.configs.recommendedTypeChecked,
      // Alternatively, use this for stricter rules
      tseslint.configs.strictTypeChecked,
      // Optionally, add this for stylistic rules
      tseslint.configs.stylisticTypeChecked,

      // Other configs...
    ],
    languageOptions: {
      parserOptions: {
        project: ['./tsconfig.node.json', './tsconfig.app.json'],
        tsconfigRootDir: import.meta.dirname,
      },
      // other options...
    },
  },
])
```

You can also install [eslint-plugin-react-x](https://github.com/Rel1cx/eslint-react/tree/main/packages/plugins/eslint-plugin-react-x) and [eslint-plugin-react-dom](https://github.com/Rel1cx/eslint-react/tree/main/packages/plugins/eslint-plugin-react-dom) for React-specific lint rules:

```js
// eslint.config.js
import reactX from 'eslint-plugin-react-x'
import reactDom from 'eslint-plugin-react-dom'

export default defineConfig([
  globalIgnores(['dist']),
  {
    files: ['**/*.{ts,tsx}'],
    extends: [
      // Other configs...
      // Enable lint rules for React
      reactX.configs['recommended-typescript'],
      // Enable lint rules for React DOM
      reactDom.configs.recommended,
    ],
    languageOptions: {
      parserOptions: {
        project: ['./tsconfig.node.json', './tsconfig.app.json'],
        tsconfigRootDir: import.meta.dirname,
      },
      // other options...
    },
  },
])
```
