// SPDX-License-Identifier: GPL-3.0-or-later
import { HashRouter, Route, Routes } from "react-router";
import { AppProvider } from "@/context/AppContext";
import Layout from "@/components/Layout";
import Dashboard from "@/pages/Dashboard";
import Cas from "@/pages/Cas";
import Certificates from "@/pages/Certificates";
import CertificateDetail from "@/pages/CertificateDetail";
import Revocations from "@/pages/Revocations";
import Settings from "@/pages/Settings";

export default function App() {
  return (
    <AppProvider>
      <HashRouter>
        <Routes>
          <Route element={<Layout />}>
            <Route index element={<Dashboard />} />
            <Route path="cas" element={<Cas />} />
            <Route path="certificates" element={<Certificates />} />
            <Route path="certificates/:id" element={<CertificateDetail />} />
            <Route path="revocations" element={<Revocations />} />
            <Route path="settings" element={<Settings />} />
          </Route>
        </Routes>
      </HashRouter>
    </AppProvider>
  );
}
