import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import App from './App.tsx'
import { initializeFileTypeIcons } from '@fluentui/react-file-type-icons';

// Register icons and pull the fonts from the default Microsoft Fluent CDN:
initializeFileTypeIcons();

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
)
