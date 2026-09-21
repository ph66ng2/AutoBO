import { useEffect, useState } from "react";
import { BrowserRouter, Navigate, Route, Routes } from "react-router-dom";
import { AppLayout } from "./components/layout/AppLayout";
import Dashboard from "./pages/Dashboard";
import Boletos from "./pages/Boletos";
import NovoBoleto from "./pages/NovoBoleto";
import Clientes from "./pages/Clientes";
import Equipamentos from "./pages/Equipamentos";
import Estoque from "./pages/Estoque";
import Configuracoes from "./pages/Configuracoes";
import { verificarIntegracaoAutoos } from "./lib/db";
import type { IntegracaoAutoOS } from "./types";

const INTEGRACAO_VAZIA: IntegracaoAutoOS = {
  clientes: false,
  equipamentos: false,
  produtos: false,
};

export default function App() {
  const [integracao, setIntegracao] = useState<IntegracaoAutoOS>(INTEGRACAO_VAZIA);

  useEffect(() => {
    verificarIntegracaoAutoos()
      .then(setIntegracao)
      .catch(() => setIntegracao(INTEGRACAO_VAZIA));
  }, []);

  return (
    <BrowserRouter>
      <Routes>
        <Route element={<AppLayout integracao={integracao} />}>
          <Route path="/" element={<Dashboard />} />
          <Route path="/boletos" element={<Boletos />} />
          <Route path="/novo-boleto" element={<NovoBoleto />} />
          <Route path="/configuracoes" element={<Configuracoes />} />
          <Route
            path="/clientes"
            element={integracao.clientes ? <Clientes /> : <Navigate to="/" replace />}
          />
          <Route
            path="/equipamentos"
            element={integracao.equipamentos ? <Equipamentos /> : <Navigate to="/" replace />}
          />
          <Route
            path="/estoque"
            element={integracao.produtos ? <Estoque /> : <Navigate to="/" replace />}
          />
        </Route>
      </Routes>
    </BrowserRouter>
  );
}
