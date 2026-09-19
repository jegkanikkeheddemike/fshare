import { BrowserRouter, Route, Routes } from "react-router";
import { IndexPage } from "./pages/Index";
import { ApproveSessionPage } from "./pages/ApproveSession";
import { ReactKeycloakProvider } from "@react-keycloak/web";

import { PrivateRoute } from "./auth";

import keycloak from "./keycloak.ts"
import { BrowsePage } from "./pages/Browse.tsx";



function App() {
  return (
    <ReactKeycloakProvider authClient={keycloak}>
      <BrowserRouter>
        <Routes>
          <Route path="/" element={<IndexPage />} />
          <Route path="/approve_session/:sessionId" element={<PrivateRoute> <ApproveSessionPage /></PrivateRoute>} />
          <Route path="/browse" element={<BrowsePage />} />
          <Route path="/browse/*" element={<BrowsePage />} />
        </Routes>
      </BrowserRouter>
    </ReactKeycloakProvider>
  )
}

export default App
