import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import './index.css'
import App from './App.tsx'

// Force dark colorful theme on load (synthwave gives nice neon accents)
document.documentElement.setAttribute('data-theme', 'synthwave')

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
)
