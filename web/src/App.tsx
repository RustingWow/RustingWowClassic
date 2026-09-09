import { BrowserRouter, Route, Routes } from "react-router-dom";

import { AuthProvider } from "./auth";
import { Layout } from "./Layout";
import { Account } from "./pages/Account";
import { ChangelogPage } from "./pages/Changelog";
import { Connect } from "./pages/Connect";
import { Home } from "./pages/Home";
import { IdeaDetail } from "./pages/IdeaDetail";
import { IdeaEdit } from "./pages/IdeaEdit";
import { IdeaNew } from "./pages/IdeaNew";
import { Ideas } from "./pages/Ideas";
import { Login } from "./pages/Login";
import { Register } from "./pages/Register";

export default function App() {
  return (
    <AuthProvider>
      <BrowserRouter>
        <Layout>
          <Routes>
            <Route path="/" element={<Home />} />
            <Route path="/ideas" element={<Ideas />} />
            <Route path="/ideas/new" element={<IdeaNew />} />
            <Route path="/ideas/:id/edit" element={<IdeaEdit />} />
            <Route path="/ideas/:id" element={<IdeaDetail />} />
            <Route path="/changelog" element={<ChangelogPage />} />
            <Route path="/register" element={<Register />} />
            <Route path="/login" element={<Login />} />
            <Route path="/connect" element={<Connect />} />
            <Route path="/account" element={<Account />} />
          </Routes>
        </Layout>
      </BrowserRouter>
    </AuthProvider>
  );
}
