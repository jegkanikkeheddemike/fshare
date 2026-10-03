import { BrowserRouter, Navigate, Route, Routes } from "react-router";
import { ApproveSessionPage } from "./pages/ApproveSession";
import { PrivateRoute } from "./components/PrivateRoute.tsx";
import { BrowsePage } from "./pages/Browse.tsx";
import { LoginPage } from "./pages/Login.tsx";
import { AuthCallback } from "./pages/AuthCallback.tsx";

function App() {
  return (
      <BrowserRouter>
        <Routes>
          <Route path="/auth_callback" element={<AuthCallback />} />
          <Route path="/login/"  element={<LoginPage />} />
          <Route path="/approve_session/:sessionId" element={<PrivateRoute> <ApproveSessionPage /></PrivateRoute>} />
          <Route path="/browse/*" element={<BrowsePage />} />
          <Route path="/" element={<Navigate to="/browse/"/> } />
        </Routes>
      </BrowserRouter>
  )
}

export default App
