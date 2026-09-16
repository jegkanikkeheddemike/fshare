import { BrowserRouter, Route, Routes } from "react-router";
import { Index } from "./pages/Index";
import { ApproveSession } from "./pages/ApproveSession";
import { ReactKeycloakProvider } from "@react-keycloak/web";

import { PrivateRoute } from "./auth";

import keycloak from "./keycloak.ts"



function App() {
  return (
    <ReactKeycloakProvider authClient={keycloak}>
      <BrowserRouter>
        <Routes>
          <Route path="/" element={<Index />} />
          <Route path="/approve_session/:sessionId" element={<PrivateRoute> <ApproveSession /></PrivateRoute>} />
        </Routes>
      </BrowserRouter>
    </ReactKeycloakProvider>
  )
}

export default App
