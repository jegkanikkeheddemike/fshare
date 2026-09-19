import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import App from './App.tsx'
import { initializeFileTypeIcons } from '@fluentui/react-file-type-icons';
import keycloak from './keycloak.ts';

// Register icons and pull the fonts from the default Microsoft Fluent CDN:
initializeFileTypeIcons();

keycloak.init({onLoad: "check-sso",}).then(() => {
  createRoot(document.getElementById('root')!).render(
    <StrictMode>
      <App />
    </StrictMode>,
  )
});
