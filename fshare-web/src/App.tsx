import { BrowserRouter, Navigate, Route, Routes } from "react-router";
import { LoginPage } from "./pages/Login.tsx";
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
          <Route path="/login/" element={<LoginPage />} />
          <Route path="/approve_session/:sessionId" element={<PrivateRoute> <ApproveSessionPage /></PrivateRoute>} />
          <Route path="/browse/" element={<BrowsePage />} />
          <Route path="/browse/*" element={<BrowsePage />} />
          <Route path="/" element={<Navigate to="/browse/"/> } />
        </Routes>
      </BrowserRouter>
    </ReactKeycloakProvider>
  )
}

export default App
