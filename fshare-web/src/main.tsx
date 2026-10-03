// import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
// import './index.css'
import App from './App.tsx'
import { StrictMode } from 'react'
import { AuthProvider, type AuthProviderProps } from 'react-oidc-context'
import { initializeFileTypeIcons } from '@fluentui/react-file-type-icons';
import { userManager } from './oidc.ts';


// Register icons and pull the fonts from the default Microsoft Fluent CDN:
initializeFileTypeIcons();


const oidcConfig: AuthProviderProps = {
  onSigninCallback: (user) => {
    const returnTo = (user?.state as { returnTo?: string })?.returnTo;

    if (
      typeof returnTo !== "string" ||
      !returnTo.startsWith("/")
      || returnTo.startsWith("//")
    ) {
      window.location.replace("/browse/");
    } else {
      window.location.replace(returnTo);
    }
  }
};

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <AuthProvider {...oidcConfig} userManager={userManager} >
      <App />
    </AuthProvider>
  </StrictMode>
)
