import { useState } from "react";
import { importarDanfePdf, importarNfe, importarNfeLote } from "../lib/db";
import type { NFeImportadaDTO } from "../types";

function fileToBase64(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => {
      const result = reader.result as string;
      const base64 = result.split(",")[1] || result;
      resolve(base64);
    };
    reader.onerror = reject;
    reader.readAsDataURL(file);
  });
}

export function useNFeImport() {
  const [resultados, setResultados] = useState<NFeImportadaDTO[]>([]);
  const [importando, setImportando] = useState(false);
  const [erro, setErro] = useState<string | null>(null);

  async function handleImportarXML(file: File) {
    setImportando(true);
    setErro(null);
    try {
      const base64 = await fileToBase64(file);
      const result = await importarNfe(base64) as NFeImportadaDTO;
      setResultados((prev) => [...prev, result]);
      return result;
    } catch (e) {
      setErro(String(e));
      return null;
    } finally {
      setImportando(false);
    }
  }

  async function handleImportarLote(files: File[]) {
    setImportando(true);
    setErro(null);
    try {
      const base64Files = await Promise.all(files.map(fileToBase64));
      const results = await importarNfeLote(base64Files) as NFeImportadaDTO[];
      setResultados(results);
      return results;
    } catch (e) {
      setErro(String(e));
      return [];
    } finally {
      setImportando(false);
    }
  }

  async function handleImportarPDF(file: File) {
    setImportando(true);
    setErro(null);
    try {
      const base64 = await fileToBase64(file);
      const result = await importarDanfePdf(base64) as NFeImportadaDTO;
      setResultados((prev) => [...prev, result]);
      return result;
    } catch (e) {
      setErro(String(e));
      return null;
    } finally {
      setImportando(false);
    }
  }

  function limparResultados() {
    setResultados([]);
    setErro(null);
  }

  return { resultados, importando, erro, handleImportarXML, handleImportarLote, handleImportarPDF, limparResultados };
}
