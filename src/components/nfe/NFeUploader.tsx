import { useState, useCallback, useRef } from "react";
import { Button } from "../ui/button";
import { useNFeImport } from "../../hooks/useNFeImport";
import type { NFeImportadaDTO } from "../../types";

interface NFeUploaderProps {
  onImportada: (dto: NFeImportadaDTO) => void;
  onVoltar: () => void;
}

export function NFeUploader({ onImportada, onVoltar }: NFeUploaderProps) {
  const [arquivo, setArquivo] = useState<File | null>(null);
  const [dragActive, setDragActive] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const { importando, erro, handleImportarXML, handleImportarPDF } = useNFeImport();

  const handleDrag = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    if (e.type === "dragenter" || e.type === "dragover") {
      setDragActive(true);
    } else if (e.type === "dragleave") {
      setDragActive(false);
    }
  }, []);

  const handleDrop = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setDragActive(false);
    if (e.dataTransfer.files && e.dataTransfer.files[0]) {
      const file = e.dataTransfer.files[0];
      if (file.name.toLowerCase().endsWith(".xml") || file.name.toLowerCase().endsWith(".pdf")) {
        setArquivo(file);
      }
    }
  }, []);

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    if (e.target.files && e.target.files[0]) {
      setArquivo(e.target.files[0]);
    }
  };

  const ehPdf = arquivo?.name.toLowerCase().endsWith(".pdf") ?? false;

  const handleImportar = async () => {
    if (!arquivo) return;
    const result = ehPdf
      ? await handleImportarPDF(arquivo)
      : await handleImportarXML(arquivo);
    if (result) {
      onImportada(result);
    }
  };

  const formatFileSize = (bytes: number): string => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  return (
    <div className="space-y-4">
          <div
            className={`relative border-2 border-dashed rounded-lg p-12 text-center transition-colors ${
              dragActive
                ? "border-primary bg-primary/10"
                : "border-input hover:border-primary/40"
            }`}
            onDragEnter={handleDrag}
            onDragLeave={handleDrag}
            onDragOver={handleDrag}
            onDrop={handleDrop}
            onClick={() => inputRef.current?.click()}
          >
            <input
              ref={inputRef}
              type="file"
              accept=".xml,.pdf"
              className="hidden"
              onChange={handleFileChange}
            />

            {arquivo ? (
              <div className="space-y-3">
                <div className="text-4xl">📄</div>
                <div>
                  <p className="text-sm font-medium">{arquivo.name}</p>
                  <p className="text-xs text-muted-foreground">{formatFileSize(arquivo.size)}</p>
                </div>
                <Button
                  size="sm"
                  variant="outline"
                  onClick={(e) => {
                    e.stopPropagation();
                    setArquivo(null);
                  }}
                >
                  Remover
                </Button>
              </div>
            ) : (
              <div className="space-y-3">
                <div className="text-4xl">📂</div>
                <p className="text-sm text-foreground">
                  Arraste um arquivo XML ou PDF aqui ou{" "}
                  <span className="text-primary underline">clique para selecionar</span>
                </p>
                <p className="text-xs text-muted-foreground">
                  XML importa tudo. PDF comercial (DANFE de mercadoria) ou de serviço (Nota Salvador) pré-preenche — você confere antes de gerar.
                </p>
              </div>
            )}
          </div>

          {erro && (
            <div className="mt-4 rounded-md border border-red-200 bg-red-50 p-3 text-sm text-red-700">
              {erro}
            </div>
          )}

          <div className="flex flex-col-reverse gap-2 sm:flex-row sm:justify-end">
            <Button variant="outline" type="button" onClick={onVoltar}>
              Cancelar
            </Button>
            <Button
              type="button"
              onClick={handleImportar}
              disabled={!arquivo || importando}
            >
              {importando ? "Lendo nota..." : "Importar"}
            </Button>
          </div>
    </div>
  );
}
