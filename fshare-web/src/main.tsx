import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import App from './App.tsx'
import { initializeFileTypeIcons } from '@fluentui/react-file-type-icons';
import keycloak from './keycloak.ts';
import { api } from './api.ts';

// Register icons and pull the fonts from the default Microsoft Fluent CDN:
initializeFileTypeIcons();


const initApp = (token: string | undefined, refresh: string | undefined, existing_session: boolean, retries: number) => {
  keycloak.init({ token, refreshToken: refresh, checkLoginIframe: false }).then(async () => {
    console.log("KC INIT SUCCESS");

    if (keycloak.authenticated && !existing_session) {
      // Just returned from keycloak. Ensure session also exists on server
      await api("/authenticate-session", {
        method: "post",
        body: JSON.stringify({ token: keycloak.token, refresh: keycloak.refreshToken })
      })
    }

    createRoot(document.getElementById('root')!).render(
      <StrictMode>
        <App />
      </StrictMode>,
    )
  }, error => {
    console.log("Init had error:", error);
    if (retries > 0) {
      // TODO VIRKER IKKE. DEN SKAL SLETTE SESSION OG REFRESH
      initApp(undefined, undefined, false, retries - 1);
    } else {
      console.log("Failed to init app.")
    }
  });
}

api("/reload-session",).then(async resp => {
  let token = null;
  let refresh = null;
  let existing_session = false;
  if (resp.ok) {
    const { token: respToken, refresh: respRefresh } = await resp.json();
    token = respToken;
    refresh = respRefresh;
    existing_session = true;
    console.log("REALOADING SUCCESS");
  }
  initApp(token, refresh, existing_session, 1);
});


