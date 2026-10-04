import { BrowserRouter, Navigate, Route, Routes } from "react-router";
import { BrowsePage } from "./pages/Browse.tsx";
import { AuthCallback } from "./pages/AuthCallback.tsx";

function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route path="/auth_callback" element={<AuthCallback />} />
        <Route path="/browse/*" element={<BrowsePage />} />
        <Route path="/" element={<Navigate to="/browse/" />} />
      </Routes>
    </BrowserRouter>
  )
}

export default App
