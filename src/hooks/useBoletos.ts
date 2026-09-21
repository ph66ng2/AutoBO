import { useState, useCallback } from "react";
import { listarBoletos, metricasDashboard } from "../lib/db";
import type { Boleto, DashboardMetricas } from "../types";

export function useBoletos() {
  const [boletos, setBoletos] = useState<Boleto[]>([]);
  const [loading, setLoading] = useState(false);

  const fetchBoletos = useCallback(async () => {
    setLoading(true);
    try {
      const data = await listarBoletos();
      setBoletos(data);
    } catch (e) {
      console.error("Failed to fetch boletos:", e);
    } finally {
      setLoading(false);
    }
  }, []);

  const fetchMetricas = useCallback(async (): Promise<DashboardMetricas> => {
    try {
      return await metricasDashboard();
    } catch (e) {
      console.error("Failed to fetch metrics:", e);
      return {
        boletos_gerados: 0,
        boletos_pagos: 0,
        valor_a_receber: 0,
        valor_recebido: 0,
        boletos_vencidos: 0,
      };
    }
  }, []);

  return { boletos, loading, fetchBoletos, fetchMetricas };
}
